use std::time::Instant;

use crate::{kvstore::KVStore, resp::RespDataType};

impl KVStore {
    pub fn ttl(&mut self, key: &str) -> RespDataType {
        let ttl = self.get_ttl(key);
        if ttl <= 0 {
            RespDataType::Integer(ttl)
        } else {
            RespDataType::Integer(ttl / 1000)
        }
    }

    pub fn pttl(&mut self, key: &str) -> RespDataType {
        RespDataType::Integer(self.get_ttl(key))
    }

    pub fn del(&mut self, keys: &[String]) -> RespDataType {
        let mut keys_deleted: i64 = 0;
        for key in keys {
            if self.del_entry(key) {
                keys_deleted += 1;
            }
        }
        RespDataType::Integer(keys_deleted)
    }

    pub fn del_entry(&mut self, key: &str) -> bool {
        log::debug!("[kvstore] attempt to delete key '{}'", key);
        self.expiries.remove(key);
        let deleted = self.data.remove(key).is_some();
        if deleted {
            log::debug!("[kvstore] deleted key '{}'", key);
        }
        deleted
    }

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
            self.del_entry(key);
            return -2;
        }

        expiry.duration_since(now).as_millis() as i64
    }
}
