use alloy::primitives::{address, Address, FixedBytes};
use alloy::providers::DynProvider;
use alloy::network::Ethereum;
use std::path::PathBuf;
use tokio::time::{sleep, Duration};

use crate::contracts::{IArbSys, INodeInterface, IOutbox};
use crate::scheduler::{ArbToL1Task, ScheduleFile};

const ARB_SYS_ADDRESS: Address = address!("0000000000000000000000000000000000000064");
const NODE_INTERFACE_ADDRESS: Address = address!("00000000000000000000000000000000000000C8");
const POLL_INTERVAL: Duration = Duration::from_secs(15 * 60);

pub struct ArbRelayHandler {
    eth_provider: DynProvider<Ethereum>,
    arb_provider: DynProvider<Ethereum>,
    outbox_address: Address,
    schedule_path: PathBuf,
}

impl ArbRelayHandler {
    pub fn new(
        eth_provider: DynProvider<Ethereum>,
        arb_provider: DynProvider<Ethereum>,
        outbox_address: Address,
        schedule_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            eth_provider,
            arb_provider,
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

        println!("[ArbRelayHandler] {} tasks ready for relay", ready.len());

        let outbox = IOutbox::new(self.outbox_address, self.eth_provider.clone());

        for task in ready {
            schedule.pending.retain(|t| t.position != task.position);

            match outbox.isSpent(task.position).call().await {
                Ok(is_spent) if is_spent => {
                    println!(
                        "[ArbRelayHandler] Epoch {} already relayed, skipping",
                        task.epoch
                    );
                    continue;
                }
                Ok(_) => {}
                Err(e) => {
                    eprintln!(
                        "[ArbRelayHandler] Failed to check isSpent for epoch {}: {}",
                        task.epoch, e
                    );
                    continue;
                }
            }

            match self.execute_relay(&task).await {
                Ok(()) => {
                    println!(
                        "[ArbRelayHandler] Successfully relayed epoch {}",
                        task.epoch
                    );
                }
                Err(e) => {
                    eprintln!(
                        "[ArbRelayHandler] Failed to relay epoch {}: {}",
                        task.epoch, e
                    );
                }
            }
        }

        schedule_file.save(&schedule);
    }

    async fn execute_relay(
        &self,
        task: &ArbToL1Task,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "[ArbRelayHandler] Executing relay for epoch {}, position {:#x}",
            task.epoch, task.position
        );

        let arb_sys = IArbSys::new(ARB_SYS_ADDRESS, self.arb_provider.clone());
        let merkle_state = arb_sys.sendMerkleTreeState().call().await?;
        let size: u64 = merkle_state.size.try_into().expect("size should fit in u64");

        let node_interface =
            INodeInterface::new(NODE_INTERFACE_ADDRESS, self.arb_provider.clone());
        let proof_result = node_interface
            .constructOutboxProof(size, task.position.to::<u64>())
            .call()
            .await?;

        let proof: Vec<FixedBytes<32>> = proof_result.proof;

        let outbox = IOutbox::new(self.outbox_address, self.eth_provider.clone());
        let tx = outbox.executeTransaction(
            proof,
            task.position,
            task.caller,
            task.destination,
            task.arb_block_num,
            task.eth_block_num,
            task.l2_timestamp,
            task.callvalue,
            task.data.clone(),
        );

        let receipt = tx.send().await?.get_receipt().await?;

        if !receipt.status() {
            return Err("executeTransaction reverted".into());
        }

        Ok(())
    }
}
