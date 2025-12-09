use alloy::primitives::{Address, Bytes, FixedBytes, U256};
use alloy::providers::{DynProvider, Provider};
use alloy::network::Ethereum;
use alloy::rpc::types::Filter;
use std::path::PathBuf;
use std::cmp::min;
use tokio::time::{sleep, Duration};

use crate::scheduler::{ArbToL1Task, ScheduleFile};

const CHUNK_SIZE: u64 = 500;
const FINALITY_BUFFER: u64 = 20;
const POLL_INTERVAL: Duration = Duration::from_secs(15 * 60);
const RETRY_DELAY: Duration = Duration::from_secs(5);
const RELAY_DELAY: u64 = 7 * 24 * 3600;
const ARB_SYS: Address = Address::new([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x64]);

pub struct L2ToL1Finder {
    arb_provider: DynProvider<Ethereum>,
    targets: Vec<FinderTarget>,
}

struct FinderTarget {
    inbox_address: Address,
    schedule_path: PathBuf,
}

impl L2ToL1Finder {
    pub fn new(arb_provider: DynProvider<Ethereum>) -> Self {
        Self {
            arb_provider,
            targets: Vec::new(),
        }
    }

    pub fn add_inbox(mut self, inbox_address: Address, schedule_path: impl Into<PathBuf>) -> Self {
        self.targets.push(FinderTarget {
            inbox_address,
            schedule_path: schedule_path.into(),
        });
        self
    }

    pub async fn run(&self) {
        for target in &self.targets {
            self.run_for_target(target).await;
        }
    }

    async fn run_for_target(&self, target: &FinderTarget) {
        let schedule_file: ScheduleFile<ArbToL1Task> = ScheduleFile::new(&target.schedule_path);
        let snapshot_sent_sig = alloy::primitives::keccak256("SnapshotSent(uint256,bytes32)");

        loop {
            let mut schedule = schedule_file.load();
            let raw_block = match self.arb_provider.get_block_number().await {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("[L2ToL1Finder] Failed to get block number: {}, retrying...", e);
                    sleep(RETRY_DELAY).await;
                    continue;
                }
            };

            let current_block = if raw_block > FINALITY_BUFFER {
                raw_block - FINALITY_BUFFER
            } else {
                raw_block
            };

            let from_block = schedule.last_checked_block.unwrap_or(0);

            if from_block >= current_block {
                println!("[L2ToL1Finder] Caught up to block {}, waiting...", current_block);
                sleep(POLL_INTERVAL).await;
                continue;
            }

            let to_block = min(from_block + CHUNK_SIZE, current_block);

            let filter = Filter::new()
                .address(target.inbox_address)
                .event_signature(snapshot_sent_sig)
                .from_block(from_block)
                .to_block(to_block);

            match self.arb_provider.get_logs(&filter).await {
                Ok(logs) => {
                    for log in logs {
                        let epoch = self.parse_epoch_from_snapshot_sent(&log);
                        if epoch.is_none() {
                            continue;
                        }
                        let epoch = epoch.unwrap();

                        if schedule.pending.iter().any(|t| t.epoch == epoch) {
                            continue;
                        }

                        let tx_hash = match log.transaction_hash {
                            Some(h) => h,
                            None => continue,
                        };

                        let task = match self.fetch_l2_to_l1_from_tx(tx_hash, epoch).await {
                            Some(t) => t,
                            None => {
                                eprintln!("[L2ToL1Finder] No L2ToL1Tx found in tx {:?}", tx_hash);
                                continue;
                            }
                        };

                        println!(
                            "[L2ToL1Finder] Found SnapshotSent: epoch={}, position={:#x}",
                            task.epoch, task.position
                        );
                        schedule.pending.push(task);
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

    fn parse_epoch_from_snapshot_sent(&self, log: &alloy::rpc::types::Log) -> Option<u64> {
        if log.topics().len() < 2 {
            return None;
        }
        Some(U256::from_be_bytes(log.topics()[1].0).to::<u64>())
    }

    async fn fetch_l2_to_l1_from_tx(
        &self,
        tx_hash: FixedBytes<32>,
        epoch: u64,
    ) -> Option<ArbToL1Task> {
        let receipt = self.arb_provider.get_transaction_receipt(tx_hash).await.ok()??;

        let l2_to_l1_tx_sig = alloy::primitives::keccak256(
            "L2ToL1Tx(address,address,uint256,uint256,uint256,uint256,uint256,uint256,bytes)"
        );

        for log in receipt.inner.logs() {
            if log.address() != ARB_SYS {
                continue;
            }
            if log.topics().first() != Some(&l2_to_l1_tx_sig) {
                continue;
            }
            if log.topics().len() < 4 {
                continue;
            }

            let caller = Address::from_slice(&log.topics()[1].0[12..]);
            let destination = Address::from_slice(&log.topics()[2].0[12..]);

            let data = &log.data().data;
            if data.len() < 192 {
                continue;
            }

            let position = U256::from_be_slice(&data[0..32]);
            let arb_block_num = U256::from_be_slice(&data[32..64]).to::<u64>();
            let eth_block_num = U256::from_be_slice(&data[64..96]).to::<u64>();
            let timestamp = U256::from_be_slice(&data[96..128]).to::<u64>();
            let callvalue = U256::from_be_slice(&data[128..160]);

            let data_offset = U256::from_be_slice(&data[160..192]).to::<usize>();
            let calldata = if data.len() > data_offset + 32 {
                let data_len = U256::from_be_slice(&data[data_offset..data_offset + 32]).to::<usize>();
                if data.len() >= data_offset + 32 + data_len {
                    Bytes::copy_from_slice(&data[data_offset + 32..data_offset + 32 + data_len])
                } else {
                    Bytes::new()
                }
            } else {
                Bytes::new()
            };

            let block_number = receipt.block_number.unwrap_or(0);
            let block_timestamp = match self.arb_provider.get_block_by_number(block_number.into()).await {
                Ok(Some(block)) => block.header.timestamp,
                _ => 0,
            };

            return Some(ArbToL1Task {
                epoch,
                position,
                execute_after: block_timestamp + RELAY_DELAY,
                l2_sender: caller,
                dest_addr: destination,
                l2_block: arb_block_num,
                l1_block: eth_block_num,
                l2_timestamp: timestamp,
                amount: callvalue,
                data: calldata,
            });
        }
        None
    }
}
