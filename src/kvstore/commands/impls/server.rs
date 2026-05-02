use std::collections::HashMap;

use crate::{kvstore::KVStore, resp::RespData};

impl KVStore {
    pub(in crate::kvstore::commands) fn dbsize(&self) -> RespData {
        (self.data.len() as i64).into()
    }

    pub(in crate::kvstore::commands) fn echo(message: String) -> RespData {
        message.into()
    }

    pub(in crate::kvstore::commands) fn flushdb(&mut self) -> RespData {
        self.data = HashMap::new();
        self.expiries = HashMap::new();
        self.transactions = HashMap::new();
        RespData::ok()
    }

    pub(in crate::kvstore::commands) fn ping(message: Option<String>) -> RespData {
        match message {
            Some(msg) => Self::echo(msg),
            None => RespData::SimpleString(String::from("PONG")),
        }
    }

    pub(in crate::kvstore::commands) fn save(&mut self) -> RespData {
        self.persist().into()
    }

    pub(in crate::kvstore::commands) fn shutdown(&self) -> RespData {
        self.cancellation_token.cancel();
        RespData::ok()
    }
}
