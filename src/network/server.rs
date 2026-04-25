use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};
use tokio_util::sync::CancellationToken;

use crate::kvstore::{self};
use crate::network::connection::Connection;

const DEFAULT_SERVER_SOCKET: &str = "127.0.0.1:55123";

pub struct Server {
    event_tx: mpsc::UnboundedSender<kvstore::Event>,
    cancellation_token: CancellationToken,
    active_connections: Arc<AtomicU8>,
}

impl Server {
    pub fn new(
        event_tx: mpsc::UnboundedSender<kvstore::Event>,
        cancellation_token: CancellationToken,
    ) -> Arc<Self> {
        Arc::new(Server {
            event_tx,
            cancellation_token,
            active_connections: Arc::new(AtomicU8::new(0)),
        })
    }

    pub async fn run(self: Arc<Self>) -> Result<()> {
        let listener = TcpListener::bind(DEFAULT_SERVER_SOCKET).await?;
        log::info!(
            "server started listening on port {}",
            listener.local_addr()?
        );
        loop {
            tokio::select! {
                connection = listener.accept() => {
                    let this = self.clone();
                    this.spawn_new_connection(connection?).await;
                }
                _ = self.cancellation_token.cancelled() => {
                    break;
                }
            }
        }
        if self.active_connections.load(Ordering::SeqCst) == 0 {
            log::info!("server shutting down, no clients connected");
            return Ok(());
        }

        log::info!("server shutting down, waiting for connections to close");
        timeout(
            Duration::from_millis(500),
            self.wait_for_connections_to_close(),
        )
        .await
        .context("a timeout occurred while waiting for connections to close")
    }

    async fn spawn_new_connection(self: Arc<Self>, connection: (TcpStream, SocketAddr)) {
        let (stream, addr) = connection;
        tokio::spawn(async move {
            self.active_connections.fetch_add(1, Ordering::SeqCst);
            Connection::run(
                stream,
                addr,
                self.cancellation_token.clone(),
                self.event_tx.clone(),
            )
            .await;
            self.active_connections.fetch_sub(1, Ordering::SeqCst);
        });
    }

    async fn wait_for_connections_to_close(&self) {
        while self.active_connections.load(Ordering::SeqCst) != 0 {
            sleep(Duration::from_millis(50)).await;
        }
    }
}
