pub mod command;
mod string_commands;

use crate::{kvstore::command::Command, resp::RespDataType};
use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub struct Event {
    pub data: RespDataType,
    pub reply_channel: oneshot::Sender<RespDataType>,
}
pub struct KVStore {
    event_channel: mpsc::UnboundedReceiver<Event>,
    cancellation_token: CancellationToken,
    data: HashMap<String, String>,
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
                Self::send_reply(event.reply_channel, RespDataType::from(err.to_string()));
                return;
            }
        };
        log::debug!("[kvstore] got command from RESP data: {:?}", command);

        let reply = match self.handle_command(command) {
            Ok(reply) => RespDataType::from(reply),
            Err(err) => RespDataType::from(err.to_string()),
        };
        log::debug!("[kvstore] sending reply: {:?}", reply);
        Self::send_reply(event.reply_channel, reply);
    }

    fn handle_command(&mut self, command: Command) -> Result<String> {
        let reply = match command {
            Command::Quit => {
                self.cancellation_token.cancel();
                String::new()
            }
            Command::Ping { message } => match message {
                None => String::from("PONG"),
                Some(msg) => msg,
            },
            Command::Set { key, value } => {
                self.set(key, value);
                String::from("OK")
            }
            Command::Get { key } => self
                .get(&key)
                .map(|val| format!("\"{}\"", val))
                .unwrap_or(String::from("(nil)")),
        };
        Ok(reply)
    }

    fn send_reply(channel: oneshot::Sender<RespDataType>, data: RespDataType) {
        if channel.send(data).is_err() {
            log::error!("[kvstore] couldn't reply to the event!");
        }
    }
}
