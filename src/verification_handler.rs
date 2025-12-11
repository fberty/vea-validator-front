use alloy::primitives::{Address, U256};
use alloy::providers::{DynProvider, Provider};
use alloy::network::Ethereum;
use std::path::PathBuf;
use tokio::time::{sleep, Duration};

use crate::contracts::{IVeaOutboxArbToEth, IVeaOutboxArbToGnosis, Claim, Party};
use crate::scheduler::{ScheduleFile, VerificationTask, VerificationPhase};

const POLL_INTERVAL: Duration = Duration::from_secs(15 * 60);

pub struct VerificationHandler {
    outbox_provider: DynProvider<Ethereum>,
    outbox_address: Address,
    weth_address: Option<Address>,
    schedule_path: PathBuf,
    route_name: &'static str,
}

impl VerificationHandler {
    pub fn new(
        outbox_provider: DynProvider<Ethereum>,
        outbox_address: Address,
        weth_address: Option<Address>,
        schedule_path: impl Into<PathBuf>,
        route_name: &'static str,
    ) -> Self {
        Self {
            outbox_provider,
            outbox_address,
            weth_address,
            schedule_path: schedule_path.into(),
            route_name,
        }
    }

    pub async fn run(&self) {
        loop {
            self.process_pending().await;
            sleep(POLL_INTERVAL).await;
        }
    }

    pub async fn process_pending(&self) {
        let schedule_file: ScheduleFile<VerificationTask> = ScheduleFile::new(&self.schedule_path);
        let mut schedule = schedule_file.load();

        let now = match self.outbox_provider.get_block_by_number(Default::default()).await {
            Ok(Some(block)) => block.header.timestamp,
            _ => return,
        };

        let ready: Vec<VerificationTask> = schedule
            .pending
            .iter()
            .filter(|t| now >= t.execute_after)
            .cloned()
            .collect();

        if ready.is_empty() {
            return;
        }

        println!("[{}][VerificationHandler] Processing {} ready tasks", self.route_name, ready.len());

        for task in ready {
            let claim = Claim {
                stateRoot: task.state_root,
                claimer: task.claimer,
                timestampClaimed: task.timestamp_claimed,
                timestampVerification: task.timestamp_verification,
                blocknumberVerification: task.blocknumber_verification,
                honest: Party::None,
                challenger: Address::ZERO,
            };

            match task.phase {
                VerificationPhase::StartVerification => {
                    if self.call_start_verification(task.epoch, claim).await {
                        schedule.pending.retain(|t| t.epoch != task.epoch);
                    }
                }
                VerificationPhase::VerifySnapshot => {
                    if self.call_verify_snapshot(task.epoch, claim).await {
                        schedule.pending.retain(|t| t.epoch != task.epoch);
                    }
                }
            }
        }

        schedule_file.save(&schedule);
    }

    async fn call_start_verification(&self, epoch: u64, claim: Claim) -> bool {
        println!("[{}][VerificationHandler] Calling startVerification for epoch {}", self.route_name, epoch);

        if self.weth_address.is_some() {
            let outbox = IVeaOutboxArbToGnosis::new(self.outbox_address, self.outbox_provider.clone());
            match outbox.startVerification(U256::from(epoch), claim).send().await {
                Ok(pending) => match pending.get_receipt().await {
                    Ok(receipt) => {
                        if receipt.status() {
                            println!("[{}][VerificationHandler] startVerification succeeded for epoch {}", self.route_name, epoch);
                            true
                        } else {
                            eprintln!("[{}][VerificationHandler] startVerification reverted for epoch {}", self.route_name, epoch);
                            false
                        }
                    }
                    Err(e) => {
                        eprintln!("[{}][VerificationHandler] Failed to get receipt for epoch {}: {}", self.route_name, epoch, e);
                        false
                    }
                },
                Err(e) => {
                    let err_msg = e.to_string();
                    if err_msg.contains("Verification already started") || err_msg.contains("already") {
                        println!("[{}][VerificationHandler] startVerification already done for epoch {}", self.route_name, epoch);
                        return true;
                    }
                    eprintln!("[{}][VerificationHandler] startVerification failed for epoch {}: {}", self.route_name, epoch, e);
                    false
                }
            }
        } else {
            let outbox = IVeaOutboxArbToEth::new(self.outbox_address, self.outbox_provider.clone());
            match outbox.startVerification(U256::from(epoch), claim).send().await {
                Ok(pending) => match pending.get_receipt().await {
                    Ok(receipt) => {
                        if receipt.status() {
                            println!("[{}][VerificationHandler] startVerification succeeded for epoch {}", self.route_name, epoch);
                            true
                        } else {
                            eprintln!("[{}][VerificationHandler] startVerification reverted for epoch {}", self.route_name, epoch);
                            false
                        }
                    }
                    Err(e) => {
                        eprintln!("[{}][VerificationHandler] Failed to get receipt for epoch {}: {}", self.route_name, epoch, e);
                        false
                    }
                },
                Err(e) => {
                    let err_msg = e.to_string();
                    if err_msg.contains("Verification already started") || err_msg.contains("already") {
                        println!("[{}][VerificationHandler] startVerification already done for epoch {}", self.route_name, epoch);
                        return true;
                    }
                    eprintln!("[{}][VerificationHandler] startVerification failed for epoch {}: {}", self.route_name, epoch, e);
                    false
                }
            }
        }
    }

    async fn call_verify_snapshot(&self, epoch: u64, claim: Claim) -> bool {
        println!("[{}][VerificationHandler] Calling verifySnapshot for epoch {}", self.route_name, epoch);

        if self.weth_address.is_some() {
            let outbox = IVeaOutboxArbToGnosis::new(self.outbox_address, self.outbox_provider.clone());
            match outbox.verifySnapshot(U256::from(epoch), claim).send().await {
                Ok(pending) => match pending.get_receipt().await {
                    Ok(receipt) => {
                        if receipt.status() {
                            println!("[{}][VerificationHandler] verifySnapshot succeeded for epoch {}", self.route_name, epoch);
                            true
                        } else {
                            eprintln!("[{}][VerificationHandler] verifySnapshot reverted for epoch {}", self.route_name, epoch);
                            false
                        }
                    }
                    Err(e) => {
                        eprintln!("[{}][VerificationHandler] Failed to get receipt for epoch {}: {}", self.route_name, epoch, e);
                        false
                    }
                },
                Err(e) => {
                    let err_msg = e.to_string();
                    if err_msg.contains("Already verified") || err_msg.contains("already") {
                        println!("[{}][VerificationHandler] verifySnapshot already done for epoch {}", self.route_name, epoch);
                        return true;
                    }
                    eprintln!("[{}][VerificationHandler] verifySnapshot failed for epoch {}: {}", self.route_name, epoch, e);
                    false
                }
            }
        } else {
            let outbox = IVeaOutboxArbToEth::new(self.outbox_address, self.outbox_provider.clone());
            match outbox.verifySnapshot(U256::from(epoch), claim).send().await {
                Ok(pending) => match pending.get_receipt().await {
                    Ok(receipt) => {
                        if receipt.status() {
                            println!("[{}][VerificationHandler] verifySnapshot succeeded for epoch {}", self.route_name, epoch);
                            true
                        } else {
                            eprintln!("[{}][VerificationHandler] verifySnapshot reverted for epoch {}", self.route_name, epoch);
                            false
                        }
                    }
                    Err(e) => {
                        eprintln!("[{}][VerificationHandler] Failed to get receipt for epoch {}: {}", self.route_name, epoch, e);
                        false
                    }
                },
                Err(e) => {
                    let err_msg = e.to_string();
                    if err_msg.contains("Already verified") || err_msg.contains("already") {
                        println!("[{}][VerificationHandler] verifySnapshot already done for epoch {}", self.route_name, epoch);
                        return true;
                    }
                    eprintln!("[{}][VerificationHandler] verifySnapshot failed for epoch {}: {}", self.route_name, epoch, e);
                    false
                }
            }
        }
    }
}
