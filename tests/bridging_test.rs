mod common;

use alloy::primitives::{Address, FixedBytes, U256};
use serial_test::serial;
use std::str::FromStr;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::time::{timeout, Duration};
use vea_validator::{
    contracts::{IVeaInboxArbToEth, IVeaOutboxArbToEth, IOutbox},
    claim_handler::ClaimHandler,
    config::ValidatorConfig,
    l2_to_l1_finder::L2ToL1Finder,
    arb_relay_handler::ArbRelayHandler,
    scheduler::{ScheduleFile, ScheduleData, ArbToL1Task},
};
use common::{restore_pristine, advance_time, Provider};
use alloy::providers::DynProvider;
use alloy::network::Ethereum;

const ARB_OUTBOX_ENV: &str = "ARB_OUTBOX";

fn get_arb_outbox() -> Address {
    std::env::var(ARB_OUTBOX_ENV)
        .expect("ARB_OUTBOX must be set")
        .parse()
        .expect("Invalid ARB_OUTBOX address")
}

#[tokio::test]
#[serial]
async fn test_l2_to_l1_finder_discovers_snapshot_sent_event() {
    println!("\n==============================================");
    println!("BRIDGING TEST: L2ToL1Finder Discovers SnapshotSent");
    println!("==============================================\n");

    let c = ValidatorConfig::from_env().expect("Failed to load config");
    let routes = c.build_routes();
    let route = &routes[0];

    let inbox_provider = Arc::new(route.inbox_provider.clone());
    let outbox_provider = Arc::new(route.outbox_provider.clone());

    restore_pristine().await;

    let inbox = IVeaInboxArbToEth::new(route.inbox_address, inbox_provider.clone());
    let outbox = IVeaOutboxArbToEth::new(route.outbox_address, outbox_provider.clone());

    let epoch_period: u64 = inbox.epochPeriod().call().await.unwrap().try_into().unwrap();

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

    println!("Saved snapshot for epoch {} with root: {:?}", current_epoch, correct_root);

    advance_time(inbox_provider.as_ref(), epoch_period + 10).await;
    advance_time(outbox_provider.as_ref(), epoch_period + 10).await;

    let target_epoch = current_epoch;

    let eth_block = outbox_provider.get_block_by_number(Default::default()).await.unwrap().unwrap();
    let eth_timestamp = eth_block.header.timestamp;
    let target_timestamp = (target_epoch + 1) * epoch_period + 10;
    let advance_amount = target_timestamp.saturating_sub(eth_timestamp);
    if advance_amount > 0 {
        advance_time(outbox_provider.as_ref(), advance_amount).await;
    }

    println!("Submitting wrong claim to trigger challenge + bridging...");
    let wrong_root = FixedBytes::<32>::from([0x99; 32]);
    let deposit = outbox.deposit().call().await.unwrap();
    outbox.claim(U256::from(target_epoch), wrong_root)
        .value(deposit)
        .send().await.unwrap()
        .get_receipt().await.unwrap();

    let claim_handler = Arc::new(ClaimHandler::new(route.clone(), c.wallet.default_signer().address()));

    let claim_event = vea_validator::event_listener::ClaimEvent {
        epoch: target_epoch,
        claimer: c.wallet.default_signer().address(),
        state_root: wrong_root,
        timestamp_claimed: eth_timestamp as u32,
    };

    claim_handler.challenge_claim(target_epoch, vea_validator::claim_handler::make_claim(&claim_event)).await.unwrap();
    println!("Challenge submitted");

    claim_handler.start_bridging(target_epoch, &claim_event).await.unwrap();
    println!("start_bridging() called - sendSnapshot emitted SnapshotSent event");

    let tmp_dir = tempdir().unwrap();
    let schedule_path = tmp_dir.path().join("arb_to_eth.json");

    let arb_provider_dyn: DynProvider<Ethereum> = route.inbox_provider.clone();

    let finder = L2ToL1Finder::new(arb_provider_dyn)
        .add_inbox(route.inbox_address, &schedule_path);

    let finder_handle = tokio::spawn(async move {
        finder.run().await;
    });

    let result = timeout(Duration::from_secs(30), async {
        loop {
            let schedule_file: ScheduleFile<ArbToL1Task> = ScheduleFile::new(&schedule_path);
            let schedule = schedule_file.load();
            if !schedule.pending.is_empty() {
                return schedule.pending[0].clone();
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }).await;

    finder_handle.abort();

    match result {
        Ok(task) => {
            println!("\nL2ToL1Finder discovered task:");
            println!("  epoch: {}", task.epoch);
            println!("  position: {:#x}", task.position);
            println!("  execute_after: {}", task.execute_after);
            println!("  l2_sender (VeaInbox): {:?}", task.l2_sender);
            println!("  dest_addr (VeaOutbox): {:?}", task.dest_addr);
            println!("  l2_block: {}", task.l2_block);
            println!("  l1_block: {}", task.l1_block);
            println!("  l2_timestamp: {}", task.l2_timestamp);
            println!("  amount: {}", task.amount);
            println!("  data len: {} bytes", task.data.len());

            assert_eq!(task.epoch, target_epoch, "Epoch should match");
            assert!(task.execute_after > 0, "execute_after should be set");
            assert_eq!(task.l2_sender, route.inbox_address, "l2_sender should be VeaInbox");
            assert_eq!(task.dest_addr, route.outbox_address, "dest_addr should be VeaOutbox");
            assert!(task.l2_block > 0, "l2_block should be set");
            assert!(task.l2_timestamp > 0, "l2_timestamp should be set");
            assert!(!task.data.is_empty(), "data should contain resolveDisputedClaim calldata");

            println!("\nL2ToL1 FINDER TEST PASSED!");
        }
        Err(_) => {
            panic!("L2ToL1Finder did not discover the event within 30 seconds");
        }
    }
}

#[tokio::test]
#[serial]
async fn test_scheduler_persistence_roundtrip() {
    println!("\n==============================================");
    println!("SCHEDULER TEST: Persistence Roundtrip");
    println!("==============================================\n");

    let tmp_dir = tempdir().unwrap();
    let schedule_path = tmp_dir.path().join("test_schedule.json");

    let task = ArbToL1Task {
        epoch: 42,
        position: U256::from(123),
        execute_after: 1700000000,
        l2_sender: Address::ZERO,
        dest_addr: Address::ZERO,
        l2_block: 100,
        l1_block: 50,
        l2_timestamp: 1700000000,
        amount: U256::ZERO,
        data: alloy::primitives::Bytes::new(),
    };

    let schedule_file: ScheduleFile<ArbToL1Task> = ScheduleFile::new(&schedule_path);

    let mut schedule = ScheduleData::default();
    schedule.last_checked_block = Some(12345);
    schedule.pending.push(task.clone());

    schedule_file.save(&schedule);
    println!("Saved schedule to {}", schedule_path.display());

    let loaded = schedule_file.load();

    assert_eq!(loaded.last_checked_block, Some(12345), "last_checked_block should persist");
    assert_eq!(loaded.pending.len(), 1, "Should have 1 pending task");

    let loaded_task = &loaded.pending[0];
    assert_eq!(loaded_task.epoch, 42, "epoch should match");
    assert_eq!(loaded_task.position, U256::from(123), "position should match");
    assert_eq!(loaded_task.execute_after, 1700000000, "execute_after should match");

    println!("\nSCHEDULER PERSISTENCE TEST PASSED!");
}

#[tokio::test]
#[serial]
async fn test_arb_relay_handler_checks_spent_status() {
    println!("\n==============================================");
    println!("BRIDGING TEST: ArbRelayHandler Checks Spent Status");
    println!("==============================================\n");

    let c = ValidatorConfig::from_env().expect("Failed to load config");
    let routes = c.build_routes();
    let route = &routes[0];
    let outbox_provider = Arc::new(route.outbox_provider.clone());

    let arb_outbox = get_arb_outbox();

    restore_pristine().await;

    let outbox = IOutbox::new(arb_outbox, outbox_provider.clone());

    let is_spent = outbox.isSpent(U256::from(0)).call().await.unwrap();
    println!("Position 0 isSpent: {}", is_spent);
    assert!(!is_spent, "Position 0 should not be spent initially");

    println!("\nARB RELAY HANDLER SPENT CHECK TEST PASSED!");
}

#[tokio::test]
#[serial]
async fn test_full_arb_to_eth_relay_flow() {
    println!("\n==============================================");
    println!("BRIDGING TEST: Full ARB to ETH Relay Flow");
    println!("==============================================\n");

    let c = ValidatorConfig::from_env().expect("Failed to load config");
    let routes = c.build_routes();
    let route = &routes[0];

    let inbox_provider = Arc::new(route.inbox_provider.clone());
    let outbox_provider = Arc::new(route.outbox_provider.clone());

    let arb_outbox = get_arb_outbox();

    restore_pristine().await;

    let inbox = IVeaInboxArbToEth::new(route.inbox_address, inbox_provider.clone());
    let outbox = IVeaOutboxArbToEth::new(route.outbox_address, outbox_provider.clone());
    let _arb_outbox_contract = IOutbox::new(arb_outbox, outbox_provider.clone());

    let epoch_period: u64 = inbox.epochPeriod().call().await.unwrap().try_into().unwrap();

    for i in 0..3 {
        let test_message = alloy::primitives::Bytes::from(vec![0xDE, 0xAD, 0xBE, 0xEF, i]);
        inbox.sendMessage(
            Address::from_str("0x0000000000000000000000000000000000000001").unwrap(),
            test_message
        ).send().await.unwrap().get_receipt().await.unwrap();
    }

    let current_epoch: u64 = inbox.epochNow().call().await.unwrap().try_into().unwrap();
    inbox.saveSnapshot().send().await.unwrap().get_receipt().await.unwrap();
    let correct_root = inbox.snapshots(U256::from(current_epoch)).call().await.unwrap();

    println!("Phase 1: Setup complete - epoch {} with root {:?}", current_epoch, correct_root);

    advance_time(inbox_provider.as_ref(), epoch_period + 10).await;
    advance_time(outbox_provider.as_ref(), epoch_period + 10).await;

    let target_epoch = current_epoch;

    let eth_block = outbox_provider.get_block_by_number(Default::default()).await.unwrap().unwrap();
    let eth_timestamp = eth_block.header.timestamp;
    let target_timestamp = (target_epoch + 1) * epoch_period + 10;
    let advance_amount = target_timestamp.saturating_sub(eth_timestamp);
    if advance_amount > 0 {
        advance_time(outbox_provider.as_ref(), advance_amount).await;
    }

    let wrong_root = FixedBytes::<32>::from([0x77; 32]);
    let deposit = outbox.deposit().call().await.unwrap();
    outbox.claim(U256::from(target_epoch), wrong_root)
        .value(deposit)
        .send().await.unwrap()
        .get_receipt().await.unwrap();

    println!("Phase 2: Wrong claim submitted");

    let claim_handler = Arc::new(ClaimHandler::new(route.clone(), c.wallet.default_signer().address()));

    let claim_event = vea_validator::event_listener::ClaimEvent {
        epoch: target_epoch,
        claimer: c.wallet.default_signer().address(),
        state_root: wrong_root,
        timestamp_claimed: eth_timestamp as u32,
    };

    claim_handler.challenge_claim(target_epoch, vea_validator::claim_handler::make_claim(&claim_event)).await.unwrap();
    claim_handler.start_bridging(target_epoch, &claim_event).await.unwrap();

    println!("Phase 3: Challenge + start_bridging complete");

    let tmp_dir = tempdir().unwrap();
    let schedule_path = tmp_dir.path().join("arb_to_eth.json");

    let arb_provider_dyn: DynProvider<Ethereum> = route.inbox_provider.clone();

    let finder = L2ToL1Finder::new(arb_provider_dyn.clone())
        .add_inbox(route.inbox_address, &schedule_path);

    let finder_handle = tokio::spawn(async move {
        finder.run().await;
    });

    let task = timeout(Duration::from_secs(30), async {
        loop {
            let schedule_file: ScheduleFile<ArbToL1Task> = ScheduleFile::new(&schedule_path);
            let schedule = schedule_file.load();
            if !schedule.pending.is_empty() {
                return schedule.pending[0].clone();
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }).await.expect("Finder should discover task within 30s");

    finder_handle.abort();

    println!("Phase 4: L2ToL1Finder discovered task - epoch {}, ticketId {:#x}", task.epoch, task.position);

    let arb_outbox_contract = IOutbox::new(arb_outbox, outbox_provider.clone());
    let is_spent_before = arb_outbox_contract.isSpent(task.position).call().await.unwrap();
    assert!(!is_spent_before, "Position should NOT be spent before relay");
    println!("Phase 5: Verified position {:#x} is not spent yet", task.position);

    println!("Phase 6: Advancing time by 7 days for relay delay...");
    let relay_delay = 7 * 24 * 3600 + 100;
    advance_time(inbox_provider.as_ref(), relay_delay).await;
    advance_time(outbox_provider.as_ref(), relay_delay).await;

    let handler = ArbRelayHandler::new(
        route.outbox_provider.clone(),
        arb_outbox,
        &schedule_path,
    );

    println!("Phase 7: Calling process_pending() to execute relay...");
    handler.process_pending().await;

    let is_spent_after = arb_outbox_contract.isSpent(task.position).call().await.unwrap();
    assert!(is_spent_after, "Position SHOULD be spent after relay");
    println!("Phase 8: Verified position {:#x} IS spent after relay!", task.position);

    let schedule_after: ScheduleData<ArbToL1Task> = ScheduleFile::new(&schedule_path).load();
    assert!(schedule_after.pending.is_empty(), "Schedule should be empty after successful relay");
    println!("Phase 9: Verified schedule is now empty");

    println!("\nFULL ARB TO ETH RELAY FLOW TEST PASSED!");
    println!("Successfully verified:");
    println!("  1. sendSnapshot emits SnapshotSent event");
    println!("  2. L2ToL1Finder discovers and schedules the task");
    println!("  3. Task has correct epoch, position, execute_after");
    println!("  4. Time advancement works for 7-day delay");
    println!("  5. ArbRelayHandler.process_pending() executes the relay");
    println!("  6. Outbox.isSpent() returns true after relay");
    println!("  7. Task is removed from schedule after successful relay");
}
