use crate::{kvstore::KVStore, resp::RespData};

impl KVStore {
    pub fn expire(&mut self, key: String, ttl: i64) -> RespData {
        self.set_ttl(key, ttl).into()
    }

    pub fn ttl(&mut self, key: &str) -> RespData {
        let ttl = self.get_ttl(key);
        if ttl <= 0 {
            ttl.into()
        } else {
            (ttl / 1000).into()
        }
    }

    pub fn pttl(&mut self, key: &str) -> RespData {
        self.get_ttl(key).into()
    }

    pub fn del(&mut self, keys: &[String]) -> RespData {
        let mut keys_deleted: i64 = 0;
        for key in keys {
            if self.remove(key) {
                keys_deleted += 1;
            }
        }
        keys_deleted.into()
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
}
