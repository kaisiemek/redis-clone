pub mod encoder;
pub mod parser;

use anyhow::{Result, bail};

#[derive(Debug, PartialEq)]
pub enum RespDataType {
    Array { data: Vec<RespDataType> },
    BulkString { data: String },
    Error { message: String },
    Nil,
}

// clone the &strs in the slice into an Array of BulkString
impl From<&[&str]> for RespDataType {
    fn from(value: &[&str]) -> Self {
        RespDataType::Array {
            data: value.iter().map(|s| RespDataType::from(*s)).collect(),
        }
    }
}

// clone the &str data into a BulkString
impl From<&str> for RespDataType {
    fn from(value: &str) -> Self {
        RespDataType::BulkString {
            data: value.to_string(),
        }
    }
}

// move the String data into a BulkString
impl From<String> for RespDataType {
    fn from(value: String) -> Self {
        RespDataType::BulkString { data: value }
    }
}

impl From<Option<String>> for RespDataType {
    fn from(value: Option<String>) -> Self {
        match value {
            Some(string) => string.into(),
            None => RespDataType::Nil,
        }
    }
}

impl From<anyhow::Error> for RespDataType {
    fn from(value: anyhow::Error) -> Self {
        Self::Error {
            message: value.to_string(),
        }
    }
}

impl TryFrom<RespDataType> for String {
    type Error = anyhow::Error;

    fn try_from(value: RespDataType) -> Result<Self> {
        match value {
            RespDataType::BulkString { data } => Ok(data),
            other => bail!("can't convert RESP data {:?} to string", other),
        }
    }
}

impl TryFrom<RespDataType> for Vec<String> {
    type Error = anyhow::Error;

    fn try_from(value: RespDataType) -> Result<Self> {
        let mut vec = Vec::new();
        match value {
            RespDataType::Array { data } => {
                for element in data {
                    vec.push(element.try_into()?);
                }
            }
            RespDataType::BulkString { data } => {
                vec.push(data);
            }
            other => bail!("{:?} can't be converted to a string vector", other),
        }
        Ok(vec)
    }
}
