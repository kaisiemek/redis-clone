mod command_impls;
mod commands;

use anyhow::Result;
use std::{collections::HashMap, time::Instant};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{kvstore::commands::Command, network};

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
        let command: Command = match argv.try_into() {
            Ok(cmd) => cmd,
            Err(err) => {
                req.send_reply(err.into());
                return;
            }
        };
        log::debug!("[kvstore] got command from RESP data: {:?}", command);

        let reply = self.handle_command(command);
        log::debug!("[kvstore] sending reply: {:?}", reply);
        req.send_reply(reply);
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
