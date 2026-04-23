mod protocol;

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, timeout};
use tokio_util::sync::CancellationToken;

use crate::kvstore;

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
                    this.handle_connection(connection?).await;
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
    async fn handle_connection(self: Arc<Self>, connection: (TcpStream, SocketAddr)) {
        let (stream, addr) = connection;

        tokio::spawn(async move {
            self.active_connections.fetch_add(1, Ordering::SeqCst);
            if let Err(err) = self.handle_client(stream, addr).await {
                log::error!("[client {}] an error occurred: {}", addr, err);
            }
            self.active_connections.fetch_sub(1, Ordering::SeqCst);
        });
    }

    async fn handle_client(&self, mut stream: TcpStream, addr: SocketAddr) -> Result<()> {
        log::info!("[client {}] new connection", addr);
        let (reader, mut writer) = stream.split();
        let bufreader = BufReader::new(reader);
        let mut lines = bufreader.lines();

        loop {
            tokio::select! {
                line_result = lines.next_line() => {
                    match line_result? {
                        Some(line) => {
                            log::debug!("[client {}] sent message: {}", addr, line);
                            match self.handle_client_request(&line).await {
                                Ok(reply) => writer.write_all(reply.as_bytes()).await?,
                                Err(err) => writer.write_all(err.to_string().as_bytes()).await?,
                            };
                            writer.write_all(b"\n").await?;
                        }
                        None => {
                            log::info!("[client {}] disconnected", addr);
                        }
                    }
                }
                _ = self.cancellation_token.cancelled() => {
                    log::info!("[client {}] closing connection", addr);
                    writer.write_all(b"server shutting down, closing connection\n").await?;
                    break;
                }
            }
        }
        Ok(())
    }

    async fn handle_client_request(&self, line: &str) -> Result<String> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let command = kvstore::Commands::try_parse_from(parts)?;
        let (sender, receiver) = oneshot::channel();

        self.event_tx.send(kvstore::Event {
            reply_channel: sender,
            command,
        })?;

        timeout(Duration::from_millis(500), receiver).await??
    }

    async fn wait_for_connections_to_close(&self) {
        while self.active_connections.load(Ordering::SeqCst) != 0 {
            sleep(Duration::from_millis(50)).await;
        }
    }
}
