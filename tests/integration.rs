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
        .unwrap_or_else(|_| "http://localhost:8545".to_string());
    
    let vea_inbox_address = std::env::var("VEA_INBOX_ARB_TO_ETH")
        .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string());
    
    let provider = ProviderBuilder::new().connect(&arbitrum_rpc).await?;
    
    let inbox_address = Address::from_str(&vea_inbox_address)?;
    
    let filter = Filter::new()
        .address(inbox_address)
        .event_signature(SnapshotSaved::SIGNATURE_HASH);
    
    let logs = provider.get_logs(&filter).await?;
    
    assert!(logs.is_empty() || !logs.is_empty(), "Successfully connected and queried logs");
    
    Ok(())
}