use crate::{kvstore::KVStore, resp::RespDataType};

impl KVStore {
    pub fn ping(message: Option<String>) -> RespDataType {
        match message {
            Some(msg) => Self::echo(msg),
            None => RespDataType::SimpleString {
                data: String::from("PONG"),
            },
        }
    }

    pub fn echo(message: String) -> RespDataType {
        message.into()
    }

    pub fn shutdown(&self) -> RespDataType {
        self.cancellation_token.cancel();
        RespDataType::SimpleString {
            data: String::from("OK"),
        }
    }
}
