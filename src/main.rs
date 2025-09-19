use anyhow::Result;
use alloy::primitives::Address;
use std::str::FromStr;
use vea_validator::listener::EventListener;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    
    println!("VEA Validator starting...");
    
    // Load configuration from environment
    let inbox_address = Address::from_str(
        &std::env::var("VEA_INBOX_ARB_TO_ETH")
            .expect("VEA_INBOX_ARB_TO_ETH must be set")
    )?;
    
    let outbox_address = Address::from_str(
        &std::env::var("VEA_OUTBOX_ARB_TO_ETH")
            .expect("VEA_OUTBOX_ARB_TO_ETH must be set")
    )?;
    
    let arbitrum_rpc = std::env::var("ARBITRUM_RPC_URL")
        .expect("ARBITRUM_RPC_URL must be set");
    
    let mainnet_rpc = std::env::var("MAINNET_RPC_URL")
        .expect("MAINNET_RPC_URL must be set");
    
    // Create event listener
    let listener = EventListener::new(
        inbox_address,
        outbox_address,
        arbitrum_rpc,
        mainnet_rpc,
    );
    
    // Start listening for events
    let handles = listener.start_listening().await?;
    
    println!("Event listeners started. Press Ctrl+C to stop.");
    
    // Wait for all listeners
    for handle in handles {
        handle.await??;
    }
    
    Ok(())
}
