use std::net::SocketAddr;

use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

const DEFAULT_SERVER_SOCKET: &str = "127.0.0.1:55123";

#[derive(Debug)]
enum Event {
    NewConnection { stream: TcpStream, addr: SocketAddr },
}

pub async fn start() -> Result<()> {
    let listener = TcpListener::bind(DEFAULT_SERVER_SOCKET).await?;
    log::info!("started listening on port {}", listener.local_addr()?);
    let (event_tx, event_rx) = mpsc::unbounded_channel::<Event>();
    let server_event_tx = event_tx.clone();
    tokio::spawn(async move {
        if let Err(err) = run_server_loop(listener, server_event_tx.clone()).await {
            log::error!("an error occurred in the sever thread: {}", err);
        }
    });
    run_event_loop(event_tx, event_rx).await?;
    Ok(())
}

async fn run_event_loop(
    sender: mpsc::UnboundedSender<Event>,
    mut receiver: mpsc::UnboundedReceiver<Event>,
) -> Result<()> {
    log::info!("running event loop");
    while let Some(event) = receiver.recv().await {
        match event {
            Event::NewConnection { stream, addr } => {
                tokio::spawn(async move {
                    if let Err(err) = handle_client(stream, addr).await {
                        log::error!("an error occured while handling client {}: {}", addr, err);
                    }
                });
            }
        }
    }
    log::info!("event loop has finished");
    Ok(())
}

async fn run_server_loop(
    listener: TcpListener,
    sender: mpsc::UnboundedSender<Event>,
) -> Result<()> {
    loop {
        let (stream, addr) = listener.accept().await?;
        sender.send(Event::NewConnection { stream, addr })?;
    }
}
async fn handle_client(mut stream: TcpStream, addr: SocketAddr) -> Result<()> {
    log::info!("new connection by {}", addr);
    let (reader, mut writer) = stream.split();
    let bufreader = BufReader::new(reader);
    let mut lines = bufreader.lines();

    while let Some(line) = lines.next_line().await? {
        writer.write_all(line.as_bytes()).await?;
        writer.write_all(b"\n").await?;
    }
    log::info!("client {} disconnected", addr);
    Ok(())
}
