use alloy::primitives::{Address, FixedBytes, U256};
use alloy::providers::DynProvider;
use alloy::network::Ethereum;
use crate::contracts::{IVeaInboxArbToEth, IVeaOutboxArbToEth};
use crate::tasks::send_tx;

pub async fn execute(
    inbox_provider: DynProvider<Ethereum>,
    inbox_address: Address,
    outbox_provider: DynProvider<Ethereum>,
    outbox_address: Address,
    epoch: u64,
    _route_name: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let inbox = IVeaInboxArbToEth::new(inbox_address, inbox_provider);
    let outbox = IVeaOutboxArbToEth::new(outbox_address, outbox_provider);

    let state_root = inbox.snapshots(U256::from(epoch)).call().await?;
    if state_root == FixedBytes::<32>::ZERO {
        return Ok(());
    }

    let claim_hash = outbox.claimHashes(U256::from(epoch)).call().await?;
    if claim_hash != FixedBytes::<32>::ZERO {
        return Ok(());
    }

    let deposit = outbox.deposit().call().await?;
    send_tx(
        outbox.claim(U256::from(epoch), state_root).value(deposit).send().await,
        &["already"],
    ).await
}
