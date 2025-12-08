use alloy::primitives::{address, Address, Bytes, U256};
use alloy::providers::{DynProvider, Provider};
use alloy::network::Ethereum;
use alloy::rpc::types::Filter;
use std::path::PathBuf;
use std::cmp::min;
use tokio::time::{sleep, Duration};

use crate::scheduler::{ArbToL1Task, ScheduleFile};

const ARB_SYS_ADDRESS: Address = address!("0000000000000000000000000000000000000064");
const CHUNK_SIZE: u64 = 500;
const FINALITY_BUFFER: u64 = 20;
const POLL_INTERVAL: Duration = Duration::from_secs(15 * 60);
const RETRY_DELAY: Duration = Duration::from_secs(5);
const RELAY_DELAY: u64 = 7 * 24 * 3600;

pub struct L2ToL1Finder {
    arb_provider: DynProvider<Ethereum>,
    destinations: Vec<FinderTarget>,
}

struct FinderTarget {
    destination: Address,
    schedule_path: PathBuf,
}

impl L2ToL1Finder {
    pub fn new(arb_provider: DynProvider<Ethereum>) -> Self {
        Self {
            arb_provider,
            destinations: Vec::new(),
        }
    }

    pub fn add_target(mut self, destination: Address, schedule_path: impl Into<PathBuf>) -> Self {
        self.destinations.push(FinderTarget {
            destination,
            schedule_path: schedule_path.into(),
        });
        self
    }

    pub async fn run(&self) {
        for target in &self.destinations {
            self.run_for_target(target).await;
        }
    }

    async fn run_for_target(&self, target: &FinderTarget) {
        let schedule_file: ScheduleFile<ArbToL1Task> = ScheduleFile::new(&target.schedule_path);
        let l2_to_l1_tx_sig = alloy::primitives::keccak256(
            "L2ToL1Tx(address,address,uint256,uint256,uint256,uint256,uint256,uint256,bytes)"
        );

        loop {
            let mut schedule = schedule_file.load();
            let current_block = match self.arb_provider.get_block_number().await {
                Ok(b) => b.saturating_sub(FINALITY_BUFFER),
                Err(e) => {
                    eprintln!("[L2ToL1Finder] Failed to get block number: {}, retrying...", e);
                    sleep(RETRY_DELAY).await;
                    continue;
                }
            };

            let from_block = schedule.last_checked_block.unwrap_or_else(|| {
                let ten_days_blocks = 10 * 24 * 3600 * 1000 / 250;
                current_block.saturating_sub(ten_days_blocks)
            });

            if from_block >= current_block {
                println!("[L2ToL1Finder] Caught up to block {}, waiting...", current_block);
                sleep(POLL_INTERVAL).await;
                continue;
            }

            let to_block = min(from_block + CHUNK_SIZE, current_block);

            let filter = Filter::new()
                .address(ARB_SYS_ADDRESS)
                .event_signature(l2_to_l1_tx_sig)
                .topic1(target.destination)
                .from_block(from_block)
                .to_block(to_block);

            match self.arb_provider.get_logs(&filter).await {
                Ok(logs) => {
                    for log in logs {
                        if let Some(task) = self.parse_l2_to_l1_event(&log, target.destination) {
                            if !schedule.pending.iter().any(|t| t.position == task.position) {
                                println!(
                                    "[L2ToL1Finder] Found L2ToL1Tx: epoch={}, position={:#x}",
                                    task.epoch, task.position
                                );
                                schedule.pending.push(task);
                            }
                        }
                    }
                    schedule.last_checked_block = Some(to_block);
                    schedule_file.save(&schedule);
                    println!(
                        "[L2ToL1Finder] Scanned blocks {}-{}, {} pending tasks",
                        from_block, to_block, schedule.pending.len()
                    );
                }
                Err(e) => {
                    eprintln!(
                        "[L2ToL1Finder] Failed to query logs {}-{}: {}, retrying...",
                        from_block, to_block, e
                    );
                    sleep(RETRY_DELAY).await;
                }
            }
        }
    }

    fn parse_l2_to_l1_event(
        &self,
        log: &alloy::rpc::types::Log,
        destination: Address,
    ) -> Option<ArbToL1Task> {
        if log.topics().len() < 4 {
            return None;
        }

        let caller = Address::from_slice(&log.topics()[1].0[12..]);
        let position = U256::from_be_bytes(log.topics()[3].0);

        let data_bytes = &log.data().data;
        if data_bytes.len() < 224 {
            return None;
        }

        let arb_block_num = U256::from_be_slice(&data_bytes[0..32]);
        let eth_block_num = U256::from_be_slice(&data_bytes[32..64]);
        let timestamp = U256::from_be_slice(&data_bytes[64..96]);
        let callvalue = U256::from_be_slice(&data_bytes[96..128]);

        let data_offset = U256::from_be_slice(&data_bytes[128..160]).to::<usize>();
        let data_len = U256::from_be_slice(&data_bytes[160..192]).to::<usize>();
        let data_start = 192 + data_offset - 32;
        let data = if data_start + data_len <= data_bytes.len() {
            Bytes::copy_from_slice(&data_bytes[data_start..data_start + data_len])
        } else {
            Bytes::new()
        };

        let execute_after = timestamp.to::<u64>() + RELAY_DELAY;

        let epoch = self.extract_epoch_from_calldata(&data)?;

        Some(ArbToL1Task {
            epoch,
            position,
            execute_after,
            destination,
            caller,
            arb_block_num,
            eth_block_num,
            l2_timestamp: timestamp,
            callvalue,
            data,
        })
    }

    fn extract_epoch_from_calldata(&self, data: &Bytes) -> Option<u64> {
        if data.len() < 36 {
            return None;
        }
        let epoch = U256::from_be_slice(&data[4..36]);
        Some(epoch.to::<u64>())
    }
}
