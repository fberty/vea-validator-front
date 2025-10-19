mod common;

use alloy::primitives::{Address, FixedBytes, U256};
use serial_test::serial;
use std::str::FromStr;
use vea_validator::{
    contracts::{IVeaInboxArbToEth, IVeaOutboxArbToEth, IWETH},
    claim_handler::ClaimHandler,
    config::ValidatorConfig,
    startup::ensure_weth_approval,
};
use common::{TestFixture, advance_time, Provider};

#[tokio::test]
#[serial]
async fn test_challenge_uses_correct_root_from_inbox() {
    println!("\n==============================================");
    println!("CHALLENGE TEST: Challenge Uses Correct Root");
    println!("==============================================\n");

    let c = ValidatorConfig::from_env().expect("Failed to load config");
    let providers = c.setup_arb_to_eth().expect("Failed to setup providers");

    let mut fixture = TestFixture::new(providers.destination_provider.clone(), providers.arbitrum_provider.clone());
    fixture.take_snapshots().await.unwrap();

    let inbox = IVeaInboxArbToEth::new(c.inbox_arb_to_eth, providers.arbitrum_with_wallet.clone());
    let outbox = IVeaOutboxArbToEth::new(c.outbox_arb_to_eth, providers.destination_with_wallet.clone());
    let epoch_period: u64 = inbox.epochPeriod().call().await.unwrap().try_into().unwrap();

    // Send messages and save snapshot
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

    println!("Correct root from inbox: {:?}", correct_root);
    assert_ne!(correct_root, FixedBytes::<32>::ZERO, "Snapshot should be saved");

    // Advance time so epoch can be claimed
    advance_time(providers.arbitrum_provider.as_ref(), epoch_period + 70).await;
    advance_time(providers.destination_provider.as_ref(), epoch_period + 70).await;

    let target_epoch = current_epoch;

    // Sync destination chain time to make the epoch claimable
    let dest_block = providers.destination_provider.get_block_by_number(Default::default()).await.unwrap().unwrap();
    let dest_timestamp = dest_block.header.timestamp;
    let target_timestamp = (target_epoch + 1) * epoch_period + 70;
    let advance_amount = target_timestamp.saturating_sub(dest_timestamp);
    if advance_amount > 0 {
        advance_time(providers.destination_provider.as_ref(), advance_amount).await;
    }

    // Malicious claim with WRONG root
    let wrong_root = FixedBytes::<32>::from([0xDE, 0xAD, 0xBE, 0xEF, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    println!("Wrong root (malicious claim): {:?}", wrong_root);

    let deposit = outbox.deposit().call().await.unwrap();
    outbox.claim(U256::from(current_epoch), wrong_root)
        .value(deposit)
        .send().await.unwrap()
        .get_receipt().await.unwrap();

    println!("✓ Malicious claim submitted");

    // Create claim handler
    let claim_handler = ClaimHandler::new(
        providers.destination_with_wallet.clone(),
        providers.arbitrum_with_wallet.clone(),
        c.outbox_arb_to_eth,
        c.inbox_arb_to_eth,
        providers.wallet_address,
        None,
    );

    // Verify the claim
    let state_root_from_handler = claim_handler.get_correct_state_root(current_epoch).await.unwrap();
    println!("Root fetched by claim_handler: {:?}", state_root_from_handler);

    assert_eq!(state_root_from_handler, correct_root, "ClaimHandler should fetch the CORRECT root from inbox");
    assert_ne!(state_root_from_handler, wrong_root, "ClaimHandler should NOT use the malicious wrong root");

    println!("\n✅ TEST PASSED!");
    println!("The ClaimHandler correctly:");
    println!("  1. Fetched the correct root from the inbox contract");
    println!("  2. Did NOT use the malicious claim's wrong root");
    println!("  3. Can now challenge with the correct proof");

    fixture.revert_snapshots().await.unwrap();
}

#[tokio::test]
#[serial]
async fn test_weth_approval_set_on_startup_if_missing() {
    println!("\n==============================================");
    println!("STARTUP TEST: WETH Max Approval on Startup");
    println!("==============================================\n");

    let c = ValidatorConfig::from_env().expect("Failed to load config");
    let providers = c.setup_arb_to_gnosis().expect("Failed to setup providers");

    let mut fixture = TestFixture::new(providers.destination_provider.clone(), providers.arbitrum_provider.clone());
    fixture.take_snapshots().await.unwrap();

    let weth = IWETH::new(c.weth_gnosis, providers.destination_with_wallet.clone());

    // Check initial allowance (should be zero in fresh test)
    let initial_allowance = weth.allowance(providers.wallet_address, c.outbox_arb_to_gnosis).call().await.unwrap();
    println!("Initial WETH allowance: {}", initial_allowance);

    // If there's already approval, revoke it to test the startup flow
    if initial_allowance > U256::ZERO {
        println!("Revoking existing approval to test startup logic...");
        let revoke_tx = weth.approve(c.outbox_arb_to_gnosis, U256::ZERO).from(providers.wallet_address);
        revoke_tx.send().await.unwrap().get_receipt().await.unwrap();
        println!("✓ Approval revoked");
    }

    // Verify approval is now zero
    let allowance_before = weth.allowance(providers.wallet_address, c.outbox_arb_to_gnosis).call().await.unwrap();
    assert_eq!(allowance_before, U256::ZERO, "Allowance should be zero before startup");

    // Call ensure_weth_approval (this is what check_balances calls on startup)
    println!("\nCalling ensure_weth_approval (simulating startup)...");
    ensure_weth_approval(&c, providers.wallet_address, &providers).await.unwrap();

    // Verify max approval was set
    let allowance_after = weth.allowance(providers.wallet_address, c.outbox_arb_to_gnosis).call().await.unwrap();
    println!("Allowance after startup: {}", allowance_after);

    assert_eq!(allowance_after, U256::MAX, "Allowance should be MAX after startup");

    println!("\n✅ STARTUP TEST PASSED!");
    println!("The startup logic correctly:");
    println!("  1. Detected missing WETH approval");
    println!("  2. Set MAX approval for Gnosis outbox");
    println!("  3. Validator can now challenge without per-tx approvals");

    fixture.revert_snapshots().await.unwrap();
}

#[tokio::test]
#[serial]
async fn test_weth_approval_skipped_if_already_exists() {
    println!("\n==============================================");
    println!("STARTUP TEST: Skip WETH Approval if Exists");
    println!("==============================================\n");

    let c = ValidatorConfig::from_env().expect("Failed to load config");
    let providers = c.setup_arb_to_gnosis().expect("Failed to setup providers");

    let mut fixture = TestFixture::new(providers.destination_provider.clone(), providers.arbitrum_provider.clone());
    fixture.take_snapshots().await.unwrap();

    let weth = IWETH::new(c.weth_gnosis, providers.destination_with_wallet.clone());

    // Manually set some approval
    println!("Setting manual WETH approval...");
    let manual_approval = U256::from(1000000000u64);
    let approve_tx = weth.approve(c.outbox_arb_to_gnosis, manual_approval).from(providers.wallet_address);
    approve_tx.send().await.unwrap().get_receipt().await.unwrap();
    println!("✓ Manual approval set: {}", manual_approval);

    // Call ensure_weth_approval (should detect existing approval and skip)
    println!("\nCalling ensure_weth_approval with existing approval...");
    ensure_weth_approval(&c, providers.wallet_address, &providers).await.unwrap();

    // Verify approval DIDN'T change (should still be manual value, not MAX)
    let final_allowance = weth.allowance(providers.wallet_address, c.outbox_arb_to_gnosis).call().await.unwrap();
    println!("Final allowance: {}", final_allowance);

    assert_eq!(final_allowance, manual_approval, "Allowance should remain unchanged when already set");

    println!("\n✅ STARTUP TEST PASSED!");
    println!("The startup logic correctly:");
    println!("  1. Detected existing WETH approval");
    println!("  2. Skipped setting new approval (no wasted gas)");

    fixture.revert_snapshots().await.unwrap();
}
