use crate::resp::RespDataType;

pub fn encode_resp_data(resp_data: RespDataType) -> String {
    match resp_data {
        RespDataType::Array { data } => encode_array(data),
        RespDataType::BulkString { data } => format!("${}\r\n{}\r\n", data.len(), data),
        RespDataType::Error { message } => format!("-ERR {}\r\n", message),
        RespDataType::Nil => String::from("_\r\n"),
        RespDataType::SimpleString(string) => format!("+{}\r\n", string),
    }
}

fn encode_array(array: Vec<RespDataType>) -> String {
    let mut string = format!("*{}\r\n", array.len());
    for element in array {
        string.push_str(&encode_resp_data(element));
    }
    string
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_array() {
        let inputs = vec![
            ["test1"].as_slice().into(),
            ["test1", "test2", "test3"].as_slice().into(),
            RespDataType::Array {
                data: vec![["test1", "test2"].as_slice().into()],
            },
            RespDataType::Array {
                data: vec![
                    ["test11", "test12", "test13"].as_slice().into(),
                    ["test21", "test22"].as_slice().into(),
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

    #[test]
    fn test_bulkstring() {
        let test_cases = vec![
            ("t", "$1\r\nt\r\n"),
            ("test", "$4\r\ntest\r\n"),
            ("0123456789", "$10\r\n0123456789\r\n"),
        ];

        for test_case in test_cases {
            assert_eq!(encode_resp_data(test_case.0.into()), test_case.1);
        }
    }

    #[test]
    fn test_error() {
        let test_cases = vec![
            (anyhow::anyhow!("error message"), "-ERR error message\r\n"),
            (anyhow::anyhow!(""), "-ERR \r\n"),
        ];
        for test_case in test_cases {
            assert_eq!(encode_resp_data(test_case.0.into()), test_case.1);
        }
    }

    #[test]
    fn test_simple_string() {
        let test_cases = vec![("simple string", "+simple string\r\n"), ("", "+\r\n")];
        for test_case in test_cases {
            assert_eq!(
                encode_resp_data(RespDataType::SimpleString(test_case.0.into())),
                test_case.1.to_string()
            );
        }
    }
}
