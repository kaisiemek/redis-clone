use std::fs::File;

use anyhow::Result;
use chrono::Utc;

use crate::kvstore::{KVStore, KVStoreValue, transaction::Transaction};

impl KVStore {
    pub(in crate::kvstore::commands) fn contains(&mut self, key: &str) -> bool {
        self.get(key).is_some()
    }

    pub(in crate::kvstore::commands) fn get(&mut self, key: &str) -> Option<&mut KVStoreValue> {
        if let Some(expiry) = self.expiries.get(key)
            && expiry - Utc::now().timestamp() <= 0
        {
            log::debug!("[kvstore] key '{}' expired", key);
            self.remove(key);
        }
        self.data.get_mut(key)
    }

    pub(in crate::kvstore::commands) fn insert<T: Into<KVStoreValue>>(
        &mut self,
        key: String,
        value: T,
    ) {
        let val = value.into();
        log::debug!("[kvstore] setting key '{}' to '{:?}'", key, val);
        self.data.insert(key, val);
    }

    pub(in crate::kvstore::commands) fn remove(&mut self, key: &str) -> bool {
        self.expiries.remove(key);
        let deleted = self.data.remove(key).is_some();
        if deleted {
            log::debug!("[kvstore] deleted key '{}'", key);
        }
        deleted
    }

    pub(in crate::kvstore::commands) fn get_ttl(&mut self, key: &str) -> i64 {
        // return -2 if the key doesn't exist at all
        if !self.contains(key) {
            return -2;
        }

        // return -1 if the key does exist, but no TTL is set
        let expiry = match self.expiries.get(key) {
            Some(expiry) => expiry,
            None => return -1,
        };

        let ttl = expiry - Utc::now().timestamp();
        // delete and return -2 if the TTL has expired
        if ttl <= 0 {
            self.remove(key);
            return -2;
        }

        return ttl;
    }

    pub(in crate::kvstore::commands) fn set_ttl(&mut self, key: String, ttl: i64) -> bool {
        if !self.contains(&key) {
            return false;
        }
        // redis accepts negative values for the expire command, making the key
        // expire immediately
        let expiry = Utc::now().timestamp() + ttl;
        log::debug!("[kvstore] key '{}' set to expire in {}s", key, ttl);
        self.expiries.insert(key, expiry);
        true
    }

    pub(in crate::kvstore::commands) fn fix_index_range(
        len: usize,
        begin: i64,
        end: i64,
    ) -> (usize, usize) {
        let start_index = Self::fix_index(len, begin);
        // redis uses inclusive end indeces
        // i.e. redis: getrange "0123" 0 0 -> "0", rust: "0123"[0..1] -> "0"
        let mut end_index = Self::fix_index(len, end);
        end_index = (end_index + 1).clamp(0, len as usize);

        if end_index < start_index {
            (0, 0)
        } else {
            (start_index, end_index)
        }
    }

    pub(in crate::kvstore::commands) fn fix_index(len: usize, mut index: i64) -> usize {
        // redis can use negative indeces like Python, Rust slicing doesn't allow that
        if index < 0 {
            index += len as i64;
        }

        // if the index is still negative clamp to 0
        // if the index is larger than the collection, clamp to max size
        index.clamp(0, len as i64) as usize
    }

    pub(in crate::kvstore::commands) fn get_current_transaction(
        &mut self,
    ) -> Option<&mut Transaction> {
        let Some(client) = self.current_client else {
            return None;
        };
        self.transactions.get_mut(&client)
    }

    pub(in crate::kvstore::commands) fn persist_kvstore(&mut self) -> Result<()> {
        let mut file = File::create("kvstore.mpk")?;
        rmp_serde::encode::write(&mut file, &self.data)?;
        let mut file2 = File::create("expiries.mpk")?;
        rmp_serde::encode::write(&mut file2, &self.expiries)?;
        Ok(())
    }
}
