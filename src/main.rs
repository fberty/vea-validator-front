use alloy::primitives::{Address, FixedBytes};
use alloy::providers::ProviderBuilder;
use alloy::signers::local::PrivateKeySigner;
use alloy::network::{Ethereum, EthereumWallet};
use std::str::FromStr;
use std::sync::Arc;
use vea_validator::{
    event_listener::{EventListener, SnapshotEvent, ClaimEvent},
    epoch_watcher::EpochWatcher,
    claim_handler::{ClaimHandler, ClaimAction},
    contracts::{IVeaInboxArbToEth, IVeaOutboxArbToEth},
};

fn make_claim(state_root: FixedBytes<32>, claimer: Address) -> IVeaOutboxArbToEth::Claim {
    IVeaOutboxArbToEth::Claim {
        stateRoot: state_root,
        claimer,
        timestampClaimed: 0,
        timestampVerification: 0,
        blocknumberVerification: 0,
        honest: IVeaOutboxArbToEth::Party::None,
        challenger: Address::ZERO,
    }
}

async fn handle_claim_action<P: alloy::providers::Provider>(
    handler: &Arc<ClaimHandler<P>>,
    action: ClaimAction,
    route: &str,
) {
    match action {
        ClaimAction::None => {},
        ClaimAction::Claim { epoch, state_root } => {
            println!("[{}] Submitting claim for epoch {}", route, epoch);
            let _ = handler.submit_claim(epoch, state_root).await;
        }
        ClaimAction::Challenge { epoch, incorrect_claim } => {
            println!("[{}] Challenging claim for epoch {}", route, epoch);
            let _ = handler.challenge_claim(
                epoch,
                make_claim(incorrect_claim.state_root, incorrect_claim.claimer)
            ).await;
        }
    }
}

async fn run_validator_for_route(
    route_name: &str,
    inbox_address: Address,
    outbox_address: Address,
    destination_rpc: String,
    arbitrum_rpc: String,
    wallet: EthereumWallet,
    wallet_address: Address,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let destination_provider = ProviderBuilder::new()
        .connect_http(destination_rpc.parse()?);
    let destination_provider = Arc::new(destination_provider);

    let arbitrum_provider = ProviderBuilder::new()
        .connect_http(arbitrum_rpc.parse()?);
    let arbitrum_provider = Arc::new(arbitrum_provider);

    let destination_provider_with_wallet = ProviderBuilder::<_, _, Ethereum>::new()
        .wallet(wallet.clone())
        .connect_provider(destination_provider.clone());
    let destination_provider_with_wallet = Arc::new(destination_provider_with_wallet);

    let arbitrum_provider_with_wallet = ProviderBuilder::<_, _, Ethereum>::new()
        .wallet(wallet)
        .connect_provider(arbitrum_provider.clone());
    let arbitrum_provider_with_wallet = Arc::new(arbitrum_provider_with_wallet);

    let claim_handler = Arc::new(ClaimHandler::new(
        destination_provider_with_wallet.clone(),
        arbitrum_provider_with_wallet.clone(),
        outbox_address,
        inbox_address,
        wallet_address,
    ));

    let event_listener_inbox = EventListener::new(
        arbitrum_provider.clone(),
        inbox_address,
    );

    let event_listener_outbox = EventListener::new(
        destination_provider.clone(),
        outbox_address,
    );

    let epoch_watcher = EpochWatcher::new(
        arbitrum_provider.clone(),
    );

    let inbox_contract = IVeaInboxArbToEth::new(inbox_address, arbitrum_provider.clone());
    let epoch_period: u64 = inbox_contract.epochPeriod().call().await?.try_into()?;

    let _current_epoch: u64 = inbox_contract.epochFinalized().call().await?.try_into()?;

    println!("[{}] Starting validator for route", route_name);
    println!("[{}] Inbox: {:?}, Outbox: {:?}", route_name, inbox_address, outbox_address);

    let claim_handler_for_epoch = claim_handler.clone();
    let route_epoch = route_name.to_string();
    let epoch_handle = tokio::spawn(async move {
        epoch_watcher.watch_epochs(epoch_period, move |epoch| {
            let handler = claim_handler_for_epoch.clone();
            let route = route_epoch.clone();
            Box::pin(async move {
                if let Ok(action) = handler.handle_epoch_end(epoch).await {
                    handle_claim_action(&handler, action, &route).await;
                }
                Ok(())
            })
        }).await
    });

    let claim_handler_for_snapshots = claim_handler.clone();
    let route_snapshot = route_name.to_string();
    let snapshot_handle = tokio::spawn(async move {
        event_listener_inbox.watch_snapshots(move |event: SnapshotEvent| {
            let handler = claim_handler_for_snapshots.clone();
            let route = route_snapshot.clone();
            Box::pin(async move {
                println!("[{}] Snapshot saved for epoch {} with root {:?}", route, event.epoch, event.state_root);

                match handler.get_claim_for_epoch(event.epoch).await {
                    Ok(Some(existing_claim)) => {
                        println!("[{}] Claim already exists for epoch {}", route, event.epoch);
                        match handler.verify_claim(&existing_claim).await {
                            Ok(true) => println!("[{}] Existing claim is valid", route),
                            Ok(false) => {
                                println!("[{}] Existing claim is INVALID - need to challenge", route);
                                if let Err(e) = handler.challenge_claim(event.epoch, make_claim(existing_claim.state_root, existing_claim.claimer)).await {
                                    eprintln!("[{}] Failed to challenge claim: {}", route, e);
                                }
                            }
                            Err(e) => eprintln!("[{}] Error verifying claim: {}", route, e),
                        }
                    }
                    Ok(None) => {
                        println!("[{}] No claim for epoch {} - submitting claim", route, event.epoch);
                        if let Err(e) = handler.submit_claim(event.epoch, event.state_root).await {
                            eprintln!("[{}] Failed to submit claim: {}", route, e);
                        }
                    }
                    Err(e) => eprintln!("[{}] Error checking claim: {}", route, e),
                }
                Ok(())
            })
        }).await
    });

    let claim_handler_for_claims = claim_handler.clone();
    let route_claim = route_name.to_string();
    let claim_handle = tokio::spawn(async move {
        event_listener_outbox.watch_claims(move |event: ClaimEvent| {
            let handler = claim_handler_for_claims.clone();
            let route = route_claim.clone();
            Box::pin(async move {
                println!("[{}] Claim detected for epoch {} by {}", route, event.epoch, event.claimer);

                if let Ok(action) = handler.handle_claim_event(event.clone()).await {
                    handle_claim_action(&handler, action, &route).await;
                }
                Ok(())
            })
        }).await
    });

    tokio::select! {
        _ = epoch_handle => println!("[{}] Epoch watcher stopped", route_name),
        _ = snapshot_handle => println!("[{}] Snapshot watcher stopped", route_name),
        _ = claim_handle => println!("[{}] Claim watcher stopped", route_name),
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let arbitrum_rpc = std::env::var("ARBITRUM_RPC_URL")
        .expect("ARBITRUM_RPC_URL must be set");

    let private_key = std::env::var("PRIVATE_KEY")
        .or_else(|_| std::fs::read_to_string("/run/secrets/validator_key")
            .map(|s| s.trim().to_string()))
        .expect("PRIVATE_KEY not set or /run/secrets/validator_key not found");

    let signer = PrivateKeySigner::from_str(&private_key)?;
    let wallet_address = signer.address();
    let wallet = EthereumWallet::from(signer);

    println!("Validator wallet address: {}", wallet_address);

    let inbox_arb_to_eth = Address::from_str(
        &std::env::var("VEA_INBOX_ARB_TO_ETH")
            .expect("VEA_INBOX_ARB_TO_ETH must be set")
    )?;
    let outbox_arb_to_eth = Address::from_str(
        &std::env::var("VEA_OUTBOX_ARB_TO_ETH")
            .expect("VEA_OUTBOX_ARB_TO_ETH must be set")
    )?;
    let ethereum_rpc = std::env::var("ETHEREUM_RPC_URL")
        .or_else(|_| std::env::var("MAINNET_RPC_URL"))
        .expect("ETHEREUM_RPC_URL or MAINNET_RPC_URL must be set");

    let inbox_arb_to_gnosis = Address::from_str(
        &std::env::var("VEA_INBOX_ARB_TO_GNOSIS")
            .expect("VEA_INBOX_ARB_TO_GNOSIS must be set")
    )?;
    let outbox_arb_to_gnosis = Address::from_str(
        &std::env::var("VEA_OUTBOX_ARB_TO_GNOSIS")
            .expect("VEA_OUTBOX_ARB_TO_GNOSIS must be set")
    )?;
    let gnosis_rpc = std::env::var("GNOSIS_RPC_URL")
        .expect("GNOSIS_RPC_URL must be set");

    let arb_to_eth_handle = tokio::spawn(run_validator_for_route(
        "ARB_TO_ETH",
        inbox_arb_to_eth,
        outbox_arb_to_eth,
        ethereum_rpc,
        arbitrum_rpc.clone(),
        wallet.clone(),
        wallet_address,
    ));

    let arb_to_gnosis_handle = tokio::spawn(run_validator_for_route(
        "ARB_TO_GNOSIS",
        inbox_arb_to_gnosis,
        outbox_arb_to_gnosis,
        gnosis_rpc,
        arbitrum_rpc,
        wallet.clone(),
        wallet_address,
    ));

    println!("Running validators for both ARB_TO_ETH and ARB_TO_GNOSIS routes simultaneously...");

    tokio::select! {
        _ = arb_to_eth_handle => println!("ARB_TO_ETH validator stopped"),
        _ = arb_to_gnosis_handle => println!("ARB_TO_GNOSIS validator stopped"),
        _ = tokio::signal::ctrl_c() => println!("\nShutting down..."),
    }

    Ok(())
}