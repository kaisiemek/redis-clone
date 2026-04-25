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
            Command::Quit => {
                self.cancellation_token.cancel();
                "OK".into()
            }
            Command::Ping { message } => match message {
                None => "PONG".into(),
                Some(msg) => msg.into(),
            },
            Command::Set { key, value } => {
                self.set(key, value);
                "OK".into()
            }
            Command::Get { key } => self.get(&key),
        };
        Ok(reply)
    }

    fn send_reply(channel: oneshot::Sender<RespDataType>, data: RespDataType) {
        if channel.send(data).is_err() {
            log::error!("[kvstore] couldn't reply to the event!");
        }
    }
}
