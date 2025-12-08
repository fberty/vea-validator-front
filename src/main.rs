use alloy::primitives::Address;
use alloy::signers::local::PrivateKeySigner;
use alloy::network::EthereumWallet;
use std::str::FromStr;
use std::sync::Arc;
use vea_validator::{
    event_listener::{EventListener, ClaimEvent},
    epoch_watcher::EpochWatcher,
    claim_handler::{ClaimHandler, make_claim},
    contracts::IVeaInboxArbToEth,
    config::ValidatorConfig,
    l2_to_l1_finder::L2ToL1Finder,
    arb_relay_handler::ArbRelayHandler,
    amb_finder::AmbFinder,
    amb_relay_handler::AmbRelayHandler,
    startup::{check_rpc_health, check_balances},
};

async fn handle_invalid_claim(
    handler: &Arc<ClaimHandler>,
    incorrect_claim: ClaimEvent,
    route: &str,
) {
    let epoch = incorrect_claim.epoch;
    println!("[{}] Challenging incorrect claim for epoch {}", route, epoch);
    match handler.challenge_claim(epoch, make_claim(&incorrect_claim)).await {
        Ok(()) => {
            println!("[{}] Challenge successful, triggering bridge resolution for epoch {}", route, epoch);
            handler.trigger_bridge_resolution(epoch, &incorrect_claim).await
                .unwrap_or_else(|e| panic!("[{}] FATAL: Failed to trigger bridge resolution for epoch {}: {}", route, epoch, e));
        }
        Err(_) => {
            println!("[{}] Claim already challenged by another validator - bridge is safe", route);
        }
    }
}

async fn run_validator_for_route(
    route: vea_validator::config::Route,
    wallet_address: Address,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let claim_handler = Arc::new(ClaimHandler::new(
        route.clone(),
        wallet_address,
    ));
    let event_listener_outbox = EventListener::new(
        route.outbox_provider.clone(),
        route.outbox_address,
    );
    let inbox_contract = IVeaInboxArbToEth::new(route.inbox_address, route.inbox_provider.clone());
    let epoch_period: u64 = inbox_contract.epochPeriod().call().await?.try_into()?;
    println!("[{}] Starting validator for route", route.name);
    println!("[{}] Inbox: {:?}, Outbox: {:?}", route.name, route.inbox_address, route.outbox_address);
    let epoch_watcher = EpochWatcher::new(route.inbox_provider.clone(), claim_handler.clone(), route.name);
    let epoch_handle = tokio::spawn(async move {
        epoch_watcher.watch_epochs(epoch_period).await
    });
    let claim_handler_for_claims = claim_handler.clone();
    let route_name = route.name;
    let claim_handle = tokio::spawn(async move {
        event_listener_outbox.watch_claims(move |event: ClaimEvent| {
            let handler = claim_handler_for_claims.clone();
            Box::pin(async move {
                println!("[{}] Claim detected for epoch {} by {}", route_name, event.epoch, event.claimer);
                let invalid_claim = handler.handle_claim_event(event.clone()).await
                    .unwrap_or_else(|e| panic!("[{}] FATAL: Failed to handle claim event for epoch {}: {}", route_name, event.epoch, e));
                if let Some(claim) = invalid_claim {
                    handle_invalid_claim(&handler, claim, route_name).await;
                }
                Ok(())
            })
        }).await
    });
    tokio::select! {
        _ = epoch_handle => println!("[{}] Epoch watcher stopped", route.name),
        _ = claim_handle => println!("[{}] Claim watcher stopped", route.name),
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let c = ValidatorConfig::from_env()?;
    let signer = PrivateKeySigner::from_str(&c.private_key)?;
    let wallet = EthereumWallet::from(signer);
    println!("Validator wallet address: {}", wallet.default_signer().address());
    let routes = c.build_routes();
    check_rpc_health(&routes).await?;
    check_balances(&c, &routes).await?;

    let arb_to_eth_route = &routes[0];
    let arb_to_gnosis_route = &routes[1];
    let wallet_address = wallet.default_signer().address();

    let l2_to_l1_finder = L2ToL1Finder::new(arb_to_eth_route.inbox_provider.clone())
        .add_target(arb_to_eth_route.outbox_address, "schedules/arb_to_eth_relay.json")
        .add_target(c.router_arb_to_gnosis, "schedules/arb_to_gnosis_relay.json");

    let amb_finder = AmbFinder::new(
        arb_to_eth_route.outbox_provider.clone(),
        c.router_arb_to_gnosis,
        "schedules/amb_to_gnosis.json",
    );

    let arb_to_eth_relay_handler = ArbRelayHandler::new(
        arb_to_eth_route.outbox_provider.clone(),
        arb_to_eth_route.inbox_provider.clone(),
        c.arb_outbox,
        "schedules/arb_to_eth_relay.json",
    );

    let arb_to_gnosis_relay_handler = ArbRelayHandler::new(
        arb_to_eth_route.outbox_provider.clone(),
        arb_to_gnosis_route.inbox_provider.clone(),
        c.arb_outbox,
        "schedules/arb_to_gnosis_relay.json",
    );

    let amb_relay_handler = AmbRelayHandler::new(
        arb_to_gnosis_route.outbox_provider.clone(),
        c.gnosis_amb,
        "schedules/amb_to_gnosis.json",
    );

    let l2_to_l1_finder_handle = tokio::spawn(async move {
        l2_to_l1_finder.run().await;
    });

    let amb_finder_handle = tokio::spawn(async move {
        amb_finder.run().await;
    });

    let arb_to_eth_relay_handle = tokio::spawn(async move {
        arb_to_eth_relay_handler.run().await;
    });

    let arb_to_gnosis_relay_handle = tokio::spawn(async move {
        arb_to_gnosis_relay_handler.run().await;
    });

    let amb_relay_handle = tokio::spawn(async move {
        amb_relay_handler.run().await;
    });

    let arb_to_eth_handle = tokio::spawn(run_validator_for_route(
        routes[0].clone(),
        wallet_address,
    ));
    let arb_to_gnosis_handle = tokio::spawn(run_validator_for_route(
        routes[1].clone(),
        wallet_address,
    ));

    println!("Running validators for both ARB_TO_ETH and ARB_TO_GNOSIS routes simultaneously...");
    println!("L2ToL1 finder, AMB finder, and relay handlers started.");

    let monitor_handle = tokio::spawn(async move {
        let (eth_result, gnosis_result) = tokio::join!(arb_to_eth_handle, arb_to_gnosis_handle);

        match eth_result {
            Ok(Ok(())) => println!("ARB_TO_ETH validator stopped gracefully"),
            Ok(Err(e)) => eprintln!("ARB_TO_ETH validator failed: {}", e),
            Err(e) => eprintln!("ARB_TO_ETH validator panicked: {}", e),
        }

        match gnosis_result {
            Ok(Ok(())) => println!("ARB_TO_GNOSIS validator stopped gracefully"),
            Ok(Err(e)) => eprintln!("ARB_TO_GNOSIS validator failed: {}", e),
            Err(e) => eprintln!("ARB_TO_GNOSIS validator panicked: {}", e),
        }
    });

    tokio::select! {
        _ = monitor_handle => println!("Both routes stopped"),
        _ = l2_to_l1_finder_handle => println!("L2ToL1 finder stopped"),
        _ = amb_finder_handle => println!("AMB finder stopped"),
        _ = arb_to_eth_relay_handle => println!("ARB_TO_ETH relay handler stopped"),
        _ = arb_to_gnosis_relay_handle => println!("ARB_TO_GNOSIS relay handler stopped"),
        _ = amb_relay_handle => println!("AMB relay handler stopped"),
        _ = tokio::signal::ctrl_c() => println!("\nShutting down..."),
    }
    Ok(())
}
