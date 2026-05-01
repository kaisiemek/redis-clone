use anyhow::{Result, anyhow, bail};

use crate::{
    kvstore::{KVStore, KVStoreValue},
    resp::RespData,
};

impl KVStore {
    pub(in crate::kvstore::commands) fn decr(&mut self, key: String) -> RespData {
        self.calc(key, 1, i64::checked_sub).into()
    }

    pub(in crate::kvstore::commands) fn decrby(&mut self, key: String, operand: i64) -> RespData {
        self.calc(key, operand, i64::checked_sub).into()
    }

    pub(in crate::kvstore::commands) fn gets(&mut self, key: String) -> RespData {
        self.get_string(&key).into()
    }

    pub(in crate::kvstore::commands) fn getset(&mut self, key: String, value: String) -> RespData {
        let previous = match self.get_string(&key) {
            Ok(prev) => prev,
            Err(err) => return err.into(),
        };
        self.set(key, value);
        previous.into()
    }

    pub(in crate::kvstore::commands) fn incr(&mut self, key: String) -> RespData {
        self.calc(key, 1, i64::checked_add).into()
    }

    pub(in crate::kvstore::commands) fn incrby(&mut self, key: String, operand: i64) -> RespData {
        self.calc(key, operand, i64::checked_add).into()
    }

    pub(in crate::kvstore::commands) fn mget(&mut self, keys: Vec<String>) -> RespData {
        keys.into_iter()
            .map(|key| self.get_string_or_nil(&key).into())
            .collect::<Vec<RespData>>()
            .into()
    }

    pub(in crate::kvstore::commands) fn mset(
        &mut self,
        keys: Vec<String>,
        values: Vec<String>,
    ) -> RespData {
        for (key, value) in keys.into_iter().zip(values) {
            self.set(key, value);
        }
        RespData::ok()
    }

    pub(in crate::kvstore::commands) fn msetnx(
        &mut self,
        keys: Vec<String>,
        values: Vec<String>,
    ) -> RespData {
        for key in keys.iter() {
            if self.contains(key) {
                return false.into();
            }
        }
        self.mset(keys, values);
        true.into()
    }

    pub(in crate::kvstore::commands) fn set(&mut self, key: String, value: String) -> RespData {
        self.expiries.remove(&key);
        self.insert(key, value);
        RespData::ok()
    }

    pub(in crate::kvstore::commands) fn setnx(&mut self, key: String, value: String) -> RespData {
        if self.contains(&key) {
            return false.into();
        }
        self.expiries.remove(&key);
        self.insert(key, value);
        true.into()
    }

    pub(in crate::kvstore::commands) fn substring(
        &mut self,
        key: String,
        begin: i64,
        end: i64,
    ) -> RespData {
        let string = match self.get_string(&key) {
            Ok(Some(string)) => string,
            // redis just returns an empty string for keys that don't exist
            Ok(None) => return "".into(),
            Err(err) => return err.into(),
        };

        let (start_index, end_index) = Self::fix_index_range(string.len(), begin, end);
        string[start_index..end_index].into()
    }

    // helpers
    fn get_string(&mut self, key: &str) -> Result<Option<String>> {
        let val = match self.get(key) {
            Some(val) => val,
            None => return Ok(None),
        };
        match val {
            KVStoreValue::String(string) => Ok(Some(string.clone())),
            _ => {
                bail!("WRONGTYPE Operation against a key holding the wrong kind of value")
            }
        }
    }

    fn get_string_or_nil(&mut self, key: &str) -> Option<&str> {
        if let Some(KVStoreValue::String(s)) = self.get(key) {
            Some(s.as_str())
        } else {
            None
        }
    }

    fn calc(&mut self, key: String, operand: i64, op: fn(i64, i64) -> Option<i64>) -> Result<i64> {
        let prev_val: i64 = match self.get_string(&key)? {
            Some(val) => val
                .parse()
                .map_err(|_| anyhow!("ERR value is not an integer or out of range"))?,
            None => 0,
        };

        let new_value = match op(prev_val, operand) {
            Some(new_value) => new_value,
            None => bail!("ERR increment or decrement would overflow"),
        };

        self.insert(key, new_value);
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

        assert_eq!(kvstore.gets("key".into()), None::<String>.into());
        assert_eq!(kvstore.set("key".into(), "value".into()), RespData::ok());
        assert_eq!(kvstore.gets("key".into()), "value".into());
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
        assert_eq!(kvstore.gets("key7".into()), "value7".into());
    }

    #[test]
    fn test_integer_ops() {
        let mut kvstore = KVStore::new(mpsc::unbounded_channel().1, CancellationToken::new());

        assert_eq!(kvstore.gets("no1".into()), None::<String>.into());
        assert_eq!(kvstore.decrby("no1".into(), 1024), (-1024).into());
        assert_eq!(kvstore.decrby("no1".into(), 1024), (-2048).into());
        assert_eq!(kvstore.incrby("no1".into(), -2048), (-4096).into());
        assert_eq!(kvstore.gets("no1".into()), "-4096".into());
        assert_eq!(kvstore.decrby("no1".into(), -2048), (-2048).into());
        assert_eq!(kvstore.decrby("no1".into(), -4096), (2048).into());
        assert_eq!(kvstore.decrby("no1".into(), 2048), (0).into());
        assert_eq!(kvstore.gets("no1".into()), "0".into());

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

        kvstore.set("no4".into(), "NaN".into());
        assert_eq!(
            kvstore.incr("no4".into()),
            anyhow!("ERR value is not an integer or out of range").into()
        );

        kvstore.set("no4".into(), "99999999999999999999999999999999999".into());
        assert_eq!(
            kvstore.incr("no4".into()),
            anyhow!("ERR value is not an integer or out of range").into()
        );
    }

    #[test]
    fn test_substring() {
        let mut kvstore = KVStore::new(mpsc::unbounded_channel().1, CancellationToken::new());
        kvstore.set("key".into(), "0123456789".into());
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
