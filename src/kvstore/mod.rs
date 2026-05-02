mod commands;
mod transaction;

use anyhow::Result;
use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
    time::Instant,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{kvstore::transaction::Transaction, network};

#[derive(Debug)]
pub enum KVStoreValue {
    String(String),
    List(VecDeque<String>),
}

impl From<&str> for KVStoreValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<String> for KVStoreValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<i64> for KVStoreValue {
    fn from(value: i64) -> Self {
        Self::String(value.to_string())
    }
}

pub struct KVStore {
    request_channel: mpsc::UnboundedReceiver<network::Request>,
    cancellation_token: CancellationToken,
    data: HashMap<String, KVStoreValue>,
    expiries: HashMap<String, Instant>,
    transactions: HashMap<SocketAddr, Transaction>,
    current_client: Option<SocketAddr>,
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
            transactions: HashMap::new(),
            current_client: None,
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
        self.current_client = Some(req.connection);
        let argv = std::mem::take(&mut req.argv);
        let reply = self.process_command(argv);
        log::debug!("[kvstore] sending reply: {:?}", reply);
        req.send_reply(reply);
        self.current_client = None;
    }
}
