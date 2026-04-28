pub mod commands;

use anyhow::Result;
use std::{collections::HashMap, time::Instant};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use crate::{kvstore::commands::Command, resp::RespDataType};

#[derive(Debug)]
pub struct Event {
    pub data: RespDataType,
    pub reply_channel: oneshot::Sender<RespDataType>,
}
pub struct KVStore {
    event_channel: mpsc::UnboundedReceiver<Event>,
    cancellation_token: CancellationToken,
    data: HashMap<String, String>,
    expiries: HashMap<String, Instant>,
}

impl KVStore {
    pub fn new(
        event_channel: mpsc::UnboundedReceiver<Event>,
        cancellation_token: CancellationToken,
    ) -> Self {
        KVStore {
            event_channel,
            cancellation_token,
            data: HashMap::new(),
            expiries: HashMap::new(),
        }
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        log::info!("[kvstore] running event loop");
        loop {
            tokio::select! {
                Some(event) = self.event_channel.recv() => {
                    self.handle_event(event);
                }
                _ = self.cancellation_token.cancelled() => {
                        break;
                }
            }
        }
        log::info!("[kvstore] event loop has finished");
        Ok(())
    }

    fn handle_event(&mut self, event: Event) {
        log::debug!("[kvstore] handling event data {:?}", event.data);

        let command: Command = match Command::try_from(event.data) {
            Ok(cmd) => cmd,
            Err(err) => {
                Self::send_reply(event.reply_channel, err.into());
                return;
            }
        };
        log::debug!("[kvstore] got command from RESP data: {:?}", command);

        let reply = match self.handle_command(command) {
            Ok(reply) => reply,
            Err(err) => err.into(),
        };
        log::debug!("[kvstore] sending reply: {:?}", reply);
        Self::send_reply(event.reply_channel, reply);
    }

    fn handle_command(&mut self, command: Command) -> Result<RespDataType> {
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

    fn send_reply(channel: oneshot::Sender<RespDataType>, data: RespDataType) {
        if channel.send(data).is_err() {
            log::error!("[kvstore] couldn't reply to the event!");
        }
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
