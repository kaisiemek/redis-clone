use std::net::SocketAddr;

use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

const DEFAULT_SERVER_SOCKET: &str = "127.0.0.1:55123";

pub async fn start() -> Result<()> {
    let listener = TcpListener::bind(DEFAULT_SERVER_SOCKET).await?;
    log::info!("started listening on port {}", listener.local_addr()?);

    loop {
        let (stream, addr) = listener.accept().await?;

        tokio::spawn(async move {
            if let Err(err) = handle_client(stream, addr).await {
                log::error!("an error occured while handling client {}: {}", addr, err);
            }
        });
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
