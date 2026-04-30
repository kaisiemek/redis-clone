use anyhow::Result;

/*
* there are 6 data types in Redis serialization protocol v2 (RESP)
* see https://redis.io/docs/latest/develop/reference/protocol-spec/
*
* simple string    (+)
* bulk string      (&)
* integer          (:)
* simple error     (-)
* null bulk string ($-1)
* array            (*)
*
* provide some convenience From trait implementations for multiple
* rust types:
*  &str/String      -> bulk string
*  i64              -> integer
*  Option<T>'s None -> null bulk string
*  Result<T>'s Err  -> simple error
*  anyhow::Error    -> simple error
*  Vec<T>           -> array
* */
#[derive(Debug, PartialEq)]
pub enum RespData {
    SimpleString(String),
    SimpleError(String),
    Integer(i64),
    BulkString(String),
    NullBulkString,
    Array(Vec<RespData>),
}

impl RespData {
    pub fn ok() -> RespData {
        RespData::SimpleString("OK".to_string())
    }
    pub fn encode(&self) -> String {
        match self {
            RespData::Array(array) => Self::encode_array(array),
            RespData::BulkString(string) => format!("${}\r\n{}\r\n", string.len(), string),
            RespData::SimpleError(message) => format!("-{}\r\n", message),
            RespData::NullBulkString => String::from("$-1\r\n"),
            RespData::SimpleString(string) => format!("+{}\r\n", string),
            RespData::Integer(int) => format!(":{}\r\n", int),
        }
    }

    fn encode_array(array: &Vec<RespData>) -> String {
        let mut string = format!("*{}\r\n", array.len());
        for element in array {
            string.push_str(element.encode().as_str());
        }
        string
    }
}

impl From<&str> for RespData {
    fn from(value: &str) -> Self {
        RespData::BulkString(value.to_string())
    }
}

impl From<String> for RespData {
    fn from(value: String) -> Self {
        RespData::BulkString(value)
    }
}

impl From<i64> for RespData {
    fn from(value: i64) -> Self {
        RespData::Integer(value)
    }
}

impl<T: Into<RespData>> From<Option<T>> for RespData {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(value) => value.into(),
            None => RespData::NullBulkString,
        }
    }
}

impl<T: Into<RespData>> From<Result<T>> for RespData {
    fn from(value: Result<T>) -> Self {
        match value {
            Ok(value) => value.into(),
            Err(err) => RespData::SimpleError(err.to_string()),
        }
    }
}

impl<T: Into<RespData>> From<Vec<T>> for RespData {
    fn from(value: Vec<T>) -> Self {
        Self::Array(value.into_iter().map(|element| element.into()).collect())
    }
}

impl From<anyhow::Error> for RespData {
    fn from(value: anyhow::Error) -> Self {
        Self::SimpleError(value.to_string())
    }
}
