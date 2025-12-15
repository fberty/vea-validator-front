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
    timestamp_verification: u32,
    blocknumber_verification: u32,
    _route_name: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let claim = Claim {
        stateRoot: state_root,
        claimer,
        timestampClaimed: timestamp_claimed,
        timestampVerification: timestamp_verification,
        blocknumberVerification: blocknumber_verification,
        honest: Party::None,
        challenger: Address::ZERO,
    };

    let outbox = IVeaOutbox::new(outbox_address, outbox_provider);
    send_tx(
        outbox.verifySnapshot(U256::from(epoch), claim).send().await,
        &["already"],
    ).await
}
