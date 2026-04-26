use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use tokio::{
    signal,
    sync::mpsc,
    task::{JoinError, JoinHandle},
    time::timeout,
};
use tokio_util::sync::CancellationToken;

use crate::{kvstore, network};

type Thread = JoinHandle<Result<()>>;
struct Application {
    cancellation_token: CancellationToken,
    server_thread: Thread,
    event_thread: Thread,
    errors: Vec<anyhow::Error>,
}

pub async fn start() -> Result<()> {
    log::info!("starting the application...");
    let cancellation_token = CancellationToken::new();
    let (server_thread, event_thread) = spawn_threads(cancellation_token.clone());

    let mut app = Application {
        cancellation_token,
        server_thread,
        event_thread,
        errors: Vec::new(),
    };
    let run_result = app.run().await;

    match &run_result {
        Ok(_) => log::info!("application shut down successfully"),
        Err(err) => log::error!("an error occurred, shutting down: {}", err),
    }
    run_result
}

fn spawn_threads(cancellation_token: CancellationToken) -> (Thread, Thread) {
    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let server = network::Server::new(event_tx, cancellation_token.clone());
    let mut kvstore = kvstore::KVStore::new(event_rx, cancellation_token.clone());
    let event_thread = tokio::spawn(async move { kvstore.run_event_loop().await });
    let server_thread = tokio::spawn(async move { server.run().await });
    (event_thread, server_thread)
}

impl Application {
    async fn run(&mut self) -> Result<()> {
        tokio::select! {
            signal_result = signal::ctrl_c() => {
                if let Err(err) = signal_result {
                    self.errors.push(anyhow!(err));
                }
                self.shutdown().await;
            }
            _ = self.cancellation_token.cancelled() => self.shutdown().await,
            server_join_res = &mut self.server_thread => self.
                handle_unexpected_thread_shutdown(server_join_res, "server").await,
            event_join_res = &mut self.event_thread => self.
                handle_unexpected_thread_shutdown(event_join_res, "event").await,
        };
        self.combine_errors()
    }

    async fn shutdown(&mut self) {
        log::info!("shutting down...");
        if !self.cancellation_token.is_cancelled() {
            self.cancellation_token.cancel();
        }
        if timeout(Duration::from_secs(1), self.wait_for_threads_to_finish())
            .await
            .is_err()
        {
            self.errors.push(anyhow!(
                "a timeout occurred while waiting for threads to finish"
            ));
        }
    }

    async fn handle_unexpected_thread_shutdown(
        &mut self,
        thread_join_res: std::result::Result<Result<()>, JoinError>,
        thread_name: &str,
    ) {
        let error = match self.ensure_thread_shutdown(thread_join_res, thread_name) {
            Ok(_) => anyhow!("[{} thread] unexpectedly finished", thread_name),
            Err(err) => anyhow!(
                "[{} thread] unexpectedly finished with error: {}",
                thread_name,
                err
            ),
        };
        self.errors.push(error);
        self.shutdown().await;
    }

    async fn wait_for_threads_to_finish(&mut self) {
        let (server_join_res, event_join_res) =
            tokio::join!(&mut self.server_thread, &mut self.event_thread);
        if let Err(err) = self.ensure_thread_shutdown(server_join_res, "server") {
            self.errors.push(anyhow!("[server thread] error: {}", err));
        }
        if let Err(err) = self.ensure_thread_shutdown(event_join_res, "event") {
            self.errors.push(anyhow!("[event thread] error: {}", err));
        }
    }

    fn ensure_thread_shutdown(
        &mut self,
        thread_join_res: std::result::Result<Result<()>, JoinError>,
        thread_name: &str,
    ) -> Result<()> {
        thread_join_res??;
        log::info!("[{} thread] shut down without errors", thread_name);
        Ok(())
    }

    fn combine_errors(&self) -> Result<()> {
        if self.errors.is_empty() {
            return Ok(());
        }
        bail!(
            "{} error(s) occurred: {}",
            self.errors.len(),
            self.errors
                .iter()
                .map(|err| err.to_string())
                .collect::<Vec<String>>()
                .join("; ")
        )
    }
}
