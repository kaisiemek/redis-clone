mod command_impls;
pub mod commands;

use anyhow::Result;
use std::{collections::HashMap, time::Instant};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{
    kvstore::commands::{Command, parser::parse_command},
    network,
    resp::RespData,
};
pub struct KVStore {
    request_channel: mpsc::UnboundedReceiver<network::Request>,
    cancellation_token: CancellationToken,
    data: HashMap<String, String>,
    expiries: HashMap<String, Instant>,
}

impl KVStore {
    pub fn new(
        request_channel: mpsc::UnboundedReceiver<network::Request>,
        cancellation_token: CancellationToken,
    ) -> Self {
        KVStore {
            request_channel,
            cancellation_token,
            data: HashMap::new(),
            expiries: HashMap::new(),
        }
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        log::info!("[kvstore] running event loop");
        loop {
            tokio::select! {
                Some(request) = self.request_channel.recv() => {
                    self.handle_request(request);
                }
                _ = self.cancellation_token.cancelled() => {
                        break;
                }
            }
        }
        log::info!("[kvstore] event loop has finished");
        Ok(())
    }

    fn handle_request(&mut self, mut req: network::Request) {
        log::debug!("[kvstore] handling request data {:?}", req.argv);
        let argv = std::mem::take(&mut req.argv);
        let command: Command = match parse_command(argv) {
            Ok(cmd) => cmd,
            Err(err) => {
                req.add_reply(err.into());
                req.send_reply();
                return;
            }
        };
        log::debug!("[kvstore] got command from RESP data: {:?}", command);

        let reply = match self.handle_command(command) {
            Ok(reply) => reply,
            Err(err) => err.into(),
        };
        log::debug!("[kvstore] sending reply: {:?}", reply);
        req.add_reply(reply);
        req.send_reply();
    }

    fn handle_command(&mut self, command: Command) -> Result<RespData> {
        let reply = match command {
            Command::Shutdown => self.shutdown(),
            Command::Ping { message } => Self::ping(message),
            Command::Echo { message } => Self::echo(message),
            Command::Ttl { key } => self.ttl(&key),
            Command::Pttl { key } => self.pttl(&key),
            // generic commands
            Command::Del { keys } => self.del(&keys),
            Command::Exists { keys } => self.exists(&keys),
            // string commands
            Command::Append { key, value } => self.append(key, value),
            Command::Decr { key } => self.decr(key),
            Command::Decrby { key, operand } => self.decrby(key, operand),
            Command::Get { key } => self.get(&key),
            Command::Getset { key, value } => self.getset(key, value),
            Command::Incr { key } => self.incr(key),
            Command::Incrby { key, operand } => self.incrby(key, operand),
            Command::Mget { keys } => self.mget(keys),
            Command::Mset { keys, values } => self.mset(keys, values),
            Command::Msetnx { keys, values } => self.msetnx(keys, values),
            Command::Set { key, value, expiry } => self.set(key, value, expiry),
            Command::Setnx { key, value } => self.setnx(key, value),
            Command::Substring { key, begin, end } => self.substring(key, begin, end),
        };
        Ok(reply)
    }

    fn remove_entry(&mut self, key: &str) -> bool {
        log::debug!("[kvstore] attempt to delete key '{}'", key);
        self.expiries.remove(key);
        let deleted = self.data.remove(key).is_some();
        if deleted {
            log::debug!("[kvstore] deleted key '{}'", key);
        }
        deleted
    }
}
