use anyhow::Result;
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::Filter;
use alloy::primitives::Address;
use alloy::sol_types::SolEvent;
use tokio::task::JoinHandle;

use crate::contracts::{
    IVeaInboxArbToEth::SnapshotSaved,
    IVeaOutboxArbToEth::Claimed,
};

pub struct EventListener {
    pub inbox_address: Address,
    pub outbox_address: Address,
    pub arbitrum_rpc_url: String,
    pub mainnet_rpc_url: String,
}

impl EventListener {
    pub fn new(
        inbox_address: Address,
        outbox_address: Address,
        arbitrum_rpc_url: String,
        mainnet_rpc_url: String,
    ) -> Self {
        Self {
            inbox_address,
            outbox_address,
            arbitrum_rpc_url,
            mainnet_rpc_url,
        }
    }

    pub async fn start_listening(&self) -> Result<Vec<JoinHandle<Result<()>>>> {
        let mut handles = vec![];

        // Listen to Inbox events on Arbitrum
        handles.push(self.listen_inbox_events());
        
        // Listen to Outbox events on Mainnet
        handles.push(self.listen_outbox_events());

        Ok(handles)
    }

    fn listen_inbox_events(&self) -> JoinHandle<Result<()>> {
        let inbox_address = self.inbox_address;
        let arbitrum_rpc_url = self.arbitrum_rpc_url.clone();

        tokio::spawn(async move {
            println!("Connecting to Arbitrum RPC: {}", arbitrum_rpc_url);
            
            // For now, using HTTP polling instead of WebSocket
            // TODO: Switch to WebSocket subscription when available
            let provider = ProviderBuilder::new()
                .connect(&arbitrum_rpc_url).await?;
            
            println!("Listening for SnapshotSaved events on Inbox at {:?}...", inbox_address);
            
            loop {
                let snapshot_filter = Filter::new()
                    .address(inbox_address)
                    .event_signature(SnapshotSaved::SIGNATURE_HASH);
                
                match provider.get_logs(&snapshot_filter).await {
                    Ok(logs) => {
                        for log in logs {
                            println!("Found SnapshotSaved event: {:?}", log);
                            // TODO: Decode and handle snapshot saved event
                        }
                    }
                    Err(e) => {
                        eprintln!("Error fetching logs: {}", e);
                    }
                }
                
                // Poll every 10 seconds
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            }
        })
    }

    fn listen_outbox_events(&self) -> JoinHandle<Result<()>> {
        let outbox_address = self.outbox_address;
        let mainnet_rpc_url = self.mainnet_rpc_url.clone();

        tokio::spawn(async move {
            println!("Connecting to Mainnet RPC: {}", mainnet_rpc_url);
            
            let provider = ProviderBuilder::new()
                .connect(&mainnet_rpc_url).await?;
            
            println!("Listening for Claimed events on Outbox at {:?}...", outbox_address);
            
            loop {
                let claimed_filter = Filter::new()
                    .address(outbox_address)
                    .event_signature(Claimed::SIGNATURE_HASH);
                
                match provider.get_logs(&claimed_filter).await {
                    Ok(logs) => {
                        for log in logs {
                            println!("Found Claimed event: {:?}", log);
                            // TODO: Decode and validate claim against our snapshot
                        }
                    }
                    Err(e) => {
                        eprintln!("Error fetching logs: {}", e);
                    }
                }
                
                // Poll every 10 seconds
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            }
        })
    }
}