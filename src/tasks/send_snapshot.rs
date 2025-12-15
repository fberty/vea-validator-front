use alloy::primitives::{Address, FixedBytes, U256};
use alloy::providers::DynProvider;
use alloy::network::Ethereum;
use crate::contracts::{IVeaInboxArbToEth, IVeaInboxArbToGnosis, Claim, Party};
use crate::tasks::send_tx;

pub async fn execute(
    inbox_provider: DynProvider<Ethereum>,
    inbox_address: Address,
    weth_address: Option<Address>,
    epoch: u64,
    state_root: FixedBytes<32>,
    claimer: Address,
    timestamp_claimed: u32,
    challenger: Address,
    _route_name: &str,
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

    if weth_address.is_some() {
        let inbox = IVeaInboxArbToGnosis::new(inbox_address, inbox_provider);
        let gas_limit = U256::from(500000);
        send_tx(
            inbox.sendSnapshot(U256::from(epoch), gas_limit, claim).send().await,
            &[],
        ).await
    } else {
        let inbox = IVeaInboxArbToEth::new(inbox_address, inbox_provider);
        send_tx(
            inbox.sendSnapshot(U256::from(epoch), claim).send().await,
            &[],
        ).await
    }
}
