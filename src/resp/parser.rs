use std::io::BufRead;

use anyhow::{Context, Result, bail};

use crate::resp::RespDataType;

pub struct RespParser<R: BufRead> {
    reader: R,
}

impl<R: BufRead> RespParser<R> {
    pub fn new(reader: R) -> Self {
        RespParser { reader }
    }

    pub fn parse(&mut self) -> Result<RespDataType> {
        let mut buf = [0u8; 1];
        self.reader
            .read_exact(&mut buf)
            .context("couldn't read the RESP data type byte")?;
        match buf[0] as char {
            '*' => self.parse_array(),
            '$' => self.parse_bulk_string(),
            other => bail!("unknown RESP data type indicator: {}", other),
        }
    }

    fn parse_array(&mut self) -> Result<RespDataType> {
        let element_count: usize = self
            .read_line()
            .context("couldn't read the element count line for the array")?
            .parse()
            .context("the character count line didn't contain a valid number")?;

        let mut data = Vec::new();
        while data.len() != element_count {
            data.push(self.parse().context(format!(
                "an error occurred while parsing the {}nth element of the array",
                data.len() + 1
            ))?);
        }

        Ok(RespDataType::Array { data })
    }

    fn parse_bulk_string(&mut self) -> Result<RespDataType> {
        // a bulk string will look like this: $<character count>\r\n<string>\r\n
        let character_count: usize = self
            .read_line()
            .context("couldn't read the character count line for the bulk string")?
            .parse()
            .context("the character count line didn't contain a valid number")?;
        let string_content = self
            .read_line()
            .context("couldn't read the bulk string content")?;

        if string_content.len() != character_count {
            bail!(
                "the string {} didn't contain the expected amount of characters ({})",
                string_content,
                character_count
            );
        }

        Ok(RespDataType::BulkString {
            data: string_content,
        })
    }

    fn read_line(&mut self) -> Result<String> {
        let mut line = String::new();
        self.reader.read_line(&mut line)?;
        if !line.ends_with("\r\n") {
            bail!("line {} was not terminated properly! (\\r\\n)", line);
        }
        // remove the CRLF line ending
        line.truncate(line.len() - 2);
        Ok(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    enum ExpectedResult {
        Success { result: RespDataType },
        Error,
    }

    struct TestCase {
        input: String,
        expected_result: ExpectedResult,
    }

    fn get_parser(input: &str) -> RespParser<BufReader<&[u8]>> {
        let bufreader = BufReader::new(input.as_bytes());
        let parser = RespParser::new(bufreader);
        parser
    }

    fn make_array(content: &[&str]) -> RespDataType {
        let mut data = Vec::new();
        for string in content {
            data.push(RespDataType::BulkString {
                data: String::from(*string),
            });
        }
        RespDataType::Array { data }
    }

    fn run_test_case(test_case: TestCase) {
        let result = get_parser(&test_case.input).parse();
        match test_case.expected_result {
            ExpectedResult::Success {
                result: expected_result,
            } => {
                assert_eq!(result.unwrap(), expected_result);
            }
            ExpectedResult::Error => {
                result.unwrap_err();
            }
        }
    }

    #[test]
    fn test_invalid_inputs() {
        let inputs = vec![
            "",
            "\r\n\r\n",
            "no type byte",
            "$10\r\n",                 // no string content
            "$10\r\ntest\r\n",         // string too short
            "$4\r\ntesttest\r\n",      // string too long
            "$\r\ntesttest\r\n",       // no string length
            "$xx\r\ntest",             // invalid string length
            "$-10\r\ntest",            // negative string length
            "*-10\r\n",                // negative array length
            "*2\r\n$1\r\nt\r\n",       // not enough elements in array
            "*1\r\n*2\r\n$1\r\nt\r\n", // not enough elements in subarray
            "*1\r\n",                  // empty array
        ];
        for input in inputs {
            let test_case = TestCase {
                input: String::from(input),
                expected_result: ExpectedResult::Error,
            };
            run_test_case(test_case);
        }
    }

    #[test]
    fn test_bulkstring() {
        let test_cases = vec![
            ("$1\r\nt\r\n", "t"),
            ("$4\r\ntest\r\n", "test"),
            ("$10\r\n0123456789\r\n", "0123456789"),
        ];
        for test_case_content in test_cases {
            let (input, expected) = test_case_content;
            let test_case = TestCase {
                input: String::from(input),
                expected_result: ExpectedResult::Success {
                    result: RespDataType::BulkString {
                        data: String::from(expected),
                    },
                },
            };
            run_test_case(test_case);
        }
    }

    #[test]
    fn test_array() {
        let inputs = vec![
            "*1\r\n$5\r\ntest1\r\n",
            "*3\r\n$5\r\ntest1\r\n$5\r\ntest2\r\n$5\r\ntest3\r\n",
            "*1\r\n*2\r\n$5\r\ntest1\r\n$5\r\ntest2\r\n",
            "*2\r\n*3\r\n$6\r\ntest11\r\n$6\r\ntest12\r\n$6\r\ntest13\r\n*2\r\n$6\r\ntest21\r\n$6\r\ntest22\r\n",
        ];
        let expected_results = vec![
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
        for (input, expected) in inputs.iter().zip(expected_results.into_iter()) {
            let test_case = TestCase {
                input: input.to_string(),
                expected_result: ExpectedResult::Success { result: expected },
            };
            run_test_case(test_case);
        }
    }
}
