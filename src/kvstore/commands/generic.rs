use std::time::Instant;

use crate::{kvstore::KVStore, resp::RespData};

impl KVStore {
    pub fn ttl(&mut self, key: &str) -> RespData {
        let ttl = self.get_ttl(key);
        if ttl <= 0 {
            RespData::Integer(ttl)
        } else {
            RespData::Integer(ttl / 1000)
        }
    }

    pub fn pttl(&mut self, key: &str) -> RespData {
        RespData::Integer(self.get_ttl(key))
    }

    pub fn del(&mut self, keys: &[String]) -> RespData {
        let mut keys_deleted: i64 = 0;
        for key in keys {
            if self.remove_entry(key) {
                keys_deleted += 1;
            }
        }
        RespData::Integer(keys_deleted)
    }

    pub fn exists(&self, keys: &[String]) -> RespData {
        let mut existing_keys: i64 = 0;
        for key in keys {
            if self.data.contains_key(key) {
                existing_keys += 1;
            }
        }
        existing_keys.into()
    }

    // helper
    fn get_ttl(&mut self, key: &str) -> i64 {
        log::debug!("[kvstore] checking TTL for key '{}'", key);
        // return -2 if the key doesn't exist at all
        if !self.data.contains_key(key) {
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
            self.remove_entry(key);
            return -2;
        }

        expiry.duration_since(now).as_millis() as i64
    }
}

