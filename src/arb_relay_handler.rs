use alloy::primitives::Address;
use alloy::providers::DynProvider;
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

    async fn process_pending(&self) {
        let schedule_file: ScheduleFile<ArbToL1Task> = ScheduleFile::new(&self.schedule_path);
        let mut schedule = schedule_file.load();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let ready: Vec<ArbToL1Task> = schedule
            .pending
            .iter()
            .filter(|t| now >= t.execute_after)
            .cloned()
            .collect();

        if ready.is_empty() {
            return;
        }

        println!("[ArbRelayHandler] Checking {} tasks for relay status", ready.len());

        let outbox = IOutbox::new(self.outbox_address, self.eth_provider.clone());

        for task in ready {
            match outbox.isSpent(task.position).call().await {
                Ok(is_spent) if is_spent => {
                    println!(
                        "[ArbRelayHandler] Epoch {} successfully relayed (position {:#x})",
                        task.epoch, task.position
                    );
                    schedule.pending.retain(|t| t.epoch != task.epoch);
                }
                Ok(_) => {
                    eprintln!(
                        "[ArbRelayHandler] WARNING: Epoch {} NOT relayed after delay! (position {:#x})",
                        task.epoch, task.position
                    );
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
