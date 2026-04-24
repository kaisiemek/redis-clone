pub mod command;
mod string_commands;

use crate::kvstore::command::Command;
use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub struct Event {
    pub command: command::Command,
    pub reply_channel: oneshot::Sender<Result<String>>,
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
        log::info!("running event loop");
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
        log::info!("event loop has finished");
        Ok(())
    }

    fn handle_event(&mut self, event: Event) {
        log::debug!("handling event {:?}", event.command);
        let reply = match event.command {
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
        if event.reply_channel.send(Ok(reply)).is_err() {
            log::error!("couldn't reply to the event!");
        }
    }
}
