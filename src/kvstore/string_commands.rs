use crate::{kvstore::KVStore, resp::RespDataType};

impl KVStore {
    pub fn get(&self, key: &str) -> RespDataType {
        self.data.get(key).cloned().into()
    }

    pub fn set(&mut self, key: String, value: String) {
        self.data.insert(key, value);
    }
}
