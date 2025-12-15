use alloy::primitives::{Address, FixedBytes, U256};
use alloy::providers::DynProvider;
use alloy::network::Ethereum;
use crate::contracts::{IVeaOutboxArbToEth, IVeaOutboxArbToGnosis, Claim, Party};
use crate::tasks::send_tx;

pub async fn execute(
    outbox_provider: DynProvider<Ethereum>,
    outbox_address: Address,
    weth_address: Option<Address>,
    wallet_address: Address,
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

    if weth_address.is_some() {
        let outbox = IVeaOutboxArbToGnosis::new(outbox_address, outbox_provider);
        send_tx(
            outbox.challenge(U256::from(epoch), claim).send().await,
            "challenge",
            route_name,
            &["Invalid claim", "already"],
        ).await
    } else {
        let outbox = IVeaOutboxArbToEth::new(outbox_address, outbox_provider);
        let deposit = outbox.deposit().call().await?;
        send_tx(
            outbox.challenge(U256::from(epoch), claim, wallet_address).value(deposit).send().await,
            "challenge",
            route_name,
            &["Invalid claim", "already"],
        ).await
    }
}
