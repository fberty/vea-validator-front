use alloy::primitives::{FixedBytes, U256};
use crate::config::Route;
use crate::contracts::{IVeaInbox, IVeaOutbox, IVeaOutboxArbToEth, IVeaOutboxArbToGnosis};
use crate::tasks::{send_tx, ClaimStore};

const SEVEN_DAYS_SECS: u32 = 7 * 24 * 3600;

pub async fn execute(
    route: &Route,
    epoch: u64,
    claim_store: &ClaimStore,
    current_timestamp: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let inbox = IVeaInbox::new(route.inbox_address, route.inbox_provider.clone());
    let outbox = IVeaOutbox::new(route.outbox_address, route.outbox_provider.clone());

    let state_root = inbox.snapshots(U256::from(epoch)).call().await?;
    if state_root == FixedBytes::<32>::ZERO {
        println!("[{}] No snapshot for epoch {}, skipping claim", route.name, epoch);
        return Ok(());
    }

    let claim_hash = outbox.claimHashes(U256::from(epoch)).call().await?;
    if claim_hash != FixedBytes::<32>::ZERO {
        println!("[{}] Epoch {} already claimed, skipping", route.name, epoch);
        return Ok(());
    }

    let current_state_root = outbox.stateRoot().call().await?;
    if current_state_root == state_root {
        println!("[{}] State root for epoch {} already verified, skipping", route.name, epoch);
        return Ok(());
    }

    let since = (current_timestamp as u32).saturating_sub(SEVEN_DAYS_SECS);
    if claim_store.has_state_root_in_recent_claims(state_root, since) {
        println!("[{}] State root for epoch {} already in pending claim, skipping", route.name, epoch);
        return Ok(());
    }

    if route.weth_address.is_some() {
        let outbox = IVeaOutboxArbToGnosis::new(route.outbox_address, route.outbox_provider.clone());
        send_tx(
            outbox.claim(U256::from(epoch), state_root).send().await,
            "claim",
            route.name,
            &["already"],
        ).await
    } else {
        let outbox = IVeaOutboxArbToEth::new(route.outbox_address, route.outbox_provider.clone());
        let deposit = outbox.deposit().call().await?;
        send_tx(
            outbox.claim(U256::from(epoch), state_root).value(deposit).send().await,
            "claim",
            route.name,
            &["already"],
        ).await
    }
}
