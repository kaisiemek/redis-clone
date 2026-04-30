use std::time::Instant;

use anyhow::{Result, anyhow, bail};

use crate::{kvstore::KVStore, resp::RespData};

impl KVStore {
    pub fn append(&mut self, key: String, value: String) -> RespData {
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

        new_size.into()
    }

    pub fn decr(&mut self, key: String) -> RespData {
        log::debug!("[kvstore] decrementing integer value '{}'", key);
        self.calc(key, 1, i64::checked_sub).into()
    }

    pub fn decrby(&mut self, key: String, operand: i64) -> RespData {
        log::debug!(
            "[kvstore] decrementing integer value '{}' by {}",
            key,
            operand
        );
        self.calc(key, operand, i64::checked_sub).into()
    }

    pub fn get(&mut self, key: &str) -> RespData {
        log::debug!("[kvstore] accessing key '{}'", key);
        if let Some(expiry) = self.expiries.get(key)
            && &Instant::now() > expiry
        {
            log::debug!("[kvstore] key '{}' expired, removing...", key);
            self.remove_entry(key);
        }
        self.data.get(key).cloned().into()
    }

    pub fn getset(&mut self, key: String, value: String) -> RespData {
        let previous = self.get(&key);
        self.expiries.remove(&key);
        self.set(key, value, None);
        previous
    }

    pub fn incr(&mut self, key: String) -> RespData {
        log::debug!("[kvstore] incrementing integer value '{}'", key);
        self.calc(key, 1, i64::checked_add).into()
    }

    pub fn incrby(&mut self, key: String, operand: i64) -> RespData {
        log::debug!(
            "[kvstore] incrementing integer value '{}' by {}",
            key,
            operand
        );
        self.calc(key, operand, i64::checked_add).into()
    }

    pub fn mget(&mut self, keys: Vec<String>) -> RespData {
        keys.iter()
            .map(|key| self.get(key))
            .collect::<Vec<_>>()
            .into()
    }

    pub fn mset(&mut self, keys: Vec<String>, values: Vec<String>) -> RespData {
        for (key, value) in keys.into_iter().zip(values) {
            self.set(key, value, None);
        }
        RespData::ok()
    }

    pub fn msetnx(&mut self, keys: Vec<String>, values: Vec<String>) -> RespData {
        if self.exists(&keys) != RespData::Integer(0) {
            return 0.into();
        }
        self.mset(keys, values);
        1.into()
    }

    pub fn set(&mut self, key: String, value: String, expiry: Option<Instant>) -> RespData {
        log::debug!("[kvstore] setting '{}' to value '{}'", key, value);
        self.expiries.remove(&key);
        if let Some(expiry) = expiry {
            log::debug!(
                "[kvstore] key '{}' bound to expire in {}ms",
                key,
                expiry.duration_since(Instant::now()).as_millis()
            );
            self.expiries.insert(key.clone(), expiry);
        }
        self.data.insert(key, value);
        RespData::ok()
    }

    pub fn setnx(&mut self, key: String, value: String) -> RespData {
        if self.data.contains_key(&key) {
            return 0.into();
        }

        self.data.insert(key, value);
        1.into()
    }

    pub fn substring(&mut self, key: String, begin: i64, end: i64) -> RespData {
        let value = match self.get(&key) {
            RespData::NullBulkString => return "".into(),
            RespData::BulkString(string) => string,
            _ => {
                return anyhow!(
                    " WRONGTYPE Operation against a key holding the wrong kind of value"
                )
                .into();
            }
        };

        let start_index = Self::fix_index(value.len() as i64, begin);
        // redis uses inclusive end indeces
        // i.e. redis: getrange "0123" 0 0 -> "0", rust: "0123"[0..1] -> "0"
        let mut end_index = Self::fix_index(value.len() as i64, end);
        end_index = (end_index + 1).clamp(0, value.len());

        if end_index < start_index {
            return "".into();
        }
        value[start_index..end_index].into()
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

    fn fix_index(len: i64, mut index: i64) -> usize {
        // redis can use negative indeces like Python, Rust slicing doesn't allow that
        if index < 0 {
            index += len;
        }

        index.clamp(0, len) as usize
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
            RespData::ok()
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

        assert_eq!(
            kvstore.mset(
                vec!["key1".into(), "key2".into(), "key3".into()],
                vec!["val1".into(), "val2".into(), "".into()]
            ),
            RespData::ok()
        );
        assert_eq!(
            kvstore.exists(&["key1".into(), "key2".into(), "key7".into()]),
            2.into()
        );
        assert_eq!(
            kvstore.mget(vec!["key1".into(), "key2".into(), "key7".into()]),
            vec![
                String::from("val1").into(),
                String::from("val2").into(),
                RespData::NullBulkString
            ]
            .into()
        );
        assert_eq!(
            kvstore.msetnx(
                vec!["key3".into(), "key4".into(), "key5".into()],
                vec!["".into(), "".into(), "".into()]
            ),
            0.into()
        );
        assert_eq!(
            kvstore.msetnx(
                vec!["key4".into(), "key5".into(), "key6".into()],
                vec!["".into(), "".into(), "".into()]
            ),
            1.into()
        );
        assert_eq!(kvstore.setnx("key7".into(), "value7".into()), 1.into());
        assert_eq!(kvstore.setnx("key7".into(), "value8".into()), 0.into());
        assert_eq!(kvstore.get("key7"), "value7".into());
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

    #[test]
    fn test_substring() {
        let mut kvstore = KVStore::new(mpsc::unbounded_channel().1, CancellationToken::new());
        kvstore.set("key".into(), "0123456789".into(), None);
        assert_eq!(kvstore.substring("key".into(), 0, 0), "0".into());
        assert_eq!(kvstore.substring("key".into(), 0, 1), "01".into());
        assert_eq!(kvstore.substring("key".into(), 1, 3), "123".into());
        assert_eq!(kvstore.substring("key".into(), 0, -1), "0123456789".into());
        assert_eq!(kvstore.substring("key".into(), -20, 0), "0".into());
        assert_eq!(
            kvstore.substring("key".into(), -20, -1),
            "0123456789".into()
        );
        assert_eq!(kvstore.substring("key".into(), 20, 21), "".into());
        assert_eq!(kvstore.substring("key".into(), 9, 20), "9".into());
        assert_eq!(kvstore.substring("key".into(), 9, 9), "9".into());
        assert_eq!(kvstore.substring("key".into(), 9, -1), "9".into());
        assert_eq!(kvstore.substring("unknown".into(), 0, 0), "".into());
    }
}
