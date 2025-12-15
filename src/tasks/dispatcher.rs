use alloy::providers::Provider;
use std::path::PathBuf;
use tokio::time::{sleep, Duration};

use crate::config::{Route, ValidatorConfig};
use crate::tasks;
use crate::tasks::{Task, TaskStore};

const POLL_INTERVAL: Duration = Duration::from_secs(15 * 60);

pub struct TaskDispatcher {
    config: ValidatorConfig,
    route: Route,
    task_store: TaskStore,
}

impl TaskDispatcher {
    pub fn new(
        config: ValidatorConfig,
        route: Route,
        schedule_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            config,
            route,
            task_store: TaskStore::new(schedule_path),
        }
    }

    pub async fn run(&self) {
        loop {
            self.process_pending().await;
            sleep(POLL_INTERVAL).await;
        }
    }

    pub async fn process_pending(&self) {
        let state = self.task_store.load();

        let now = match self.route.outbox_provider.get_block_by_number(Default::default()).await {
            Ok(Some(block)) => block.header.timestamp,
            _ => return,
        };

        let ready: Vec<Task> = state
            .tasks
            .iter()
            .filter(|t| now >= t.execute_after())
            .cloned()
            .collect();

        if ready.is_empty() {
            return;
        }

        println!("[{}][Dispatcher] Processing {} ready tasks", self.route.name, ready.len());

        for task in ready {
            let success = self.execute_task(&task, now).await;
            if success {
                self.task_store.remove_task(&task);
            }
        }
    }

    async fn execute_task(&self, task: &Task, current_timestamp: u64) -> bool {
        let wallet_address = self.config.wallet.default_signer().address();
        match task {
            Task::SaveSnapshot { epoch, .. } => {
                tasks::save_snapshot::execute(
                    self.route.inbox_provider.clone(),
                    self.route.inbox_address,
                    *epoch,
                    self.route.name,
                ).await.is_ok()
            }
            Task::Claim { epoch, .. } => {
                tasks::claim::execute(
                    self.route.inbox_provider.clone(),
                    self.route.inbox_address,
                    self.route.outbox_provider.clone(),
                    self.route.outbox_address,
                    *epoch,
                    self.route.name,
                ).await.is_ok()
            }
            Task::VerifyClaim { epoch, state_root, claimer, timestamp_claimed, .. } => {
                tasks::verify_claim::execute(
                    &self.route,
                    wallet_address,
                    *epoch,
                    *state_root,
                    *claimer,
                    *timestamp_claimed,
                    current_timestamp,
                    &self.task_store,
                ).await.is_ok()
            }
            Task::Challenge { epoch, state_root, claimer, timestamp_claimed, .. } => {
                tasks::challenge::execute(
                    self.route.outbox_provider.clone(),
                    self.route.outbox_address,
                    self.route.weth_address,
                    wallet_address,
                    *epoch,
                    *state_root,
                    *claimer,
                    *timestamp_claimed,
                    self.route.name,
                ).await.is_ok()
            }
            Task::SendSnapshot { epoch, state_root, claimer, timestamp_claimed, challenger, .. } => {
                tasks::send_snapshot::execute(
                    self.route.inbox_provider.clone(),
                    self.route.inbox_address,
                    self.route.weth_address,
                    *epoch,
                    *state_root,
                    *claimer,
                    *timestamp_claimed,
                    *challenger,
                    self.route.name,
                ).await.is_ok()
            }
            Task::StartVerification { epoch, state_root, claimer, timestamp_claimed, .. } => {
                tasks::start_verification::execute(
                    self.route.outbox_provider.clone(),
                    self.route.outbox_address,
                    *epoch,
                    *state_root,
                    *claimer,
                    *timestamp_claimed,
                    self.route.name,
                ).await.is_ok()
            }
            Task::VerifySnapshot { epoch, state_root, claimer, timestamp_claimed, timestamp_verification, blocknumber_verification, .. } => {
                tasks::verify_snapshot::execute(
                    self.route.outbox_provider.clone(),
                    self.route.outbox_address,
                    *epoch,
                    *state_root,
                    *claimer,
                    *timestamp_claimed,
                    *timestamp_verification,
                    *blocknumber_verification,
                    self.route.name,
                ).await.is_ok()
            }
            Task::ExecuteRelay { position, l2_sender, dest_addr, l2_block, l1_block, l2_timestamp, amount, data, .. } => {
                tasks::execute_relay::execute(
                    self.route.inbox_provider.clone(),
                    self.route.outbox_provider.clone(),
                    self.config.arb_outbox,
                    *position,
                    *l2_sender,
                    *dest_addr,
                    *l2_block,
                    *l1_block,
                    *l2_timestamp,
                    *amount,
                    data.clone(),
                    self.route.name,
                ).await.is_ok()
            }
        }
    }
}
