use std::collections::VecDeque;

use anyhow::{Result, anyhow, bail};

use crate::{
    kvstore::{KVStore, KVStoreValue},
    resp::RespData,
};

impl KVStore {
    pub(in crate::kvstore::commands) fn lindex(&mut self, key: String, mut index: i64) -> RespData {
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

    pub(in crate::kvstore::commands) fn llen(&mut self, key: String) -> RespData {
        match self.get_list(&key) {
            Ok(Some(list)) => (list.len() as i64).into(),
            Ok(None) => 0.into(),
            Err(err) => err.into(),
        }
    }

    pub(in crate::kvstore::commands) fn lpop(&mut self, key: String) -> RespData {
        self.pop(key, false)
    }

    pub(in crate::kvstore::commands) fn lpush(
        &mut self,
        key: String,
        values: Vec<String>,
    ) -> RespData {
        self.push(key, values, false)
    }

    pub(in crate::kvstore::commands) fn lrange(
        &mut self,
        key: String,
        begin: i64,
        end: i64,
    ) -> RespData {
        let list = match self.get_list(&key) {
            Ok(Some(list)) => list,
            // redis just returns an empty string for keys that don't exist
            Ok(None) => return RespData::Array(Vec::new()),
            Err(err) => return err.into(),
        };

        let (start_index, end_index) = Self::fix_index_range(list.len(), begin, end);
        list.make_contiguous()[start_index..end_index].into()
    }

    pub(in crate::kvstore::commands) fn lrem(
        &mut self,
        key: String,
        mut count: i64,
        element: String,
    ) -> RespData {
        let list = match self.get_list(&key) {
            Ok(Some(list)) => list,
            // redis just returns an empty string for keys that don't exist
            Ok(None) => return 0.into(),
            Err(err) => return err.into(),
        };
        let new_list = std::mem::take(list);

        /* Redis controls the element removal according to the value of count:
         *   count > 0 -> remove the first n elements
         *   count = 0 -> remove all elements
         *   count < 0 -> remove the last n elements
         */
        let reverse = count < 0;
        if count < 0 {
            count = -count;
        } else if count == 0 {
            count = new_list.len() as i64;
        }

        let mut removed: i64 = 0;
        let filter_fn = |el: &String| {
            if el == &element && removed < count {
                removed += 1;
                false
            } else {
                true
            }
        };

        *list = if reverse {
            new_list
                .into_iter()
                .rev()
                .filter(filter_fn)
                .collect::<Vec<String>>()
                .into_iter()
                .rev()
                .collect()
        } else {
            new_list.into_iter().filter(filter_fn).collect()
        };

        if list.is_empty() {
            self.remove(&key);
        }

        removed.into()
    }

    pub(in crate::kvstore::commands) fn lset(
        &mut self,
        key: String,
        mut index: i64,
        element: String,
    ) -> RespData {
        let list = match self.get_list(&key) {
            Ok(Some(list)) => list,
            Ok(None) => return anyhow!("ERR no such key").into(),
            Err(err) => return err.into(),
        };

        // redis allows negative indeces
        if index < 0 {
            index += list.len() as i64;
        }

        if index < 0 || index >= list.len() as i64 {
            return anyhow!("ERR index out of range").into();
        }
        list[index as usize] = element;

        RespData::ok()
    }

    pub(in crate::kvstore::commands) fn ltrim(
        &mut self,
        key: String,
        begin: i64,
        end: i64,
    ) -> RespData {
        let list = match self.get_list(&key) {
            Ok(Some(list)) => list,
            Ok(None) => return RespData::ok(),
            Err(err) => return err.into(),
        };

        let (begin_index, end_index) = Self::fix_index_range(list.len(), begin, end);
        if begin_index >= end_index {
            self.remove(&key);
            return RespData::ok();
        }
        *list = list.drain(begin_index..end_index).collect();

        RespData::ok()
    }

    pub(in crate::kvstore::commands) fn rpop(&mut self, key: String) -> RespData {
        self.pop(key, true)
    }

    pub(in crate::kvstore::commands) fn rpush(
        &mut self,
        key: String,
        values: Vec<String>,
    ) -> RespData {
        self.push(key, values, true)
    }

    fn pop(&mut self, key: String, popright: bool) -> RespData {
        let list = match self.get_list(&key) {
            Ok(Some(list)) => list,
            Ok(None) => return RespData::NullBulkString,
            Err(err) => return err.into(),
        };

        let popped_el = match if popright {
            list.pop_back()
        } else {
            list.pop_front()
        } {
            Some(el) => el.into(),
            None => RespData::NullBulkString,
        };

        if list.is_empty() {
            self.remove(&key);
        }
        popped_el
    }

    fn push(&mut self, key: String, values: Vec<String>, pushright: bool) -> RespData {
        match self.get_list(&key) {
            Ok(Some(list)) => {
                if pushright {
                    list.extend(values);
                } else {
                    for val in values {
                        list.push_front(val);
                    }
                }
                (list.len() as i64).into()
            }
            Ok(None) => {
                let len = values.len();
                self.insert(
                    key,
                    KVStoreValue::List(if pushright {
                        VecDeque::from_iter(values)
                    } else {
                        VecDeque::from_iter(values.into_iter().rev())
                    }),
                );
                (len as i64).into()
            }
            Err(err) => err.into(),
        }
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
    fn test_push_pop_len() {
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
        assert_eq!(kvstore.rpop("l".into()), RespData::BulkString("a".into()));
        assert_eq!(kvstore.rpop("l".into()), RespData::BulkString("b".into()));
        assert_eq!(
            kvstore.rpush("l".into(), make_vec(vec!["x", "y", "z"])),
            5.into()
        );
        expect_range(&mut kvstore, "l", 0, -1, vec!["d", "c", "x", "y", "z"]);
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

        let expected = ["d", "c", "b", "a"];
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

    #[test]
    fn test_lrem() {
        let mut kvstore = KVStore::new(mpsc::unbounded_channel().1, CancellationToken::new());
        kvstore.lpush(
            "l".into(),
            make_vec(vec!["d", "x", "x", "c", "x", "x", "b", "x", "x", "a"]),
        );
        assert_eq!(kvstore.lrem("l".into(), 2, "x".into()), 2.into());
        expect_range(
            &mut kvstore,
            "l",
            0,
            -1,
            vec!["a", "b", "x", "x", "c", "x", "x", "d"],
        );
        assert_eq!(kvstore.lrem("l".into(), -3, "x".into()), 3.into());
        expect_range(&mut kvstore, "l", 0, -1, vec!["a", "b", "x", "c", "d"]);

        kvstore.lpush(
            "l2".into(),
            make_vec(vec!["d", "x", "x", "c", "x", "x", "b", "x", "x", "a"]),
        );
        assert_eq!(kvstore.lrem("l2".into(), 10, "y".into()), 0.into());
        expect_range(
            &mut kvstore,
            "l2",
            0,
            -1,
            vec!["a", "x", "x", "b", "x", "x", "c", "x", "x", "d"],
        );
        assert_eq!(kvstore.lrem("l2".into(), 10, "x".into()), 6.into());
        expect_range(&mut kvstore, "l2", 0, -1, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn test_lset() {
        let mut kvstore = KVStore::new(mpsc::unbounded_channel().1, CancellationToken::new());

        kvstore.lpush("l".into(), make_vec(vec!["c", "b", "a"]));
        assert_eq!(
            kvstore.lset("l".into(), -3, "x".into()),
            RespData::SimpleString("OK".into())
        );
        expect_range(&mut kvstore, "l", 0, -1, vec!["x", "b", "c"]);

        assert_eq!(
            kvstore.lset("l".into(), 1, "x".into()),
            RespData::SimpleString("OK".into())
        );
        expect_range(&mut kvstore, "l", 0, -1, vec!["x", "x", "c"]);

        assert_eq!(
            kvstore.lset("l".into(), 4, "x".into()),
            RespData::SimpleError("ERR index out of range".into())
        );
        assert_eq!(
            kvstore.lset("l".into(), -4, "x".into()),
            RespData::SimpleError("ERR index out of range".into())
        );
        assert_eq!(
            kvstore.lset("l1".into(), 4, "x".into()),
            RespData::SimpleError("ERR no such key".into())
        );
    }
    #[test]
    fn test_ltrim() {
        let mut kvstore = KVStore::new(mpsc::unbounded_channel().1, CancellationToken::new());

        kvstore.lpush(
            "l".into(),
            make_vec(vec!["k", "j", "i", "h", "g", "f", "e", "d", "c", "b", "a"]),
        );
        assert_eq!(
            kvstore.ltrim("l".into(), 0, 8),
            RespData::SimpleString("OK".into())
        );
        expect_range(
            &mut kvstore,
            "l",
            0,
            -1,
            vec!["a", "b", "c", "d", "e", "f", "g", "h", "i"],
        );
        assert_eq!(
            kvstore.ltrim("l".into(), -4, -1),
            RespData::SimpleString("OK".into())
        );
        expect_range(&mut kvstore, "l", 0, -1, vec!["f", "g", "h", "i"]);
        assert_eq!(
            kvstore.ltrim("l".into(), 1, 0),
            RespData::SimpleString("OK".into())
        );
        expect_range(&mut kvstore, "l", 0, -1, vec![]);

        kvstore.lpush(
            "l".into(),
            make_vec(vec!["k", "j", "i", "h", "g", "f", "e", "d", "c", "b", "a"]),
        );
        assert_eq!(
            kvstore.ltrim("l".into(), -1000, 2),
            RespData::SimpleString("OK".into())
        );
        expect_range(&mut kvstore, "l", 0, -1, vec!["a", "b", "c"]);
        assert_eq!(
            kvstore.ltrim("l".into(), 1, 1000),
            RespData::SimpleString("OK".into())
        );
        expect_range(&mut kvstore, "l", 0, -1, vec!["b", "c"]);
    }
}
