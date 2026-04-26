use std::{net::SocketAddr, time::Duration};

use anyhow::{Result, bail};
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
    kvstore::{self, Event},
    resp::{encoder::encode_resp_data, parser::RespCommandParser},
};

pub struct Connection {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
    addr: SocketAddr,
    cancellation_token: CancellationToken,
    event_channel: mpsc::UnboundedSender<Event>,
    parser: RespCommandParser,
    linebuf: Vec<u8>,
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
            linebuf: Vec::new(),
        }
    }

    pub async fn run(
        stream: TcpStream,
        addr: SocketAddr,
        cancellation_token: CancellationToken,
        event_channel: mpsc::UnboundedSender<Event>,
    ) {
        log::info!("[connection {}] established", addr);
        let mut connection = Self::new(stream, addr, cancellation_token, event_channel);
        match connection.main_loop().await {
            Ok(_) => log::info!("[connection {}] ended, client disconnected", addr),
            Err(err) => log::error!("[connection {}] ended, {}", addr, err),
        }
    }

    async fn main_loop(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                _ = self.cancellation_token.cancelled() => {
                    bail!("server shutting down");
                }

                line_result = self.reader.read_until(b'\n', &mut self.linebuf) => {
                    let bytes_read = line_result?;
                    if bytes_read == 0 {
                        break;
                    }
                    let line = String::from_utf8_lossy(&self.linebuf).to_string();
                    log::debug!("[connection {}] received {} bytes: {}", self.addr, bytes_read, line.replace("\r\n",
                    "\\r\\n"));
                    self.process_line(line).await?;
                    self.linebuf.clear();
                }
            }
        }
        Ok(())
    }

    async fn process_line(&mut self, line: String) -> Result<()> {
        if let Some(reply) = self.get_reply(line).await {
            log::debug!(
                "[connection {}] sending reply: {}",
                self.addr,
                reply.replace("\r\n", "\\r\\n")
            );
            self.writer.write_all(reply.as_bytes()).await?;
        }
        Ok(())
    }

    async fn get_reply(&mut self, line: String) -> Option<String> {
        self.produce_reply(line)
            .await
            .unwrap_or_else(|err| Some(err.to_string()))
    }

    async fn produce_reply(&mut self, line: String) -> Result<Option<String>> {
        let Some(resp_data) = self.parser.feed_line(line)? else {
            return Ok(None);
        };
        log::debug!(
            "[connection {}] got RESP data from parser: {:?}",
            self.addr,
            resp_data
        );
        let (sender, receiver) = oneshot::channel();
        self.event_channel.send(kvstore::Event {
            reply_channel: sender,
            data: resp_data,
        })?;
        let kvstore_reply = timeout(Duration::from_millis(500), receiver).await??;
        Ok(Some(encode_resp_data(kvstore_reply)))
    }
}
