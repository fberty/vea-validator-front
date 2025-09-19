use alloy::primitives::{Address, FixedBytes, U256};
use alloy::providers::Provider;
use alloy::rpc::types::Filter;
use alloy::primitives::keccak256;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SnapshotEvent {
    pub epoch: u64,
    pub state_root: FixedBytes<32>,
    pub count: u64,
}

#[derive(Debug, Clone)]
pub struct ClaimEvent {
    pub epoch: u64,
    pub state_root: FixedBytes<32>,
    pub claimer: Address,
}

pub struct EventListener<P: Provider> {
    provider: Arc<P>,
    contract_address: Address,
}

impl<P: Provider> EventListener<P> {
    pub fn new(provider: Arc<P>, contract_address: Address) -> Self {
        Self {
            provider,
            contract_address,
        }
    }

    pub async fn listen_for_snapshots(&self) -> Result<Vec<SnapshotEvent>, Box<dyn std::error::Error>> {
        // Event signature for SnapshotSaved(bytes32,uint256,uint64)
        let event_signature = "SnapshotSaved(bytes32,uint256,uint64)";
        let event_hash = keccak256(event_signature.as_bytes());

        let filter = Filter::new()
            .address(self.contract_address)
            .event_signature(event_hash)
            .from_block(0u64);

        let logs = self.provider.get_logs(&filter).await?;
        
        let mut events = Vec::new();
        for log in logs {
            // SnapshotSaved has NO indexed params - all in data
            // data layout: bytes32 snapshot, uint256 epoch, uint64 count
            if log.data().data.len() >= 96 {
                let state_root = FixedBytes::<32>::from_slice(&log.data().data[0..32]);
                let epoch = U256::from_be_slice(&log.data().data[32..64]).to::<u64>();
                let count = U256::from_be_slice(&log.data().data[64..96]).to::<u64>();
                
                events.push(SnapshotEvent {
                    epoch,
                    state_root,
                    count,
                });
            }
        }

        Ok(events)
    }

    pub async fn listen_for_claims(&self) -> Result<Vec<ClaimEvent>, Box<dyn std::error::Error>> {
        // Event signature for Claimed(address,uint256,bytes32)
        let event_signature = "Claimed(address,uint256,bytes32)";
        let event_hash = keccak256(event_signature.as_bytes());

        let filter = Filter::new()
            .address(self.contract_address)
            .event_signature(event_hash)
            .from_block(0u64);

        let logs = self.provider.get_logs(&filter).await?;
        
        let mut events = Vec::new();
        for log in logs {
            if log.topics().len() >= 3 {
                // Claimed event has indexed claimer and epoch
                // topics[0] = event signature
                // topics[1] = claimer (indexed address)
                // topics[2] = epoch (indexed uint256)
                // data = stateRoot (bytes32)
                let claimer = Address::from_slice(&log.topics()[1].0[12..]);
                let epoch = U256::from_be_bytes(log.topics()[2].0).to::<u64>();
                
                let state_root = if log.data().data.len() >= 32 {
                    FixedBytes::<32>::from_slice(&log.data().data[0..32])
                } else {
                    FixedBytes::<32>::default()
                };
                
                events.push(ClaimEvent {
                    epoch,
                    state_root,
                    claimer,
                });
            }
        }

        Ok(events)
    }
}