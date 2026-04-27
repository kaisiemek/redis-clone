use std::time::Instant;

use crate::{kvstore::KVStore, resp::RespDataType};

impl KVStore {
    pub fn get(&mut self, key: &str) -> RespDataType {
        log::debug!("[kvstore] accessing key '{}'", key);
        if let Some(expiry) = self.expiries.get(key) {
            if &Instant::now() > expiry {
                log::debug!("[kvstore] key '{}' expired, removing...", key);
                self.expiries.remove(key);
                self.data.remove(key);
            }
        }
        self.data.get(key).cloned().into()
    }

    pub fn set(&mut self, key: String, value: String, expiry: Option<Instant>) -> RespDataType {
        log::debug!("[kvstore] setting '{}' to value '{}'", key, value);
        if let Some(expiry) = expiry {
            log::debug!(
                "[kvstore] key '{}' bound to expire in {}ms",
                key,
                expiry.duration_since(Instant::now()).as_millis()
            );
            self.expiries.insert(key.clone(), expiry);
        }
        self.data.insert(key, value);
        RespDataType::SimpleString {
            data: String::from("OK"),
        }
    }
}
