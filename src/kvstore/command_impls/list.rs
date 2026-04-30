use std::collections::VecDeque;

use anyhow::{Result, bail};

use crate::{
    kvstore::{KVStore, KVStoreValue},
    resp::RespData,
};

impl KVStore {
    pub fn lindex(&mut self, key: String, mut index: i64) -> RespData {
        let list = match self.get_list(&key) {
            Ok(Some(list)) => list,
            Ok(None) => return RespData::NullBulkString,
            Err(err) => return err.into(),
        };

        // redis can use negative indeces like Python, Rust slicing doesn't allow that
        if index < 0 {
            index += list.len() as i64;
        }

        // return nil when the index is out of range of the list
        if index < 0 || index >= list.len() as i64 {
            return RespData::NullBulkString;
        }

        list[index as usize].as_str().into()
    }

    pub fn llen(&mut self, key: String) -> RespData {
        match self.get_list(&key) {
            Ok(Some(list)) => (list.len() as i64).into(),
            Ok(None) => return 0.into(),
            Err(err) => return err.into(),
        }
    }

    pub fn lpop(&mut self, key: String) -> RespData {
        let list = match self.get_list(&key) {
            Ok(Some(list)) => list,
            Ok(None) => return RespData::NullBulkString,
            Err(err) => return err.into(),
        };

        let popped_el: RespData = match list.pop_front() {
            Some(el) => el.into(),
            None => RespData::NullBulkString,
        };

        if list.is_empty() {
            self.remove(&key);
        }
        popped_el
    }

    pub fn lpush(&mut self, key: String, values: Vec<String>) -> RespData {
        match self.get_list(&key) {
            Ok(Some(list)) => {
                for val in values {
                    list.push_front(val);
                }
                (list.len() as i64).into()
            }
            Ok(None) => {
                let new_list = VecDeque::from_iter(values.into_iter().rev());
                let len = new_list.len() as i64;
                self.insert(key, KVStoreValue::List(new_list));
                len.into()
            }
            Err(err) => return err.into(),
        }
    }

    pub fn lrange(&mut self, key: String, begin: i64, end: i64) -> RespData {
        let list = match self.get_list(&key) {
            Ok(Some(list)) => list,
            // redis just returns an empty string for keys that don't exist
            Ok(None) => return RespData::Array(Vec::new()),
            Err(err) => return err.into(),
        };

        let (start_index, end_index) = Self::fix_index_range(list.len(), begin, end);
        list.make_contiguous()[start_index..end_index].into()
    }

    fn get_list(&mut self, key: &str) -> Result<Option<&mut VecDeque<String>>> {
        let val = match self.get(key) {
            Some(val) => val,
            None => return Ok(None),
        };
        match val {
            KVStoreValue::List(list) => Ok(Some(list)),
            _ => {
                bail!("WRONGTYPE Operation against a key holding the wrong kind of value")
            }
        }
    }
}

#[cfg(test)]
mod test {
    use tokio::sync::mpsc;
    use tokio_util::sync::CancellationToken;

    use super::*;

    fn make_vec(v: Vec<&str>) -> Vec<String> {
        v.iter().map(|el| el.to_string()).collect()
    }

    fn get_vec_from_reply(r: RespData) -> Vec<String> {
        let arr = match r {
            RespData::Array(arr) => arr,
            other => panic!("expected RESP array, got {:?}", other),
        };

        let mut v = Vec::new();
        for el in arr {
            match el {
                RespData::BulkString(string) => {
                    v.push(string);
                }
                other => {
                    panic!("array contained unexpected RESP type: {:?}", other);
                }
            }
        }

        v
    }

    fn get_str_from_reply(r: RespData) -> String {
        match r {
            RespData::BulkString(string) => string,
            other => panic!("expected RESP bulk string, got {:?}", other),
        }
    }

    fn expect_range(kvstore: &mut KVStore, key: &str, begin: i64, end: i64, expect: Vec<&str>) {
        let reply = get_vec_from_reply(kvstore.lrange(key.into(), begin, end));
        assert_eq!(reply, make_vec(expect));
    }

    fn expect_index(kvstore: &mut KVStore, key: &str, index: i64, expect: &str) {
        let reply = get_str_from_reply(kvstore.lindex(key.into(), index));
        assert_eq!(reply, expect);
    }

    #[test]
    fn test_push_pop() {
        let mut kvstore = KVStore::new(mpsc::unbounded_channel().1, CancellationToken::new());
        assert_eq!(kvstore.llen("l".into()), RespData::Integer(0));
        assert_eq!(kvstore.lpop("l".into()), RespData::NullBulkString);
        assert_eq!(
            kvstore.lpush("l".into(), make_vec(vec!["a", "b", "c"])),
            3.into()
        );
        assert_eq!(kvstore.llen("l".into()), RespData::Integer(3));
        assert_eq!(
            kvstore.lpush("l".into(), make_vec(vec!["d", "e", "f"])),
            6.into()
        );
        assert_eq!(kvstore.llen("l".into()), RespData::Integer(6));
        expect_range(&mut kvstore, "l", 0, -1, vec!["f", "e", "d", "c", "b", "a"]);
        assert_eq!(kvstore.lpop("l".into()), RespData::BulkString("f".into()));
        assert_eq!(kvstore.lpop("l".into()), RespData::BulkString("e".into()));
        expect_range(&mut kvstore, "l", 0, -1, vec!["d", "c", "b", "a"]);
    }

    #[test]
    fn test_list_access() {
        let mut kvstore = KVStore::new(mpsc::unbounded_channel().1, CancellationToken::new());
        kvstore.lpush("l".into(), make_vec(vec!["a", "b", "c", "d"]));
        // d c b a
        expect_range(&mut kvstore, "l", 0, -1, vec!["d", "c", "b", "a"]);
        expect_range(&mut kvstore, "l", -3, -1, vec!["c", "b", "a"]);
        expect_range(&mut kvstore, "l", 0, 1, vec!["d", "c"]);
        expect_range(&mut kvstore, "l", -50, 0, vec!["d"]);
        expect_range(&mut kvstore, "l", 50, 100, vec![]);
        expect_range(&mut kvstore, "l1", 0, -1, vec![]);

        let expected = vec!["d", "c", "b", "a"];
        for (i, e) in expected.iter().enumerate() {
            expect_index(&mut kvstore, "l", i as i64, e);
        }
        for (i, e) in expected.iter().rev().enumerate() {
            expect_index(&mut kvstore, "l", -((i + 1) as i64), e);
        }
        let reply = kvstore.lindex("l".into(), -5);
        let RespData::NullBulkString = reply else {
            panic!("expected nil, got {:?}", reply);
        };
        let reply = kvstore.lindex("l".into(), 5);
        let RespData::NullBulkString = reply else {
            panic!("expected nil, got {:?}", reply);
        };
        let reply = kvstore.lindex("l1".into(), 0);
        let RespData::NullBulkString = reply else {
            panic!("expected nil, got {:?}", reply);
        };
    }
}
