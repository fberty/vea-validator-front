use alloy::providers::Provider;
use tokio::time::{sleep, Duration};

use crate::config::Route;
use crate::tasks;

const BEFORE_EPOCH_BUFFER: u64 = 60;
const AFTER_EPOCH_BUFFER: u64 = 15 * 60;

pub struct EpochWatcher {
    route: Route,
    make_claims: bool,
}

impl EpochWatcher {
    pub fn new(route: Route, make_claims: bool) -> Self {
        Self { route, make_claims }
    }

    async fn get_current_timestamp(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let block = self.route.inbox_provider.get_block_by_number(Default::default()).await?.unwrap();
        Ok(block.header.timestamp)
    }

    pub async fn watch_epochs(&self, epoch_period: u64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut last_before_epoch: Option<u64> = None;
        let mut last_after_epoch: Option<u64> = None;
        loop {
            let now = self.get_current_timestamp().await?;
            let current_epoch = now / epoch_period;
            let next_epoch_start = (current_epoch + 1) * epoch_period;
            let time_until_next_epoch = next_epoch_start.saturating_sub(now);

            if time_until_next_epoch <= BEFORE_EPOCH_BUFFER && last_before_epoch != Some(current_epoch) {
                println!("[{}] Triggering saveSnapshot for epoch {}", self.route.name, current_epoch);
                tasks::save_snapshot::execute(&self.route, current_epoch).await
                    .unwrap_or_else(|e| panic!("[{}] FATAL: Failed to save snapshot for epoch {}: {}", self.route.name, current_epoch, e));
                last_before_epoch = Some(current_epoch);
            }

            if self.make_claims {
                let time_since_epoch_start = now.saturating_sub(current_epoch * epoch_period);
                if time_since_epoch_start >= AFTER_EPOCH_BUFFER && current_epoch > 0 {
                    let prev_epoch = current_epoch - 1;
                    if last_after_epoch != Some(prev_epoch) {
                        println!("[{}] Triggering claim for epoch {}", self.route.name, prev_epoch);
                        tasks::claim::execute(&self.route, prev_epoch).await
                            .unwrap_or_else(|e| panic!("[{}] FATAL: Failed to handle claim for epoch {}: {}", self.route.name, prev_epoch, e));
                        last_after_epoch = Some(prev_epoch);
                    }
                }
            }

            sleep(Duration::from_secs(10)).await;
        }
    }
}
