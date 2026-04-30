mod command_impls;
mod commands;

use anyhow::Result;
use std::{
    collections::{HashMap, VecDeque},
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{kvstore::commands::Command, network};

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

    fn insert<T: Into<KVStoreValue>>(&mut self, key: String, value: T) {
        let val = value.into();
        log::debug!("[kvstore] setting key '{}' to '{:?}'", key, val);
        self.data.insert(key, val);
    }

    fn get(&mut self, key: &str) -> Option<&KVStoreValue> {
        if let Some(expiry) = self.expiries.get(key)
            && &Instant::now() > expiry
        {
            log::debug!("[kvstore] key '{}' expired", key);
            self.remove(key);
        }
        self.data.get(key)
    }

    fn contains(&mut self, key: &str) -> bool {
        self.get(key).is_some()
    }

    fn set_ttl(&mut self, key: String, ttl: i64) -> bool {
        if !self.contains(&key) {
            return false;
        }
        // redis accepts negative values for the expire command, making the key
        // expire immediately
        let expiry = Instant::now() + Duration::from_secs(ttl.clamp(0, i64::MAX) as u64);
        log::debug!("[kvstore] key '{}' set to expire in {}s", key, ttl);
        self.expiries.insert(key, expiry);
        true
    }

    fn get_ttl(&mut self, key: &str) -> i64 {
        // return -2 if the key doesn't exist at all
        if !self.contains(key) {
            return -2;
        }

        // return -1 if the key does exist, but no TTL is set
        let expiry = match self.expiries.get(key) {
            Some(expiry) => expiry,
            None => return -1,
        };

        let now = Instant::now();

        // delete and return -2 if the TTL has expired
        if expiry < &now {
            self.remove(key);
            return -2;
        }

        expiry.duration_since(now).as_millis() as i64
    }

    fn remove(&mut self, key: &str) -> bool {
        self.expiries.remove(key);
        let deleted = self.data.remove(key).is_some();
        if deleted {
            log::debug!("[kvstore] deleted key '{}'", key);
        }
        deleted
    }
}
