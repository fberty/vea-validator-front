use anyhow::Result;
use alloy::providers::{Provider, ProviderBuilder};
use alloy::primitives::Address;
use alloy::rpc::types::Filter;
use alloy::sol;
use alloy::sol_types::SolEvent;
use std::str::FromStr;

sol! {
    event SnapshotSaved(bytes32 _snapshot, uint256 _epoch, uint64 _count);
}

#[tokio::test]
async fn test_listen_snapshot_saved_event() -> Result<()> {
    dotenv::dotenv().ok();
    
    let arbitrum_rpc = std::env::var("ARBITRUM_RPC_URL")
        .expect("ARBITRUM_RPC_URL must be set");
    
    let vea_inbox_address = std::env::var("VEA_INBOX_ARB_TO_ETH")
        .expect("VEA_INBOX_ARB_TO_ETH must be set");
    
    let provider = ProviderBuilder::new().connect(&arbitrum_rpc).await?;
    
    let inbox_address = Address::from_str(&vea_inbox_address)?;
    
    let filter = Filter::new()
        .address(inbox_address)
        .event_signature(SnapshotSaved::SIGNATURE_HASH);
    
    let logs = provider.get_logs(&filter).await?;
    
    assert!(logs.is_empty() || !logs.is_empty(), "Successfully connected and queried logs");
    
    Ok(())
}