use alloy::primitives::{Address, FixedBytes, U256};
use crate::config::Route;
use crate::contracts::{IVeaInboxArbToEth, IVeaInboxArbToGnosis, Claim, Party};
use crate::tasks::send_tx;

pub async fn execute(
    route: &Route,
    epoch: u64,
    state_root: FixedBytes<32>,
    claimer: Address,
    timestamp_claimed: u32,
    challenger: Address,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let claim = Claim {
        stateRoot: state_root,
        claimer,
        timestampClaimed: timestamp_claimed,
        timestampVerification: 0,
        blocknumberVerification: 0,
        honest: Party::None,
        challenger,
    };

    if route.weth_address.is_some() {
        let inbox = IVeaInboxArbToGnosis::new(route.inbox_address, route.inbox_provider.clone());
        let gas_limit = U256::from(500000);
        send_tx(
            inbox.sendSnapshot(U256::from(epoch), gas_limit, claim).send().await,
            "sendSnapshot",
            route.name,
            &[],
        ).await
    } else {
        let inbox = IVeaInboxArbToEth::new(route.inbox_address, route.inbox_provider.clone());
        send_tx(
            inbox.sendSnapshot(U256::from(epoch), claim).send().await,
            "sendSnapshot",
            route.name,
            &[],
        ).await
    }
}
