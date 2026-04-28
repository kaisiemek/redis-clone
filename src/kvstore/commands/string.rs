use std::time::Instant;

use anyhow::{Result, anyhow, bail};

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

    pub fn decr(&mut self, key: String) -> RespDataType {
        log::debug!("[kvstore] decrementing integer value '{}'", key);
        self.calc(key, 1, i64::checked_sub).into()
    }

    pub fn decrby(&mut self, key: String, operand: i64) -> RespDataType {
        log::debug!(
            "[kvstore] decrementing integer value '{}' by {}",
            key,
            operand
        );
        self.calc(key, operand, i64::checked_sub).into()
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

    pub fn getset(&mut self, key: String, value: String) -> RespDataType {
        let previous = self.get(&key);
        self.expiries.remove(&key);
        self.set(key, value, None);
        previous
    }

    pub fn incr(&mut self, key: String) -> RespDataType {
        log::debug!("[kvstore] incrementing integer value '{}'", key);
        self.calc(key, 1, i64::checked_add).into()
    }

    pub fn incrby(&mut self, key: String, operand: i64) -> RespDataType {
        log::debug!(
            "[kvstore] incrementing integer value '{}' by {}",
            key,
            operand
        );
        self.calc(key, operand, i64::checked_add).into()
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

    // helpers
    fn calc(&mut self, key: String, operand: i64, op: fn(i64, i64) -> Option<i64>) -> Result<i64> {
        let previous_value: i64 = match self.data.get(&key) {
            Some(val) => val
                .parse()
                .map_err(|_| anyhow!("ERR value is not an integer or out of range"))?,
            None => 0,
        };

        let new_value = match op(previous_value, operand) {
            Some(new_value) => new_value,
            None => bail!("ERR increment or decrement would overflow"),
        };

        self.data.insert(key, format!("{}", new_value));
        Ok(new_value)
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

        assert_eq!(kvstore.get("key"), None::<String>.into());
        assert_eq!(
            kvstore.set("key".into(), "value".into(), None),
            RespDataType::SimpleString("OK".into())
        );
        assert_eq!(kvstore.get("key"), "value".into());
        assert_eq!(
            kvstore.getset("key".into(), "newvalue".into()),
            "value".into()
        );
        assert_eq!(
            kvstore.getset("newkey".into(), "value".into()),
            None::<String>.into()
        );
    }

    #[test]
    fn test_append() {
        let mut kvstore = KVStore::new(mpsc::unbounded_channel().1, CancellationToken::new());

        assert_eq!(kvstore.get("key"), None::<String>.into());
        assert_eq!(kvstore.append("key".into(), "val1".into()), 4.into());
        assert_eq!(kvstore.get("key"), "val1".into());
        assert_eq!(kvstore.append("key".into(), "val2".into()), 8.into());
        assert_eq!(kvstore.get("key"), "val1val2".into());
        assert_eq!(kvstore.append("key".into(), "".into()), 8.into());
        assert_eq!(kvstore.get("key"), "val1val2".into());
        assert_eq!(kvstore.get("key"), "val1val2".into());
    }

    #[test]
    fn test_integer_ops() {
        let mut kvstore = KVStore::new(mpsc::unbounded_channel().1, CancellationToken::new());

        assert_eq!(kvstore.get("no1"), None::<String>.into());
        assert_eq!(kvstore.decrby("no1".into(), 1024), (-1024).into());
        assert_eq!(kvstore.decrby("no1".into(), 1024), (-2048).into());
        assert_eq!(kvstore.incrby("no1".into(), -2048), (-4096).into());
        assert_eq!(kvstore.get("no1"), "-4096".into());
        assert_eq!(kvstore.decrby("no1".into(), -2048), (-2048).into());
        assert_eq!(kvstore.decrby("no1".into(), -4096), (2048).into());
        assert_eq!(kvstore.decrby("no1".into(), 2048), (0).into());
        assert_eq!(kvstore.get("no1"), "0".into());

        assert_eq!(
            kvstore.decrby("no2".into(), i64::MAX),
            (i64::MIN + 1).into()
        );
        assert_eq!(kvstore.decr("no2".into()), i64::MIN.into());
        assert_eq!(
            kvstore.decr("no2".into()),
            anyhow!("ERR increment or decrement would overflow").into()
        );

        assert_eq!(kvstore.incrby("no3".into(), i64::MAX), i64::MAX.into());
        assert_eq!(
            kvstore.incr("no3".into()),
            anyhow!("ERR increment or decrement would overflow").into()
        );

        kvstore.set("no4".into(), "NaN".into(), None);
        assert_eq!(
            kvstore.incr("no4".into()),
            anyhow!("ERR value is not an integer or out of range").into()
        );

        kvstore.set(
            "no4".into(),
            "99999999999999999999999999999999999".into(),
            None,
        );
        assert_eq!(
            kvstore.incr("no4".into()),
            anyhow!("ERR value is not an integer or out of range").into()
        );
    }
}
