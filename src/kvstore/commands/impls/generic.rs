use anyhow::anyhow;

use crate::{
    kvstore::{KVStore, KVStoreValue},
    resp::RespData,
};

impl KVStore {
    pub(in crate::kvstore::commands) fn del(&mut self, keys: &[String]) -> RespData {
        let mut keys_deleted: i64 = 0;
        for key in keys {
            if self.remove(key) {
                keys_deleted += 1;
            }
        }
        keys_deleted.into()
    }

    pub(in crate::kvstore::commands) fn exists(&self, keys: &[String]) -> RespData {
        let mut existing_keys: i64 = 0;
        for key in keys {
            if self.data.contains_key(key) {
                existing_keys += 1;
            }
        }
        existing_keys.into()
    }

    pub(in crate::kvstore::commands) fn expire(&mut self, key: String, ttl: i64) -> RespData {
        self.set_ttl(key, ttl).into()
    }

    pub(in crate::kvstore::commands) fn rename(&mut self, key: String, newkey: String) -> RespData {
        if self.move_element(key, newkey) {
            RespData::ok()
        } else {
            anyhow!("ERR no such key").into()
        }
    }

    pub(in crate::kvstore::commands) fn renamenx(
        &mut self,
        key: String,
        newkey: String,
    ) -> RespData {
        if !self.contains(&key) {
            return anyhow!("ERR no such key").into();
        }
        if self.contains(&newkey) {
            return 0.into();
        }

        self.move_element(key, newkey).into()
    }

    pub(in crate::kvstore::commands) fn ttl(&mut self, key: String) -> RespData {
        self.get_ttl(&key).into()
    }

    pub(in crate::kvstore::commands) fn value_type(&mut self, key: String) -> RespData {
        match self.get(&key) {
            Some(KVStoreValue::String(_)) => "string".into(),
            Some(KVStoreValue::List(_)) => "list".into(),
            None => "none".into(),
        }
    }
}
