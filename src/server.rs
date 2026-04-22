use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

const DEFAULT_SERVER_SOCKET: &str = "127.0.0.1:55123";

#[derive(Debug)]
pub enum Event {
    Quit,
}

pub struct Server {
    event_tx: mpsc::UnboundedSender<Event>,
    cancellation_token: CancellationToken,
    active_connections: Arc<AtomicU8>,
}

impl Server {
    pub fn new(
        event_tx: mpsc::UnboundedSender<Event>,
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
        log::info!("server shutting down");
        Ok(())
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

        while let Some(line) = lines.next_line().await? {
            log::debug!("[client {}] received message: {}", addr, line);
            if line.trim().eq_ignore_ascii_case("quit") {
                self.event_tx.send(Event::Quit)?;
                writer.write_all(b"goodbye!\n").await?;
                return Ok(());
            }
            writer.write_all(line.as_bytes()).await?;
            writer.write_all(b"\n").await?;
        }
        log::info!("[client {}] disconnected", addr);
        Ok(())
    }
}

pub async fn start() -> Result<()> {
    let (event_tx, event_rx) = mpsc::unbounded_channel::<Event>();
    let cancellation_token = CancellationToken::new();
    let server = Server::new(event_tx.clone(), cancellation_token.clone());

    let event_thread =
        tokio::spawn(async move { run_event_loop(event_rx, cancellation_token).await });
    let server_thread = tokio::spawn(async move { server.run().await });

    let (server_res, event_res) = tokio::join!(server_thread, event_thread);
    server_res??;
    event_res??;
    log::info!("all threads have finished");

    Ok(())
}

async fn run_event_loop(
    mut receiver: mpsc::UnboundedReceiver<Event>,
    cancellation_token: CancellationToken,
) -> Result<()> {
    log::info!("running event loop");
    loop {
        tokio::select! {
            Some(event) = receiver.recv() => {
                handle_event(event, &cancellation_token).await;
            }
            _ = cancellation_token.cancelled() => {
                    break;
            }
        }
    }
    log::info!("event loop has finished");
    Ok(())
}

async fn handle_event(event: Event, cancellation_token: &CancellationToken) {
    log::debug!("handling event: {:?}", event);
    match event {
        Event::Quit => cancellation_token.cancel(),
    }
}
