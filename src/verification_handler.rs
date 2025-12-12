use alloy::primitives::{Address, U256};
use alloy::providers::{DynProvider, Provider};
use alloy::network::Ethereum;
use std::path::PathBuf;
use tokio::time::{sleep, Duration};

use crate::contracts::{IVeaOutboxArbToEth, IVeaOutboxArbToGnosis, IVeaInboxArbToEth, IVeaInboxArbToGnosis, Claim, Party};
use crate::scheduler::{ScheduleFile, VerificationTask, VerificationPhase};

const POLL_INTERVAL: Duration = Duration::from_secs(15 * 60);

pub struct VerificationHandler {
    inbox_provider: DynProvider<Ethereum>,
    inbox_address: Address,
    outbox_provider: DynProvider<Ethereum>,
    outbox_address: Address,
    weth_address: Option<Address>,
    wallet_address: Address,
    schedule_path: PathBuf,
    route_name: &'static str,
}

impl VerificationHandler {
    pub fn new(
        inbox_provider: DynProvider<Ethereum>,
        inbox_address: Address,
        outbox_provider: DynProvider<Ethereum>,
        outbox_address: Address,
        weth_address: Option<Address>,
        wallet_address: Address,
        schedule_path: impl Into<PathBuf>,
        route_name: &'static str,
    ) -> Self {
        Self {
            inbox_provider,
            inbox_address,
            outbox_provider,
            outbox_address,
            weth_address,
            wallet_address,
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
                challenger: task.challenger,
            };

            match task.phase {
                VerificationPhase::Challenge => {
                    if self.call_challenge(task.epoch, claim).await {
                        schedule.pending.retain(|t| !(t.epoch == task.epoch && matches!(t.phase, VerificationPhase::Challenge)));
                    }
                }
                VerificationPhase::SendSnapshot => {
                    if self.call_send_snapshot(task.epoch, claim).await {
                        schedule.pending.retain(|t| !(t.epoch == task.epoch && matches!(t.phase, VerificationPhase::SendSnapshot)));
                    }
                }
                VerificationPhase::StartVerification => {
                    if self.call_start_verification(task.epoch, claim).await {
                        schedule.pending.retain(|t| !(t.epoch == task.epoch && matches!(t.phase, VerificationPhase::StartVerification)));
                    }
                }
                VerificationPhase::VerifySnapshot => {
                    if self.call_verify_snapshot(task.epoch, claim).await {
                        schedule.pending.retain(|t| !(t.epoch == task.epoch && matches!(t.phase, VerificationPhase::VerifySnapshot)));
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

    async fn call_challenge(&self, epoch: u64, claim: Claim) -> bool {
        println!("[{}][VerificationHandler] Calling challenge for epoch {}", self.route_name, epoch);

        if self.weth_address.is_some() {
            let outbox = IVeaOutboxArbToGnosis::new(self.outbox_address, self.outbox_provider.clone());
            match outbox.challenge(U256::from(epoch), claim).send().await {
                Ok(pending) => match pending.get_receipt().await {
                    Ok(receipt) => {
                        if receipt.status() {
                            println!("[{}][VerificationHandler] challenge succeeded for epoch {}", self.route_name, epoch);
                            true
                        } else {
                            panic!("[{}][VerificationHandler] FATAL: challenge reverted for epoch {}", self.route_name, epoch);
                        }
                    }
                    Err(e) => {
                        panic!("[{}][VerificationHandler] FATAL: challenge receipt failed for epoch {}: {}", self.route_name, epoch, e);
                    }
                },
                Err(e) => {
                    let err_msg = e.to_string();
                    if err_msg.contains("Invalid claim") || err_msg.contains("already") {
                        println!("[{}][VerificationHandler] Claim already challenged - bridge is safe", self.route_name);
                        return true;
                    }
                    panic!("[{}][VerificationHandler] FATAL: challenge failed for epoch {}: {}", self.route_name, epoch, e);
                }
            }
        } else {
            let outbox = IVeaOutboxArbToEth::new(self.outbox_address, self.outbox_provider.clone());
            let deposit = match outbox.deposit().call().await {
                Ok(d) => d,
                Err(e) => panic!("[{}][VerificationHandler] FATAL: Failed to get deposit for challenge: {}", self.route_name, e),
            };
            match outbox.challenge(U256::from(epoch), claim, self.wallet_address).value(deposit).send().await {
                Ok(pending) => match pending.get_receipt().await {
                    Ok(receipt) => {
                        if receipt.status() {
                            println!("[{}][VerificationHandler] challenge succeeded for epoch {}", self.route_name, epoch);
                            true
                        } else {
                            panic!("[{}][VerificationHandler] FATAL: challenge reverted for epoch {}", self.route_name, epoch);
                        }
                    }
                    Err(e) => {
                        panic!("[{}][VerificationHandler] FATAL: challenge receipt failed for epoch {}: {}", self.route_name, epoch, e);
                    }
                },
                Err(e) => {
                    let err_msg = e.to_string();
                    if err_msg.contains("Invalid claim") || err_msg.contains("already") {
                        println!("[{}][VerificationHandler] Claim already challenged - bridge is safe", self.route_name);
                        return true;
                    }
                    panic!("[{}][VerificationHandler] FATAL: challenge failed for epoch {}: {}", self.route_name, epoch, e);
                }
            }
        }
    }

    async fn call_send_snapshot(&self, epoch: u64, claim: Claim) -> bool {
        println!("[{}][VerificationHandler] Calling sendSnapshot for epoch {}", self.route_name, epoch);

        if self.weth_address.is_some() {
            let inbox = IVeaInboxArbToGnosis::new(self.inbox_address, self.inbox_provider.clone());
            let gas_limit = U256::from(500000);
            match inbox.sendSnapshot(U256::from(epoch), gas_limit, claim).send().await {
                Ok(pending) => match pending.get_receipt().await {
                    Ok(receipt) => {
                        if receipt.status() {
                            println!("[{}][VerificationHandler] sendSnapshot succeeded for epoch {}", self.route_name, epoch);
                            true
                        } else {
                            panic!("[{}][VerificationHandler] FATAL: sendSnapshot reverted for epoch {}", self.route_name, epoch);
                        }
                    }
                    Err(e) => {
                        panic!("[{}][VerificationHandler] FATAL: sendSnapshot receipt failed for epoch {}: {}", self.route_name, epoch, e);
                    }
                },
                Err(e) => {
                    panic!("[{}][VerificationHandler] FATAL: sendSnapshot failed for epoch {}: {}", self.route_name, epoch, e);
                }
            }
        } else {
            let inbox = IVeaInboxArbToEth::new(self.inbox_address, self.inbox_provider.clone());
            match inbox.sendSnapshot(U256::from(epoch), claim).send().await {
                Ok(pending) => match pending.get_receipt().await {
                    Ok(receipt) => {
                        if receipt.status() {
                            println!("[{}][VerificationHandler] sendSnapshot succeeded for epoch {}", self.route_name, epoch);
                            true
                        } else {
                            panic!("[{}][VerificationHandler] FATAL: sendSnapshot reverted for epoch {}", self.route_name, epoch);
                        }
                    }
                    Err(e) => {
                        panic!("[{}][VerificationHandler] FATAL: sendSnapshot receipt failed for epoch {}: {}", self.route_name, epoch, e);
                    }
                },
                Err(e) => {
                    panic!("[{}][VerificationHandler] FATAL: sendSnapshot failed for epoch {}: {}", self.route_name, epoch, e);
                }
            }
        }
    }
}
