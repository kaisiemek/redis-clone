pub mod encoder;
pub mod parser;

#[derive(Debug, PartialEq)]
pub enum RespDataType {
    Array { data: Vec<RespDataType> },
    BulkString { data: String },
}
