use anyhow::Result;
use alloy::primitives::Address;
use std::str::FromStr;
use vea_validator::listener::EventListener;

#[tokio::test]
async fn test_event_listener_creation() -> Result<()> {
    let inbox_address = Address::from_str("0x0000000000000000000000000000000000000001")?;
    let outbox_address = Address::from_str("0x0000000000000000000000000000000000000002")?;
    
    let listener = EventListener::new(
        inbox_address,
        outbox_address,
        "http://localhost:8545".to_string(),
        "http://localhost:8546".to_string(),
    );
    
    // Test that we can create a listener
    assert_eq!(listener.inbox_address, inbox_address);
    assert_eq!(listener.outbox_address, outbox_address);
    
    Ok(())
}

#[tokio::test]
async fn test_start_listening_handles() -> Result<()> {
    let inbox_address = Address::from_str("0x0000000000000000000000000000000000000001")?;
    let outbox_address = Address::from_str("0x0000000000000000000000000000000000000002")?;
    
    let listener = EventListener::new(
        inbox_address,
        outbox_address,
        "http://localhost:8545".to_string(),
        "http://localhost:8546".to_string(),
    );
    
    let handles = listener.start_listening().await?;
    
    // Should create 2 handles (one for inbox, one for outbox)
    assert_eq!(handles.len(), 2);
    
    // Abort the handles to clean up
    for handle in handles {
        handle.abort();
    }
    
    Ok(())
}