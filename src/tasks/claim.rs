use alloy::primitives::{FixedBytes, U256};
use crate::config::Route;
use crate::contracts::{IVeaInbox, IVeaOutboxArbToEth, IVeaOutboxArbToGnosis};
use crate::tasks::send_tx;

pub async fn execute(
    route: &Route,
    epoch: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let inbox = IVeaInbox::new(route.inbox_address, route.inbox_provider.clone());

    let state_root = inbox.snapshots(U256::from(epoch)).call().await?;
    if state_root == FixedBytes::<32>::ZERO {
        return Ok(());
    }

    if route.weth_address.is_some() {
        let outbox = IVeaOutboxArbToGnosis::new(route.outbox_address, route.outbox_provider.clone());
        let claim_hash = outbox.claimHashes(U256::from(epoch)).call().await?;
        if claim_hash != FixedBytes::<32>::ZERO {
            return Ok(());
        }
        send_tx(
            outbox.claim(U256::from(epoch), state_root).send().await,
            "claim",
            route.name,
            &["already"],
        ).await
    } else {
        let outbox = IVeaOutboxArbToEth::new(route.outbox_address, route.outbox_provider.clone());
        let claim_hash = outbox.claimHashes(U256::from(epoch)).call().await?;
        if claim_hash != FixedBytes::<32>::ZERO {
            return Ok(());
        }
        let deposit = outbox.deposit().call().await?;
        send_tx(
            outbox.claim(U256::from(epoch), state_root).value(deposit).send().await,
            "claim",
            route.name,
            &["already"],
        ).await
    }
}
