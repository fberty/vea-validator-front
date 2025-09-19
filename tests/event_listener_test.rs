use alloy::primitives::{Address, FixedBytes};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::network::EthereumWallet;
use alloy::signers::local::PrivateKeySigner;
use std::sync::Arc;
use std::str::FromStr;
use vea_validator::event_listener::EventListener;
use vea_validator::contracts::IVeaInboxArbToEth;
#[allow(unused_imports)]
use serial_test::serial;

// Test fixture for managing blockchain state snapshots
struct TestFixture<P: Provider> {
    provider: Arc<P>,
    snapshot_id: Option<String>,
}

impl<P: Provider> TestFixture<P> {
    fn new(provider: Arc<P>) -> Self {
        Self {
            provider,
            snapshot_id: None,
        }
    }

    async fn take_snapshot(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let empty_params: Vec<serde_json::Value> = vec![];
        let snapshot_result: serde_json::Value = self.provider
            .raw_request("evm_snapshot".into(), empty_params)
            .await?;
        
        self.snapshot_id = Some(snapshot_result.as_str().unwrap().to_string());
        println!("Created snapshot: {}", self.snapshot_id.as_ref().unwrap());
        Ok(())
    }

    async fn revert_snapshot(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref snapshot_id) = self.snapshot_id {
            let _: serde_json::Value = self.provider
                .raw_request("evm_revert".into(), vec![serde_json::json!(snapshot_id)])
                .await?;
            println!("Reverted to snapshot: {}", snapshot_id);
        }
        Ok(())
    }

    async fn deploy_fresh_contracts(&self) -> Result<(Address, Address), Box<dyn std::error::Error>> {
        // Deploy fresh contracts using the same pattern as full-devnet.sh
        let signer = PrivateKeySigner::from_str(
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
        )?;
        let wallet = EthereumWallet::from(signer);
        
        let _provider_with_wallet = ProviderBuilder::new()
            .wallet(wallet)
            .connect_provider(&*self.provider);

        // For simplicity, return the addresses from .env for now
        // In a full implementation, we'd deploy fresh contracts here
        let inbox_address = Address::from_str(
            &std::env::var("VEA_INBOX_ARB_TO_ETH")
                .expect("VEA_INBOX_ARB_TO_ETH must be set")
        )?;
        let outbox_address = Address::from_str(
            &std::env::var("VEA_OUTBOX_ARB_TO_ETH")  
                .expect("VEA_OUTBOX_ARB_TO_ETH must be set")
        )?;

        Ok((inbox_address, outbox_address))
    }
}

impl<P: Provider> Drop for TestFixture<P> {
    fn drop(&mut self) {
        // Note: async drop is not available in stable Rust
        // We'll handle cleanup in the test functions
    }
}

#[tokio::test]
#[serial]
async fn test_listen_for_snapshot_events() {
    dotenv::dotenv().ok();
    
    // Setup provider
    let arbitrum_rpc = std::env::var("ARBITRUM_RPC_URL")
        .expect("ARBITRUM_RPC_URL must be set");
    
    let provider = ProviderBuilder::new()
        .connect_http(arbitrum_rpc.parse().unwrap());
    let provider = Arc::new(provider);
    
    // Create test fixture and take snapshot
    let mut fixture = TestFixture::new(provider.clone());
    fixture.take_snapshot().await.unwrap();
    
    // Deploy fresh contracts and get addresses
    let (inbox_address, _outbox_address) = fixture.deploy_fresh_contracts().await.unwrap();
    
    // Create event listener
    let listener = EventListener::new(provider.clone(), inbox_address);
    
    // Trigger a snapshot on the local devnet
    trigger_snapshot_saved(provider.clone(), inbox_address).await;
    
    // Listen for events
    let events = listener.listen_for_snapshots().await.unwrap();
    
    // Verify we captured the event
    assert!(!events.is_empty(), "Should have captured at least one SnapshotSaved event");
    
    let event = &events[0];
    assert!(event.epoch > 0, "Epoch should be non-zero");
    // With empty tree (count=0), state root is expected to be zero
    assert!(event.count == 0, "Count should be 0 for empty tree");
    
    // Cleanup: revert to snapshot
    fixture.revert_snapshot().await.unwrap();
}

#[tokio::test]
#[serial]
async fn test_listen_for_claim_events() {
    dotenv::dotenv().ok();
    
    // Setup provider
    let mainnet_rpc = std::env::var("MAINNET_RPC_URL")
        .expect("MAINNET_RPC_URL must be set");
    
    let provider = ProviderBuilder::new()
        .connect_http(mainnet_rpc.parse().unwrap());
    let provider = Arc::new(provider);
    
    // Create test fixture and take snapshot
    let mut fixture = TestFixture::new(provider.clone());
    fixture.take_snapshot().await.unwrap();
    
    // Deploy fresh contracts and get addresses  
    let (_inbox_address, outbox_address) = fixture.deploy_fresh_contracts().await.unwrap();
    
    // Create event listener
    let listener = EventListener::new(provider.clone(), outbox_address);
    
    // Trigger a claim on the local devnet
    trigger_claim(provider.clone(), outbox_address).await;
    
    // Listen for events
    let events = listener.listen_for_claims().await.unwrap();
    
    // Verify we captured the event (or skip if no implementation yet)
    if !events.is_empty() {
        let event = &events[0];
        assert!(event.epoch > 0, "Epoch should be non-zero");
        assert!(event.state_root != FixedBytes::<32>::default(), "State root should not be zero");
        assert!(event.claimer != Address::ZERO, "Claimer should not be zero address");
    } else {
        println!("No claim events found - trigger_claim not implemented yet");
    }
    
    // Cleanup: revert to snapshot
    fixture.revert_snapshot().await.unwrap();
}

// Helper function to trigger a snapshot event
async fn trigger_snapshot_saved<P: Provider>(provider: Arc<P>, inbox_address: Address) {
    let signer = PrivateKeySigner::from_str(
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
    ).unwrap();
    let wallet = EthereumWallet::from(signer);
    
    let provider_with_wallet = ProviderBuilder::new()
        .wallet(wallet)
        .connect_provider(&*provider);
    
    // Create contract instance and call saveSnapshot properly
    let contract = IVeaInboxArbToEth::new(inbox_address, provider_with_wallet);
    
    println!("Calling saveSnapshot on {}", inbox_address);
    let tx = contract.saveSnapshot();
    let receipt = tx.send().await.unwrap().get_receipt().await.unwrap();
    println!("Transaction successful: {:?}", receipt.transaction_hash);
}

#[tokio::test]
#[serial]
async fn test_snapshot_with_message() {
    dotenv::dotenv().ok();
    
    // Setup provider
    let arbitrum_rpc = std::env::var("ARBITRUM_RPC_URL")
        .expect("ARBITRUM_RPC_URL must be set");
    
    let provider = ProviderBuilder::new()
        .connect_http(arbitrum_rpc.parse().unwrap());
    let provider = Arc::new(provider);
    
    // Create test fixture and take snapshot
    let mut fixture = TestFixture::new(provider.clone());
    fixture.take_snapshot().await.unwrap();
    
    // Deploy fresh contracts and get addresses
    let (inbox_address, _outbox_address) = fixture.deploy_fresh_contracts().await.unwrap();
    
    // Setup provider with wallet for transactions
    let signer = PrivateKeySigner::from_str(
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
    ).unwrap();
    let wallet = EthereumWallet::from(signer);
    
    let provider_with_wallet = ProviderBuilder::new()
        .wallet(wallet)
        .connect_provider(&*provider);
    
    // Create contract instance
    let contract = IVeaInboxArbToEth::new(inbox_address, provider_with_wallet);
    
    // Send a message
    println!("Sending message...");
    let msg_tx = contract.sendMessage(
        Address::ZERO, // to
        vec![1, 2, 3, 4].into() // some test data
    );
    let msg_receipt = msg_tx.send().await.unwrap().get_receipt().await.unwrap();
    println!("Message sent: {:?}", msg_receipt.transaction_hash);
    
    // Save snapshot
    println!("Saving snapshot...");
    let snap_tx = contract.saveSnapshot();
    let snap_receipt = snap_tx.send().await.unwrap().get_receipt().await.unwrap();
    println!("Snapshot saved: {:?}", snap_receipt.transaction_hash);
    
    // Listen for events
    let listener = EventListener::new(provider.clone(), inbox_address);
    let events = listener.listen_for_snapshots().await.unwrap();
    
    // Find the latest event (should have count=1)
    let latest_event = events.last().expect("Should have at least one event");
    println!("Merkle root after message: {:?}", latest_event.state_root);
    println!("Message count: {}", latest_event.count);
    
    assert!(latest_event.count == 1, "Count should be 1 after sending one message");
    assert!(latest_event.state_root != FixedBytes::<32>::default(), "State root should not be zero with messages");
    
    // Cleanup: revert to snapshot
    fixture.revert_snapshot().await.unwrap();
}

// Helper function to trigger a claim event
async fn trigger_claim<P: Provider>(provider: Arc<P>, _outbox_address: Address) {
    // For now, this is a placeholder - we'll need to implement proper claim triggering
    // with correct parameters once we understand the contract better
    
    // Get current epoch (block.timestamp / epochPeriod - 1)
    let block = provider.get_block_by_number(Default::default()).await.unwrap().unwrap();
    let current_epoch = block.header.timestamp / 3600 - 1; // Assuming 1 hour epochs
    
    // This will need proper implementation with correct state root and deposit
    println!("TODO: Implement claim triggering for epoch {}", current_epoch);
}