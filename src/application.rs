use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use tokio::{
    signal,
    sync::mpsc,
    task::{self, JoinError, JoinSet},
    time::timeout,
};
use tokio_util::sync::CancellationToken;

use crate::{kvstore, network};

type ApplicationThreads = JoinSet<Result<()>>;
type JoinResult = std::result::Result<(task::Id, Result<()>), JoinError>;

struct Application {
    cancellation_token: CancellationToken,
    threads: ApplicationThreads,
    event_thread: task::Id,
    server_thread: task::Id,
    errors: Vec<anyhow::Error>,
}

pub async fn start() -> Result<()> {
    log::info!("[application] starting...");

    let mut app = Application::initialise();
    let run_result = app.run().await;

    match &run_result {
        Ok(_) => log::info!("[application] shut down successfully"),
        Err(err) => log::error!("[application] {}", err),
    }
    run_result
}

impl Application {
    fn initialise() -> Self {
        let ctok = CancellationToken::new();
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let server = network::Server::new(event_tx, ctok.clone());
        let mut kvstore = kvstore::KVStore::new(event_rx, ctok.clone());

        let mut threads = JoinSet::new();
        let event_thread = threads
            .spawn(async move { kvstore.run_event_loop().await })
            .id();
        let server_thread = threads.spawn(async move { server.run().await }).id();

        Self {
            cancellation_token: ctok,
            threads,
            event_thread,
            server_thread,
            errors: Vec::new(),
        }
    }

    async fn run(&mut self) -> Result<()> {
        let cancellation_token = self.cancellation_token.clone();
        tokio::select! {
            signal_result = signal::ctrl_c() => {
                if let Err(err) = signal_result {
                    self.errors.push(anyhow!(err));
                }
            }
            _ = cancellation_token.cancelled() => {},
            _ = self.monitor_threads() => {
            }
        }
        self.shutdown().await;
        self.combine_errors()
    }

    async fn monitor_threads(&mut self) {
        if let Some(thread_join_res) = self.threads.join_next_with_id().await {
            self.handle_unexpected_thread_shutdown(thread_join_res);
        }
    }

    async fn shutdown(&mut self) {
        log::info!("[application] shutting down...");
        if !self.cancellation_token.is_cancelled() {
            self.cancellation_token.cancel();
        }
        if timeout(Duration::from_secs(1), self.wait_for_threads_to_finish())
            .await
            .is_err()
        {
            self.threads.abort_all();
            self.errors.push(anyhow!(
                "[application] a timeout occurred while waiting for threads to finish"
            ));
        }
    }

    async fn wait_for_threads_to_finish(&mut self) {
        while let Some(thread_join_res) = self.threads.join_next_with_id().await {
            match self.handle_thread_join_result(thread_join_res) {
                Ok(id) => log::info!(
                    "[thread {}] exited without errors",
                    self.get_thread_name(id)
                ),
                Err(err) => self.errors.push(err),
            }
        }
    }

    fn handle_unexpected_thread_shutdown(&mut self, thread_join_res: JoinResult) {
        let err = match self.handle_thread_join_result(thread_join_res) {
            Ok(id) => anyhow!(
                "[{} thread]: quit unexpectedly without errors",
                self.get_thread_name(id)
            ),
            Err(err) => err,
        };
        self.errors.push(err);
    }

    fn handle_thread_join_result(&mut self, thread_join_res: JoinResult) -> Result<task::Id> {
        match thread_join_res {
            Ok((id, Ok(_))) => Ok(id),
            Ok((id, Err(thread_err))) => bail!(
                "[{} thread]: quit unexpectedly with error: {}",
                self.get_thread_name(id),
                thread_err
            ),
            Err(join_err) => bail!(
                "[{} thread]: join error: {}",
                self.get_thread_name(join_err.id()),
                join_err
            ),
        }
    }

    fn get_thread_name(&self, thread_id: task::Id) -> &'static str {
        if thread_id == self.server_thread {
            "server"
        } else if thread_id == self.event_thread {
            "event"
        } else {
            "unknown"
        }
    }

    fn combine_errors(&self) -> Result<()> {
        if self.errors.is_empty() {
            return Ok(());
        }
        bail!(
            "{} error(s) occurred:\n{}",
            self.errors.len(),
            self.errors
                .iter()
                .map(|err| err.to_string())
                .collect::<Vec<String>>()
                .join("\n")
        )
    }
}
