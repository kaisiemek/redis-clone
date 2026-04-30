use crate::{kvstore::KVStore, resp::RespData};

impl KVStore {
    pub fn ping(message: Option<String>) -> RespData {
        match message {
            Some(msg) => Self::echo(msg),
            None => RespData::SimpleString(String::from("PONG")),
        }
    }

    pub fn echo(message: String) -> RespData {
        message.into()
    }

    pub fn shutdown(&self) -> RespData {
        self.cancellation_token.cancel();
        RespData::ok()
    }
}
