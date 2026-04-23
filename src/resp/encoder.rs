use crate::resp::RespDataType;

pub fn encode_resp_data(resp_data: RespDataType) -> String {
    match resp_data {
        RespDataType::Array { data } => encode_array(data),
        RespDataType::BulkString { data } => encode_bulk_string(data),
    }
}

fn encode_array(array: Vec<RespDataType>) -> String {
    let mut string = format!("*{}\r\n", array.len());
    for element in array {
        string.push_str(&encode_resp_data(element));
    }
    string
}

fn encode_bulk_string(string: String) -> String {
    format!("${}\r\n{}\r\n", string.len(), string)
}

#[cfg(test)]
mod test {
    use super::*;

    fn make_string(content: &str) -> RespDataType {
        RespDataType::BulkString {
            data: content.to_string(),
        }
    }

    fn make_array(content: &[&str]) -> RespDataType {
        RespDataType::Array {
            data: content.iter().map(|s| make_string(s)).collect(),
        }
    }

    #[test]
    fn test_bulkstring() {
        let test_cases = vec![
            ("t", "$1\r\nt\r\n"),
            ("test", "$4\r\ntest\r\n"),
            ("0123456789", "$10\r\n0123456789\r\n"),
        ];

        for test_case in test_cases {
            assert_eq!(encode_resp_data(make_string(test_case.0)), test_case.1);
        }
    }

    #[test]
    fn test_array() {
        let inputs = vec![
            make_array(&["test1"]),
            make_array(&["test1", "test2", "test3"]),
            RespDataType::Array {
                data: vec![make_array(&["test1", "test2"])],
            },
            RespDataType::Array {
                data: vec![
                    make_array(&["test11", "test12", "test13"]),
                    make_array(&["test21", "test22"]),
                ],
            },
        ];
        let expected_results = vec![
            "*1\r\n$5\r\ntest1\r\n",
            "*3\r\n$5\r\ntest1\r\n$5\r\ntest2\r\n$5\r\ntest3\r\n",
            "*1\r\n*2\r\n$5\r\ntest1\r\n$5\r\ntest2\r\n",
            "*2\r\n*3\r\n$6\r\ntest11\r\n$6\r\ntest12\r\n$6\r\ntest13\r\n*2\r\n$6\r\ntest21\r\n$6\r\ntest22\r\n",
        ];
        for (input, expected) in inputs.into_iter().zip(expected_results.iter()) {
            assert_eq!(encode_resp_data(input), String::from(*expected));
        }
    }
}
