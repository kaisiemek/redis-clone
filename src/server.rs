use std::net::SocketAddr;

use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

const DEFAULT_SERVER_SOCKET: &str = "127.0.0.1:55123";

#[derive(Debug)]
enum Event {
    NewConnection { stream: TcpStream, addr: SocketAddr },
    Quit,
}

pub async fn start() -> Result<()> {
    let (event_tx, event_rx) = mpsc::unbounded_channel::<Event>();
    let cancellation_token = CancellationToken::new();

    let server_event_tx = event_tx.clone();
    let server_cancel_token = cancellation_token.clone();
    let server_thread =
        tokio::spawn(async move { run_server(server_event_tx, server_cancel_token).await });

    let event_thread =
        tokio::spawn(async move { run_event_loop(event_tx, event_rx, cancellation_token).await });

    let (server_res, event_res) = tokio::join!(server_thread, event_thread);
    server_res??;
    event_res??;
    log::info!("all threads have finished");

    Ok(())
}

async fn run_event_loop(
    sender: mpsc::UnboundedSender<Event>,
    mut receiver: mpsc::UnboundedReceiver<Event>,
    cancellation_token: CancellationToken,
) -> Result<()> {
    log::info!("running event loop");
    loop {
        tokio::select! {
            Some(event) = receiver.recv() => {
                handle_event(event, sender.clone(), &cancellation_token).await;
            }
            _ = cancellation_token.cancelled() => {
                    break;
            }
        }
    }
    log::info!("event loop has finished");
    Ok(())
}

async fn handle_event(
    event: Event,
    sender: mpsc::UnboundedSender<Event>,
    cancellation_token: &CancellationToken,
) {
    log::debug!("handling event: {:?}", event);
    match event {
        Event::NewConnection { stream, addr } => {
            tokio::spawn(async move {
                if let Err(err) = handle_client(stream, addr, sender).await {
                    log::error!("[client {}] an error occured: {}", addr, err);
                }
            });
        }
        Event::Quit => cancellation_token.cancel(),
    }
}

async fn run_server(
    sender: mpsc::UnboundedSender<Event>,
    cancellation_token: CancellationToken,
) -> Result<()> {
    let listener = TcpListener::bind(DEFAULT_SERVER_SOCKET).await?;
    log::info!(
        "server started listening on port {}",
        listener.local_addr()?
    );
    loop {
        tokio::select! {
            connection = listener.accept() => {
              let (stream, addr) = connection?;
              sender.send(Event::NewConnection { stream, addr })?;
            }
            _ = cancellation_token.cancelled() => {
                break;
            }
        }
    }
    log::info!("server shutting down");
    Ok(())
}
async fn handle_client(
    mut stream: TcpStream,
    addr: SocketAddr,
    sender: mpsc::UnboundedSender<Event>,
) -> Result<()> {
    log::info!("[client {}] new connection", addr);
    let (reader, mut writer) = stream.split();
    let bufreader = BufReader::new(reader);
    let mut lines = bufreader.lines();

    while let Some(line) = lines.next_line().await? {
        log::debug!("[client {}] received message: {}", addr, line);
        if line.trim().eq_ignore_ascii_case("quit") {
            sender.send(Event::Quit)?;
            writer.write_all(b"goodbye!\n").await?;
            return Ok(());
        }
        writer.write_all(line.as_bytes()).await?;
        writer.write_all(b"\n").await?;
    }
    log::info!("[client {}] disconnected", addr);
    Ok(())
}
