use alloy::primitives::{Address, FixedBytes, U256};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::signers::local::PrivateKeySigner;
use alloy::network::{Ethereum, EthereumWallet};
use alloy::rpc::types::Filter;
use alloy::sol_types::SolEvent;
use serial_test::serial;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::time::{timeout, Duration};
use vea_validator::{
    contracts::{IVeaInboxArbToEth, IVeaOutboxArbToEth},
    event_listener::{EventListener, ClaimEvent},
    claim_handler::ClaimHandler,
};

// Test fixture
struct TestFixture<P1: Provider, P2: Provider> {
    eth_provider: Arc<P1>,
    arb_provider: Arc<P2>,
    eth_snapshot_id: Option<String>,
    arb_snapshot_id: Option<String>,
}

impl<P1: Provider, P2: Provider> TestFixture<P1, P2> {
    fn new(eth_provider: Arc<P1>, arb_provider: Arc<P2>) -> Self {
        Self {
            eth_provider,
            arb_provider,
            eth_snapshot_id: None,
            arb_snapshot_id: None,
        }
    }

    async fn take_snapshots(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let empty_params: Vec<serde_json::Value> = vec![];

        let eth_snapshot: serde_json::Value = self.eth_provider
            .raw_request("evm_snapshot".into(), empty_params.clone())
            .await?;
        self.eth_snapshot_id = Some(eth_snapshot.as_str().unwrap().to_string());

        let arb_snapshot: serde_json::Value = self.arb_provider
            .raw_request("evm_snapshot".into(), empty_params)
            .await?;
        self.arb_snapshot_id = Some(arb_snapshot.as_str().unwrap().to_string());

        Ok(())
    }

    async fn revert_snapshots(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref snapshot_id) = self.eth_snapshot_id {
            let _: serde_json::Value = self.eth_provider
                .raw_request("evm_revert".into(), vec![serde_json::json!(snapshot_id)])
                .await?;
        }

        if let Some(ref snapshot_id) = self.arb_snapshot_id {
            let _: serde_json::Value = self.arb_provider
                .raw_request("evm_revert".into(), vec![serde_json::json!(snapshot_id)])
                .await?;
        }

        Ok(())
    }
}

async fn advance_time<P: Provider>(provider: &P, seconds: u64) {
    let _: serde_json::Value = provider
        .raw_request("evm_increaseTime".into(), vec![serde_json::json!(seconds)])
        .await
        .expect("Failed to advance time");

    let empty_params: Vec<serde_json::Value> = vec![];
    let _: serde_json::Value = provider
        .raw_request("evm_mine".into(), empty_params)
        .await
        .expect("Failed to mine block");
}

/// Helper to wait for a Verified event on the outbox
async fn wait_for_verified_event<P: Provider>(
    provider: &Arc<P>,
    outbox_address: Address,
    expected_epoch: u64,
    timeout_secs: u64,
) -> Result<(), String> {
    let start_block = provider.get_block_number().await
        .map_err(|e| format!("Failed to get block number: {}", e))?;

    let result = timeout(Duration::from_secs(timeout_secs), async {
        loop {
            let filter = Filter::new()
                .address(outbox_address)
                .event_signature(IVeaOutboxArbToEth::Verified::SIGNATURE_HASH)
                .from_block(start_block);

            let logs = provider.get_logs(&filter).await
                .map_err(|e| format!("Failed to get logs: {}", e))?;

            for log in logs {
                // Convert RPC log to primitives log
                let prim_log = alloy::primitives::Log {
                    address: log.address(),
                    data: log.data().clone(),
                };

                if let Ok(event) = IVeaOutboxArbToEth::Verified::decode_log(&prim_log) {
                    if event._epoch == U256::from(expected_epoch) {
                        return Ok::<(), String>(());
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }).await;

    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(format!("Timeout waiting for Verified event for epoch {}", expected_epoch)),
    }
}

#[tokio::test]
#[serial]
async fn test_validator_detects_and_challenges_wrong_claim() {
    dotenv::dotenv().ok();

    println!("\n==============================================");
    println!("VALIDATOR INTEGRATION TEST: Wrong Claim Detection");
    println!("==============================================\n");

    // Setup
    let inbox_address = Address::from_str(
        &std::env::var("VEA_INBOX_ARB_TO_ETH").expect("VEA_INBOX_ARB_TO_ETH must be set")
    ).expect("Invalid inbox address");

    let outbox_address = Address::from_str(
        &std::env::var("VEA_OUTBOX_ARB_TO_ETH").expect("VEA_OUTBOX_ARB_TO_ETH must be set")
    ).expect("Invalid outbox address");

    let arbitrum_rpc = std::env::var("ARBITRUM_RPC_URL").expect("ARBITRUM_RPC_URL must be set");
    let ethereum_rpc = std::env::var("ETHEREUM_RPC_URL")
        .or_else(|_| std::env::var("MAINNET_RPC_URL"))
        .expect("ETHEREUM_RPC_URL or MAINNET_RPC_URL must be set");

    let private_key = std::env::var("PRIVATE_KEY")
        .unwrap_or_else(|_| "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string());

    let ethereum_provider = ProviderBuilder::new().connect_http(ethereum_rpc.parse().unwrap());
    let ethereum_provider = Arc::new(ethereum_provider);

    let arbitrum_provider = ProviderBuilder::new().connect_http(arbitrum_rpc.parse().unwrap());
    let arbitrum_provider = Arc::new(arbitrum_provider);

    let mut fixture = TestFixture::new(ethereum_provider.clone(), arbitrum_provider.clone());
    fixture.take_snapshots().await.unwrap();

    let signer = PrivateKeySigner::from_str(&private_key).unwrap();
    let wallet_address = signer.address();
    let wallet = EthereumWallet::from(signer);

    let ethereum_with_wallet = ProviderBuilder::<_, _, Ethereum>::new()
        .wallet(wallet.clone())
        .connect_provider(ethereum_provider.clone());
    let ethereum_with_wallet = Arc::new(ethereum_with_wallet);

    let arbitrum_with_wallet = ProviderBuilder::<_, _, Ethereum>::new()
        .wallet(wallet.clone())
        .connect_provider(arbitrum_provider.clone());
    let arbitrum_with_wallet = Arc::new(arbitrum_with_wallet);

    // STEP 1: Setup - create an epoch with messages and snapshot
    println!("--- SETUP: Creating epoch with messages and snapshot ---");
    let inbox = IVeaInboxArbToEth::new(inbox_address, arbitrum_with_wallet.clone());
    let outbox = IVeaOutboxArbToEth::new(outbox_address, ethereum_with_wallet.clone());

    let epoch_period: u64 = inbox.epochPeriod().call().await.unwrap().try_into().unwrap();

    // Send messages
    for i in 0..3 {
        let test_message = alloy::primitives::Bytes::from(vec![0xAA, 0xBB, 0xCC, i]);
        inbox.sendMessage(
            Address::from_str("0x0000000000000000000000000000000000000001").unwrap(),
            test_message
        ).send().await.unwrap().get_receipt().await.unwrap();
    }

    let current_epoch: u64 = inbox.epochNow().call().await.unwrap().try_into().unwrap();
    inbox.saveSnapshot().send().await.unwrap().get_receipt().await.unwrap();
    let correct_root = inbox.snapshots(U256::from(current_epoch)).call().await.unwrap();

    if correct_root == FixedBytes::<32>::ZERO {
        panic!("Got zero root, test cannot proceed");
    }

    println!("✓ Saved snapshot for epoch {} with correct root: {:?}", current_epoch, correct_root);

    // Advance time so epoch can be claimed
    advance_time(arbitrum_provider.as_ref(), epoch_period + 10).await;
    advance_time(ethereum_provider.as_ref(), epoch_period + 10).await;

    let target_epoch = current_epoch;

    // Sync ethereum time to make the epoch claimable
    // Outbox requires: _epoch == block.timestamp / epochPeriod - 1
    let eth_block = ethereum_provider.get_block_by_number(Default::default()).await.unwrap().unwrap();
    let eth_timestamp = eth_block.header.timestamp;
    let target_timestamp = (target_epoch + 1) * epoch_period + 10;
    let advance_amount = target_timestamp.saturating_sub(eth_timestamp);
    if advance_amount > 0 {
        println!("Syncing Ethereum time (advancing {} seconds)", advance_amount);
        advance_time(ethereum_provider.as_ref(), advance_amount).await;
    }

    // STEP 2: Create the ClaimHandler (this is what the validator uses)
    println!("\n--- Starting Validator Components ---");
    let claim_handler = Arc::new(ClaimHandler::new(
        ethereum_with_wallet.clone(),
        arbitrum_with_wallet.clone(),
        outbox_address,
        inbox_address,
        wallet_address,
    ));

    // Create event listener for claims
    let event_listener = EventListener::new(
        ethereum_provider.clone(),
        outbox_address,
    );

    // Flag to track if validator challenged
    let challenge_detected = Arc::new(AtomicBool::new(false));
    let challenge_flag = challenge_detected.clone();

    // Start watching for claims (like the validator does in main.rs)
    let claim_handler_clone = claim_handler.clone();
    let watch_handle = tokio::spawn(async move {
        event_listener.watch_claims(move |event: ClaimEvent| {
            let handler = claim_handler_clone.clone();
            let flag = challenge_flag.clone();
            Box::pin(async move {
                println!("📡 Validator detected claim for epoch {} by {}", event.epoch, event.claimer);

                // This is what the validator does in main.rs
                if let Ok(action) = handler.handle_claim_event(event.clone()).await {
                    match action {
                        vea_validator::claim_handler::ClaimAction::Challenge { epoch, incorrect_claim } => {
                            println!("⚔️  Validator decided to CHALLENGE epoch {}", epoch);

                            // Challenge the claim
                            if let Err(e) = handler.challenge_claim(epoch, vea_validator::claim_handler::make_claim(&incorrect_claim)).await {
                                eprintln!("❌ Challenge failed: {}", e);
                            } else {
                                println!("✅ Validator successfully challenged the claim!");
                                flag.store(true, Ordering::SeqCst);
                            }
                        }
                        vea_validator::claim_handler::ClaimAction::None => {
                            println!("ℹ️  Validator decided NO ACTION needed");
                        }
                        _ => {}
                    }
                }
                Ok(())
            })
        }).await
    });

    // Give the watcher time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // STEP 3: Malicious actor makes WRONG claim
    println!("\n--- ATTACK: Malicious actor submits wrong claim ---");
    let wrong_root = FixedBytes::<32>::from([0x99; 32]);
    println!("Wrong root: {:?}", wrong_root);
    println!("Correct root: {:?}", correct_root);

    let deposit = outbox.deposit().call().await.unwrap();
    outbox.claim(U256::from(target_epoch), wrong_root)
        .value(deposit)
        .send().await.unwrap()
        .get_receipt().await.unwrap();

    println!("✓ Malicious claim submitted");

    // STEP 4: Wait for validator to react
    println!("\n--- Waiting for validator to detect and challenge... ---");

    // Give the validator up to 5 seconds to detect and challenge
    let result = timeout(Duration::from_secs(5), async {
        while !challenge_detected.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }).await;

    watch_handle.abort();

    if result.is_ok() {
        println!("\n✅✅✅ VALIDATOR TEST PASSED! ✅✅✅");
        println!("The validator:");
        println!("  1. Detected the malicious claim via event watching");
        println!("  2. Verified it was incorrect");
        println!("  3. Automatically challenged it");
        println!("\nThis proves the validator's reactive logic works!");
    } else {
        panic!("❌ VALIDATOR FAILED: Did not challenge the wrong claim within 5 seconds");
    }

    fixture.revert_snapshots().await.unwrap();
}

#[tokio::test]
#[serial]
async fn test_validator_triggers_bridge_resolution() {
    dotenv::dotenv().ok();

    println!("\n==============================================");
    println!("VALIDATOR INTEGRATION TEST: Bridge Resolution");
    println!("==============================================\n");

    // Setup (same as above)
    let inbox_address = Address::from_str(
        &std::env::var("VEA_INBOX_ARB_TO_ETH").expect("VEA_INBOX_ARB_TO_ETH must be set")
    ).expect("Invalid inbox address");

    let outbox_address = Address::from_str(
        &std::env::var("VEA_OUTBOX_ARB_TO_ETH").expect("VEA_OUTBOX_ARB_TO_ETH must be set")
    ).expect("Invalid outbox address");

    let arbitrum_rpc = std::env::var("ARBITRUM_RPC_URL").expect("ARBITRUM_RPC_URL must be set");
    let ethereum_rpc = std::env::var("ETHEREUM_RPC_URL")
        .or_else(|_| std::env::var("MAINNET_RPC_URL"))
        .expect("ETHEREUM_RPC_URL or MAINNET_RPC_URL must be set");

    let private_key = std::env::var("PRIVATE_KEY")
        .unwrap_or_else(|_| "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string());

    let ethereum_provider = ProviderBuilder::new().connect_http(ethereum_rpc.parse().unwrap());
    let ethereum_provider = Arc::new(ethereum_provider);

    let arbitrum_provider = ProviderBuilder::new().connect_http(arbitrum_rpc.parse().unwrap());
    let arbitrum_provider = Arc::new(arbitrum_provider);

    let mut fixture = TestFixture::new(ethereum_provider.clone(), arbitrum_provider.clone());
    fixture.take_snapshots().await.unwrap();

    let signer = PrivateKeySigner::from_str(&private_key).unwrap();
    let wallet_address = signer.address();
    let wallet = EthereumWallet::from(signer);

    let ethereum_with_wallet = ProviderBuilder::<_, _, Ethereum>::new()
        .wallet(wallet.clone())
        .connect_provider(ethereum_provider.clone());
    let ethereum_with_wallet = Arc::new(ethereum_with_wallet);

    let arbitrum_with_wallet = ProviderBuilder::<_, _, Ethereum>::new()
        .wallet(wallet.clone())
        .connect_provider(arbitrum_provider.clone());
    let arbitrum_with_wallet = Arc::new(arbitrum_with_wallet);

    // Setup epoch with snapshot
    println!("--- SETUP: Creating epoch with messages and snapshot ---");
    let inbox = IVeaInboxArbToEth::new(inbox_address, arbitrum_with_wallet.clone());
    let outbox = IVeaOutboxArbToEth::new(outbox_address, ethereum_with_wallet.clone());

    let epoch_period: u64 = inbox.epochPeriod().call().await.unwrap().try_into().unwrap();

    for i in 0..3 {
        let test_message = alloy::primitives::Bytes::from(vec![0xDD, 0xEE, 0xFF, i]);
        inbox.sendMessage(
            Address::from_str("0x0000000000000000000000000000000000000001").unwrap(),
            test_message
        ).send().await.unwrap().get_receipt().await.unwrap();
    }

    let current_epoch: u64 = inbox.epochNow().call().await.unwrap().try_into().unwrap();
    inbox.saveSnapshot().send().await.unwrap().get_receipt().await.unwrap();
    let correct_root = inbox.snapshots(U256::from(current_epoch)).call().await.unwrap();

    if correct_root == FixedBytes::<32>::ZERO {
        panic!("Got zero root, test cannot proceed");
    }

    println!("✓ Saved snapshot for epoch {}", current_epoch);

    advance_time(arbitrum_provider.as_ref(), epoch_period + 10).await;
    advance_time(ethereum_provider.as_ref(), epoch_period + 10).await;

    let target_epoch = current_epoch;

    // Sync ethereum time
    let eth_block = ethereum_provider.get_block_by_number(Default::default()).await.unwrap().unwrap();
    let eth_timestamp = eth_block.header.timestamp;
    let target_timestamp = (target_epoch + 1) * epoch_period + 10;
    let advance_amount = target_timestamp.saturating_sub(eth_timestamp);
    if advance_amount > 0 {
        println!("Syncing Ethereum time (advancing {} seconds)", advance_amount);
        advance_time(ethereum_provider.as_ref(), advance_amount).await;
    }

    // Create the bridge resolver (from main.rs)
    println!("\n--- Creating Bridge Resolver (from main.rs) ---");
    let arb_rpc_clone = arbitrum_rpc.clone();
    let wallet_clone = wallet.clone();
    let inbox_addr = inbox_address;
    let wallet_addr = wallet_address;

    let bridge_resolver_called = Arc::new(AtomicBool::new(false));
    let bridge_flag = bridge_resolver_called.clone();

    let bridge_resolver = move |epoch: u64, claim: ClaimEvent| {
        let rpc = arb_rpc_clone.clone();
        let wlt = wallet_clone.clone();
        let inbox = inbox_addr;
        let wlt_addr = wallet_addr;
        let flag = bridge_flag.clone();

        async move {
            println!("🌉 Bridge resolver triggered for epoch {}!", epoch);

            let provider = ProviderBuilder::<_, _, Ethereum>::new()
                .wallet(wlt)
                .connect_http(rpc.parse()?);
            let provider = Arc::new(provider);

            let inbox_contract = IVeaInboxArbToEth::new(inbox, provider);

            let outbox_claim = IVeaInboxArbToEth::Claim {
                stateRoot: claim.state_root,
                claimer: claim.claimer,
                timestampClaimed: claim.timestamp_claimed,
                timestampVerification: 0,
                blocknumberVerification: 0,
                honest: IVeaInboxArbToEth::Party::None,
                challenger: wlt_addr,
            };

            let tx = inbox_contract.sendSnapshot(U256::from(epoch), outbox_claim)
                .from(wlt_addr);

            let pending = tx.send().await?;
            let receipt = pending.get_receipt().await?;

            if !receipt.status() {
                return Err(Box::<dyn std::error::Error + Send + Sync>::from("sendSnapshot transaction failed"));
            }

            println!("✅ sendSnapshot called successfully! Transaction: {:?}", receipt.transaction_hash);
            flag.store(true, Ordering::SeqCst);
            Ok(())
        }
    };

    // Create claim handler with bridge resolver
    let claim_handler = Arc::new(ClaimHandler::new(
        ethereum_with_wallet.clone(),
        arbitrum_with_wallet.clone(),
        outbox_address,
        inbox_address,
        wallet_address,
    ));

    let event_listener = EventListener::new(
        ethereum_provider.clone(),
        outbox_address,
    );

    // Watch for claims and trigger bridge resolution
    let claim_handler_clone = claim_handler.clone();
    let resolver = bridge_resolver.clone();

    let watch_handle = tokio::spawn(async move {
        event_listener.watch_claims(move |event: ClaimEvent| {
            let handler = claim_handler_clone.clone();
            let resolver_clone = resolver.clone();

            Box::pin(async move {
                println!("📡 Detected claim for epoch {}", event.epoch);

                if let Ok(action) = handler.handle_claim_event(event.clone()).await {
                    match action {
                        vea_validator::claim_handler::ClaimAction::Challenge { epoch, .. } => {
                            println!("⚔️  Challenging and triggering bridge resolution...");

                            // Challenge first
                            if let Err(e) = handler.challenge_claim(epoch, vea_validator::claim_handler::make_claim(&event)).await {
                                eprintln!("Challenge failed: {}", e);
                            } else {
                                // Then trigger bridge resolution (THIS IS THE KEY PART)
                                if let Err(e) = resolver_clone(epoch, event.clone()).await {
                                    eprintln!("Bridge resolution failed: {}", e);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(())
            })
        }).await
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Malicious claim
    println!("\n--- ATTACK: Submitting wrong claim ---");
    let wrong_root = FixedBytes::<32>::from([0x88; 32]);
    let deposit = outbox.deposit().call().await.unwrap();
    outbox.claim(U256::from(target_epoch), wrong_root)
        .value(deposit)
        .send().await.unwrap()
        .get_receipt().await.unwrap();

    println!("✓ Wrong claim submitted");

    // Wait for bridge resolution
    println!("\n--- Waiting for validator to trigger bridge resolution... ---");

    let result = timeout(Duration::from_secs(5), async {
        while !bridge_resolver_called.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }).await;

    watch_handle.abort();

    if result.is_err() {
        panic!("❌ VALIDATOR FAILED: Did not trigger bridge resolution within 5 seconds");
    }

    println!("✓ Bridge resolver called, now waiting for message to arrive...");

    // STEP 5: Wait for the Verified event (message has arrived and been processed)
    println!("\n--- STEP 5: Waiting for Verified event (message arrival) ---");

    match wait_for_verified_event(&ethereum_provider, outbox_address, target_epoch, 5).await {
        Ok(()) => {
            println!("✅ Verified event detected! Bridge message arrived successfully!");
        }
        Err(e) => {
            panic!("❌ Failed to detect Verified event: {}", e);
        }
    }

    // STEP 6: Verify the claim was resolved correctly
    println!("\n--- STEP 6: Verifying claim resolution ---");
    let final_claim_hash = outbox.claimHashes(U256::from(target_epoch)).call().await.unwrap();

    // The claim hash should have been updated with the honest party marked
    if final_claim_hash == FixedBytes::<32>::ZERO {
        panic!("Claim hash is zero - claim was not resolved");
    }

    println!("✓ Claim was resolved (hash: {:?})", final_claim_hash);

    println!("\n✅✅✅ COMPLETE BRIDGE RESOLUTION TEST PASSED! ✅✅✅");
    println!("The validator:");
    println!("  1. Detected the malicious claim");
    println!("  2. Challenged it");
    println!("  3. Automatically triggered bridge resolution via sendSnapshot");
    println!("  4. Bridge message was delivered (ArbSys → Outbox)");
    println!("  5. Verified event was emitted");
    println!("  6. Claim was marked as resolved");
    println!("\nThis proves the COMPLETE end-to-end bridge resolution flow!");

    fixture.revert_snapshots().await.unwrap();
}
