use std::{net::SocketAddr, time::Duration};

use anyhow::Result;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    sync::{mpsc, oneshot},
    time::timeout,
};
use tokio_util::sync::CancellationToken;

use crate::{
    kvstore::{self, Event, command::Command},
    resp::{RespDataType, parser::RespCommandParser},
};

pub struct Connection {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
    addr: SocketAddr,
    cancellation_token: CancellationToken,
    event_channel: mpsc::UnboundedSender<Event>,
    parser: RespCommandParser,
}

impl Connection {
    pub fn new(
        stream: TcpStream,
        addr: SocketAddr,
        cancellation_token: CancellationToken,
        event_channel: mpsc::UnboundedSender<Event>,
    ) -> Self {
        let (reader, writer) = stream.into_split();
        Self {
            reader: BufReader::new(reader),
            writer,
            addr,
            cancellation_token,
            event_channel,
            parser: RespCommandParser::new(),
        }
    }

    pub async fn run(
        stream: TcpStream,
        addr: SocketAddr,
        cancellation_token: CancellationToken,
        event_channel: mpsc::UnboundedSender<Event>,
    ) -> Result<()> {
        log::info!("[client {}] new connection", addr);
        let mut connection = Self::new(stream, addr, cancellation_token, event_channel);
        connection.main_loop().await
    }

    async fn main_loop(&mut self) -> Result<()> {
        let mut linebuf = Vec::new();
        loop {
            tokio::select! {
                _ = self.cancellation_token.cancelled() => {
                    break;
                }

                line_result = self.reader.read_until(b'\n', &mut linebuf) => {
                    let bytes_read = line_result?;
                    if bytes_read == 0 {
                        log::info!("[client {}] disconnected", self.addr);
                        break;
                    }
                    self.process_line(String::from_utf8_lossy(&linebuf).to_string()).await?;
                    linebuf.clear();
                }
            }
        }
        Ok(())
    }

    async fn process_line(&mut self, line: String) -> Result<()> {
        let reply = match self.parser.feed_line(line) {
            Ok(None) => {
                return Ok(());
            }
            Ok(Some(resp_data)) => self.process_resp_data(resp_data).await?,
            Err(err) => err.to_string(),
        };
        self.writer.write_all(reply.as_bytes()).await?;
        self.writer.write_all(b"\r\n").await?;
        Ok(())
    }

    async fn process_resp_data(&self, data: RespDataType) -> Result<String> {
        let command = Command::try_from(data)?;
        let (sender, receiver) = oneshot::channel();
        self.event_channel.send(kvstore::Event {
            reply_channel: sender,
            command,
        })?;

        timeout(Duration::from_millis(500), receiver).await??
    }
}
