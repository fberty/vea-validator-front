use alloy::primitives::{Address, FixedBytes, U256};
use alloy::providers::{DynProvider, Provider};
use alloy::network::Ethereum;
use alloy::rpc::types::Filter;
use std::path::PathBuf;
use std::cmp::min;
use tokio::time::{sleep, Duration};

use crate::scheduler::{AmbTask, ScheduleFile};

const CHUNK_SIZE: u64 = 500;
const FINALITY_BUFFER: u64 = 5;
const POLL_INTERVAL: Duration = Duration::from_secs(15 * 60);
const RETRY_DELAY: Duration = Duration::from_secs(5);
const AMB_RELAY_DELAY: u64 = 10 * 60;

pub struct AmbFinder {
    eth_provider: DynProvider<Ethereum>,
    router_address: Address,
    schedule_path: PathBuf,
}

impl AmbFinder {
    pub fn new(
        eth_provider: DynProvider<Ethereum>,
        router_address: Address,
        schedule_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            eth_provider,
            router_address,
            schedule_path: schedule_path.into(),
        }
    }

    pub async fn run(&self) {
        let schedule_file: ScheduleFile<AmbTask> = ScheduleFile::new(&self.schedule_path);
        let routed_sig = alloy::primitives::keccak256("Routed(uint256,bytes32)");

        loop {
            let mut schedule = schedule_file.load();
            let current_block = match self.eth_provider.get_block_number().await {
                Ok(b) => b.saturating_sub(FINALITY_BUFFER),
                Err(e) => {
                    eprintln!("[AmbFinder] Failed to get block number: {}, retrying...", e);
                    sleep(RETRY_DELAY).await;
                    continue;
                }
            };

            let from_block = schedule.last_checked_block.unwrap_or_else(|| {
                let ten_days_blocks = 10 * 24 * 3600 / 12;
                current_block.saturating_sub(ten_days_blocks)
            });

            if from_block >= current_block {
                println!("[AmbFinder] Caught up to block {}, waiting...", current_block);
                sleep(POLL_INTERVAL).await;
                continue;
            }

            let to_block = min(from_block + CHUNK_SIZE, current_block);

            let filter = Filter::new()
                .address(self.router_address)
                .event_signature(routed_sig)
                .from_block(from_block)
                .to_block(to_block);

            match self.eth_provider.get_logs(&filter).await {
                Ok(logs) => {
                    for log in logs {
                        if let Some(task) = self.parse_routed_event(&log) {
                            if !schedule.pending.iter().any(|t| t.ticket_id == task.ticket_id) {
                                println!(
                                    "[AmbFinder] Found Routed event: epoch={}, ticket_id={:#x}",
                                    task.epoch, task.ticket_id
                                );
                                schedule.pending.push(task);
                            }
                        }
                    }
                    schedule.last_checked_block = Some(to_block);
                    schedule_file.save(&schedule);
                    println!(
                        "[AmbFinder] Scanned blocks {}-{}, {} pending tasks",
                        from_block, to_block, schedule.pending.len()
                    );
                }
                Err(e) => {
                    eprintln!(
                        "[AmbFinder] Failed to query logs {}-{}: {}, retrying...",
                        from_block, to_block, e
                    );
                    sleep(RETRY_DELAY).await;
                }
            }
        }
    }

    fn parse_routed_event(&self, log: &alloy::rpc::types::Log) -> Option<AmbTask> {
        if log.topics().len() < 2 {
            return None;
        }

        let epoch = U256::from_be_bytes(log.topics()[1].0).to::<u64>();

        if log.data().data.len() < 32 {
            return None;
        }
        let ticket_id = FixedBytes::<32>::from_slice(&log.data().data[0..32]);

        let block_timestamp = log.block_timestamp.unwrap_or(0);
        let execute_after = block_timestamp + AMB_RELAY_DELAY;

        Some(AmbTask {
            epoch,
            ticket_id,
            execute_after,
        })
    }
}
