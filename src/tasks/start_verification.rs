use alloy::primitives::{Address, FixedBytes, U256};
use alloy::providers::DynProvider;
use alloy::network::Ethereum;
use crate::contracts::{IVeaOutbox, Claim, Party};
use crate::tasks::send_tx;

pub async fn execute(
    outbox_provider: DynProvider<Ethereum>,
    outbox_address: Address,
    epoch: u64,
    state_root: FixedBytes<32>,
    claimer: Address,
    timestamp_claimed: u32,
    route_name: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let claim = Claim {
        stateRoot: state_root,
        claimer,
        timestampClaimed: timestamp_claimed,
        timestampVerification: 0,
        blocknumberVerification: 0,
        honest: Party::None,
        challenger: Address::ZERO,
    };

    let outbox = IVeaOutbox::new(outbox_address, outbox_provider);
    send_tx(
        outbox.startVerification(U256::from(epoch), claim).send().await,
        "startVerification",
        route_name,
        &["already"],
    ).await
}
