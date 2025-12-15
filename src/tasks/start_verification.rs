use alloy::primitives::{Address, FixedBytes, U256};
use crate::config::Route;
use crate::contracts::{IVeaOutbox, Claim, Party};
use crate::tasks::send_tx;

pub async fn execute(
    route: &Route,
    epoch: u64,
    state_root: FixedBytes<32>,
    claimer: Address,
    timestamp_claimed: u32,
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

    let outbox = IVeaOutbox::new(route.outbox_address, route.outbox_provider.clone());
    send_tx(
        outbox.startVerification(U256::from(epoch), claim).send().await,
        "startVerification",
        route.name,
        &["already"],
    ).await
}
