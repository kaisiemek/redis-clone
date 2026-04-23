use anyhow::Result;

use crate::resp::RespDataType;

#[derive(Debug)]
pub enum Commands {
    Quit,
    Ping,
    Set { key: String, value: String },
    Get { key: String },
}

impl TryFrom<RespDataType> for Commands {
    type Error = anyhow::Error;

    fn try_from(respdata: &RespDataType) -> Result<Commands> {}
}
