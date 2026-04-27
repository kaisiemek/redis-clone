use std::time::Instant;

use crate::{kvstore::KVStore, resp::RespDataType};

impl KVStore {
    pub fn append(&mut self, key: String, value: String) -> RespDataType {
        log::debug!("[kvstore] appending string '{}' to key '{}'", value, key);
        let new_size = match self.data.get_mut(&key) {
            Some(val) => {
                val.push_str(&value);
                val.len()
            }
            None => {
                let new_size = value.len();
                self.data.insert(key, value);
                new_size
            }
        } as i64;

        RespDataType::Integer(new_size)
    }

    pub fn get(&mut self, key: &str) -> RespDataType {
        log::debug!("[kvstore] accessing key '{}'", key);
        if let Some(expiry) = self.expiries.get(key) {
            if &Instant::now() > expiry {
                log::debug!("[kvstore] key '{}' expired, removing...", key);
                self.del_entry(key);
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
        RespDataType::SimpleString(String::from("OK"))
    }
}

#[cfg(test)]
mod test {
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use super::*;

    #[test]
    fn test_get_set() {
        let mut kvstore = KVStore::new(mpsc::unbounded_channel().1, CancellationToken::new());

        assert_eq!(kvstore.get("key"), None.into());
        assert_eq!(
            kvstore.set("key".into(), "value".into(), None),
            RespDataType::SimpleString("OK".into())
        );
        assert_eq!(kvstore.get("key"), "value".into());
    }

    #[test]
    fn test_append() {
        let mut kvstore = KVStore::new(mpsc::unbounded_channel().1, CancellationToken::new());

        assert_eq!(kvstore.get("key"), None.into());
        assert_eq!(kvstore.append("key".into(), "val1".into()), 4.into());
        assert_eq!(kvstore.get("key"), "val1".into());
        assert_eq!(kvstore.append("key".into(), "val2".into()), 8.into());
        assert_eq!(kvstore.get("key"), "val1val2".into());
        assert_eq!(kvstore.append("key".into(), "".into()), 8.into());
        assert_eq!(kvstore.get("key"), "val1val2".into());
        assert_eq!(kvstore.get("key"), "val1val2".into());
    }
}
