pub mod encoder;

use anyhow::{Result, bail};

#[derive(Debug, PartialEq)]
pub enum RespData {
    Array { data: Vec<RespData> },
    BulkString { data: String },
    Error { message: String },
    Nil,
    SimpleString(String),
    Integer(i64),
}

// clone the &strs in the slice into an Array of BulkString
impl From<&[&str]> for RespData {
    fn from(value: &[&str]) -> Self {
        RespData::Array {
            data: value.iter().map(|s| RespData::from(*s)).collect(),
        }
    }
}

// clone the &str data into a BulkString
impl From<&str> for RespData {
    fn from(value: &str) -> Self {
        RespData::BulkString {
            data: value.to_string(),
        }
    }
}

// move the String data into a BulkString
impl From<String> for RespData {
    fn from(value: String) -> Self {
        RespData::BulkString { data: value }
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
            None => RespData::Nil,
        }
    }
}

impl<T: Into<RespData>> From<Result<T>> for RespData {
    fn from(value: Result<T>) -> Self {
        match value {
            Ok(value) => value.into(),
            Err(err) => RespData::Error {
                message: err.to_string(),
            },
        }
    }
}

impl<T: Into<RespData>> From<Vec<T>> for RespData {
    fn from(value: Vec<T>) -> Self {
        Self::Array {
            data: value.into_iter().map(|el| el.into()).collect(),
        }
    }
}

impl From<anyhow::Error> for RespData {
    fn from(value: anyhow::Error) -> Self {
        Self::Error {
            message: value.to_string(),
        }
    }
}

impl TryFrom<RespData> for String {
    type Error = anyhow::Error;

    fn try_from(value: RespData) -> Result<Self> {
        match value {
            RespData::BulkString { data } => Ok(data),
            other => bail!("can't convert RESP data {:?} to string", other),
        }
    }
}

impl TryFrom<RespData> for Vec<String> {
    type Error = anyhow::Error;

    fn try_from(value: RespData) -> Result<Self> {
        let mut vec = Vec::new();
        match value {
            RespData::Array { data } => {
                for element in data {
                    vec.push(element.try_into()?);
                }
            }
            RespData::BulkString { data } => {
                vec.push(data);
            }
            other => bail!("{:?} can't be converted to a string vector", other),
        }
        Ok(vec)
    }
}
