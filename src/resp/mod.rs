pub mod encoder;
pub mod parser;

#[derive(Debug, PartialEq)]
pub enum RespDataType {
    Array { data: Vec<RespDataType> },
    BulkString { data: String },
}

pub struct RespEncoder {}

impl RespEncoder {
    pub fn new() -> Self {
        RespEncoder {}
    }

    pub fn encode(&self, resp_data: RespDataType) -> String {
        match resp_data {
            RespDataType::Array { data } => self.encode_array(data),
            RespDataType::BulkString { data } => self.encode_bulk_string(data),
        }
    }

    fn encode_array(&self, array: Vec<RespDataType>) -> String {
        let mut string = format!("*{}\r\n", array.len());
        for element in array {
            string.push_str(&self.encode(element));
        }
        string
    }

    fn encode_bulk_string(&self, string: String) -> String {
        format!("${}\r\n{}\r\n", string.len(), string)
    }
}
