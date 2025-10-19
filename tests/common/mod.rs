pub use alloy::providers::Provider;
use std::sync::Arc;

pub struct TestFixture<P1: Provider, P2: Provider> {
    pub eth_provider: Arc<P1>,
    pub arb_provider: Arc<P2>,
    eth_snapshot_id: Option<String>,
    arb_snapshot_id: Option<String>,
}

impl<P1: Provider, P2: Provider> TestFixture<P1, P2> {
    pub fn new(eth_provider: Arc<P1>, arb_provider: Arc<P2>) -> Self {
        Self {
            eth_provider,
            arb_provider,
            eth_snapshot_id: None,
            arb_snapshot_id: None,
        }
    }

    pub async fn take_snapshots(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Sync both chains to same timestamp before taking snapshot
        // This prevents epoch calculation issues when Anvil instances have been running for different durations
        let arb_block = self.arb_provider.get_block_by_number(Default::default()).await?;
        let eth_block = self.eth_provider.get_block_by_number(Default::default()).await?;

        if let (Some(arb_blk), Some(eth_blk)) = (arb_block, eth_block) {
            let arb_time = arb_blk.header.timestamp;
            let eth_time = eth_blk.header.timestamp;

            // Align both chains to the higher timestamp
            if arb_time > eth_time {
                let diff = arb_time - eth_time;
                let _: serde_json::Value = self.eth_provider
                    .raw_request("evm_increaseTime".into(), vec![serde_json::json!(diff)])
                    .await?;
                let _: serde_json::Value = self.eth_provider
                    .raw_request("evm_mine".into(), Vec::<serde_json::Value>::new())
                    .await?;
            } else if eth_time > arb_time {
                let diff = eth_time - arb_time;
                let _: serde_json::Value = self.arb_provider
                    .raw_request("evm_increaseTime".into(), vec![serde_json::json!(diff)])
                    .await?;
                let _: serde_json::Value = self.arb_provider
                    .raw_request("evm_mine".into(), Vec::<serde_json::Value>::new())
                    .await?;
            }
        }

        let empty_params: Vec<serde_json::Value> = vec![];

        let eth_snapshot: serde_json::Value = self.eth_provider
            .raw_request("evm_snapshot".into(), empty_params.clone())
            .await?;
        self.eth_snapshot_id = Some(eth_snapshot.as_str().unwrap().to_string());

        let arb_snapshot: serde_json::Value = self.arb_provider
            .raw_request("evm_snapshot".into(), empty_params)
            .await?;
        self.arb_snapshot_id = Some(arb_snapshot.as_str().unwrap().to_string());

        Ok(())
    }

    pub async fn revert_snapshots(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref snapshot_id) = self.eth_snapshot_id {
            let _: serde_json::Value = self.eth_provider
                .raw_request("evm_revert".into(), vec![serde_json::json!(snapshot_id)])
                .await?;
        }

        if let Some(ref snapshot_id) = self.arb_snapshot_id {
            let _: serde_json::Value = self.arb_provider
                .raw_request("evm_revert".into(), vec![serde_json::json!(snapshot_id)])
                .await?;
        }

        Ok(())
    }
}

pub async fn advance_time<P: Provider>(provider: &P, seconds: u64) {
    let _: serde_json::Value = provider
        .raw_request("evm_increaseTime".into(), vec![serde_json::json!(seconds)])
        .await
        .expect("Failed to advance time");

    let empty_params: Vec<serde_json::Value> = vec![];
    let _: serde_json::Value = provider
        .raw_request("evm_mine".into(), empty_params)
        .await
        .expect("Failed to mine block");
}
