use crate::{kvstore::KVStore, resp::RespData};

impl KVStore {
    pub(in crate::kvstore::commands) fn ping(message: Option<String>) -> RespData {
        match message {
            Some(msg) => Self::echo(msg),
            None => RespData::SimpleString(String::from("PONG")),
        }
    }

    pub(in crate::kvstore::commands) fn echo(message: String) -> RespData {
        message.into()
    }

    pub(in crate::kvstore::commands) fn shutdown(&self) -> RespData {
        self.cancellation_token.cancel();
        RespData::ok()
    }
}
