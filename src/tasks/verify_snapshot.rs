use alloy::primitives::{Address, U256};
use crate::config::Route;
use crate::contracts::IVeaOutbox;
use crate::tasks::{send_tx, ClaimStore};

pub async fn execute(
    route: &Route,
    epoch: u64,
    claim_store: &ClaimStore,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let claim_data = claim_store.get(epoch);
    if claim_data.challenger != Address::ZERO {
        println!("[{}][task::verify_snapshot] Epoch {} already challenged, dropping task", route.name, epoch);
        return Ok(());
    }

    let claim = claim_store.get_claim(epoch);
    let outbox = IVeaOutbox::new(route.outbox_address, route.outbox_provider.clone());
    send_tx(
        outbox.verifySnapshot(U256::from(epoch), claim).send().await,
        "verifySnapshot",
        route.name,
        &["already", "challenged"],
    ).await
}
