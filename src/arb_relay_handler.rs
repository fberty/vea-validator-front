use alloy::primitives::{Address, FixedBytes, U256};
use alloy::providers::{DynProvider, Provider};
use alloy::network::Ethereum;
use std::path::PathBuf;
use tokio::time::{sleep, Duration};

use crate::contracts::IOutbox;
use crate::scheduler::{ArbToL1Task, ScheduleFile};

const POLL_INTERVAL: Duration = Duration::from_secs(15 * 60);

pub struct ArbRelayHandler {
    eth_provider: DynProvider<Ethereum>,
    outbox_address: Address,
    schedule_path: PathBuf,
}

impl ArbRelayHandler {
    pub fn new(
        eth_provider: DynProvider<Ethereum>,
        outbox_address: Address,
        schedule_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            eth_provider,
            outbox_address,
            schedule_path: schedule_path.into(),
        }
    }

    pub async fn run(&self) {
        loop {
            self.process_pending().await;
            sleep(POLL_INTERVAL).await;
        }
    }

    pub async fn process_pending(&self) {
        let schedule_file: ScheduleFile<ArbToL1Task> = ScheduleFile::new(&self.schedule_path);
        let mut schedule = schedule_file.load();

        let now = match self.eth_provider.get_block_by_number(Default::default()).await {
            Ok(Some(block)) => block.header.timestamp,
            _ => return,
        };

        let ready: Vec<ArbToL1Task> = schedule
            .pending
            .iter()
            .filter(|t| now >= t.execute_after)
            .cloned()
            .collect();

        if ready.is_empty() {
            return;
        }

        println!("[ArbRelayHandler] Processing {} ready tasks", ready.len());

        let outbox = IOutbox::new(self.outbox_address, self.eth_provider.clone());

        for task in ready {
            match outbox.isSpent(task.position).call().await {
                Ok(is_spent) if is_spent => {
                    println!(
                        "[ArbRelayHandler] Epoch {} already relayed (position {:#x})",
                        task.epoch, task.position
                    );
                    schedule.pending.retain(|t| t.epoch != task.epoch);
                }
                Ok(_) => {
                    println!(
                        "[ArbRelayHandler] Executing relay for epoch {} (position {:#x})",
                        task.epoch, task.position
                    );

                    let empty_proof: Vec<FixedBytes<32>> = vec![];

                    match outbox
                        .executeTransaction(
                            empty_proof,
                            task.position,
                            task.l2_sender,
                            task.dest_addr,
                            U256::from(task.l2_block),
                            U256::from(task.l1_block),
                            U256::from(task.l2_timestamp),
                            task.amount,
                            task.data.clone(),
                        )
                        .send()
                        .await
                    {
                        Ok(pending) => {
                            match pending.get_receipt().await {
                                Ok(receipt) => {
                                    println!(
                                        "[ArbRelayHandler] Epoch {} relayed successfully! tx: {:?}",
                                        task.epoch, receipt.transaction_hash
                                    );
                                    schedule.pending.retain(|t| t.epoch != task.epoch);
                                }
                                Err(e) => {
                                    eprintln!(
                                        "[ArbRelayHandler] Epoch {} tx failed to confirm: {}",
                                        task.epoch, e
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "[ArbRelayHandler] Failed to execute relay for epoch {}: {}",
                                task.epoch, e
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "[ArbRelayHandler] Failed to check isSpent for epoch {}: {}",
                        task.epoch, e
                    );
                }
            }
        }

        schedule_file.save(&schedule);
    }
}
