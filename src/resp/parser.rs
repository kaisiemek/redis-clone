use anyhow::{Context, Result, ensure};

use crate::resp::RespDataType;

enum ParserState {
    ReadArrayLength,
    ReadBulkStringLength,
    ReadString { length: usize },
}
pub struct RespCommandParser {
    command_fragments: Vec<RespDataType>,
    expected_array_size: usize,
    current_state: ParserState,
}

impl RespCommandParser {
    pub fn new() -> Self {
        Self {
            command_fragments: Vec::new(),
            expected_array_size: 0,
            current_state: ParserState::ReadArrayLength,
        }
    }

    pub fn feed_line(&mut self, line: String) -> Result<Option<RespDataType>> {
        let result = self.feed_line_inner(line);
        // reset the parser if the command parsing is complete or an error occurred
        match result {
            Err(_) | Ok(Some(_)) => self.reset(),
            _ => {}
        }
        result
    }

    fn feed_line_inner(&mut self, mut line: String) -> Result<Option<RespDataType>> {
        // remove the line ending
        ensure!(
            line.ends_with("\r\n"),
            "[parser] line wasn't properly terminated: {}",
            line
        );
        line.truncate(line.len() - 2);

        match self.current_state {
            ParserState::ReadArrayLength => self.read_array_length(line)?,
            ParserState::ReadBulkStringLength => self.read_bulk_string_length(line)?,
            ParserState::ReadString { length } => self.read_bulk_string(line, length)?,
        }

        if self.expected_array_size == 0 {
            Ok(Some(RespDataType::Array { data: Vec::new() }))
        } else if self.command_fragments.len() == self.expected_array_size {
            let elements = std::mem::take(&mut self.command_fragments);
            Ok(Some(RespDataType::Array { data: elements }))
        } else {
            Ok(None)
        }
    }

    fn read_array_length(&mut self, line: String) -> Result<()> {
        ensure!(
            line.starts_with('*'),
            "[parser] expected an array length line, got: {}",
            line
        );
        self.expected_array_size = line[1..].parse().context("[parser] invalid array length")?;

        // just ignore empty arrays and wait for the next one
        if self.expected_array_size != 0 {
            self.current_state = ParserState::ReadBulkStringLength;
        }
        Ok(())
    }

    fn read_bulk_string_length(&mut self, line: String) -> Result<()> {
        ensure!(
            line.starts_with('$'),
            "[parser] expected a bulk string length line, got: {}",
            line
        );
        let string_length: usize = line[1..]
            .parse()
            .context("[parser] invalid string length")?;
        self.current_state = ParserState::ReadString {
            length: string_length,
        };
        Ok(())
    }

    fn read_bulk_string(&mut self, line: String, expected_length: usize) -> Result<()> {
        ensure!(
            line.len() == expected_length,
            "[parser] bulk string had the wrong length, expected: {}, got: {}",
            expected_length,
            line.len()
        );
        self.command_fragments
            .push(RespDataType::BulkString { data: line });
        self.current_state = ParserState::ReadBulkStringLength;
        Ok(())
    }

    fn reset(&mut self) {
        self.expected_array_size = 0;
        self.current_state = ParserState::ReadArrayLength;
        self.command_fragments.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_inputs() {
        let inputs = vec![
            " ",
            "\r\n\r\n",
            "no array len",
            "$10\r\n",                      // start with bulk string
            "*1\r\n$10\r\ntest\r\n",        // string too short
            "*1\r\n$4\r\ntesttest\r\n",     // string too long
            "*1\r\n$\r\ntesttest\r\n",      // no string length
            "*1$xx\r\ntest",                // invalid string length
            "*1$-10\r\ntest",               // negative string length
            "*1\r\n$1\r\nt\r\n$1\r\nt\r\n", // string after valid array
            "*\r\n",                        // no array length
            "*-1\r\n",                      // negative array length
            "*xx\r\n",                      // invalid array length
        ];

        let mut parser = RespCommandParser::new();
        for input in inputs {
            let mut error_occurred = false;
            for line in input.split_inclusive("\r\n") {
                error_occurred = parser.feed_line(line.to_string()).is_err();
            }
            assert!(error_occurred, "no error occured for input {}", input);
        }
    }

    #[test]
    fn test_valid_arrays() {
        let inputs = vec![
            "*1\r\n$5\r\ntest1\r\n",
            "*3\r\n$5\r\ntest1\r\n$5\r\ntest2\r\n$5\r\ntest3\r\n",
            "*0\r\n",
            "*2\r\n$0\r\n\r\n$4\r\ntest\r\n",
        ];
        let expected_results: Vec<RespDataType> = vec![
            ["test1"].as_slice().into(),
            ["test1", "test2", "test3"].as_slice().into(),
            [].as_slice().into(),
            ["", "test"].as_slice().into(),
        ];

        let mut parser = RespCommandParser::new();
        for (input, expected) in inputs.into_iter().zip(expected_results.into_iter()) {
            let mut output = None;
            for line in input.split_inclusive("\r\n") {
                output = parser.feed_line(line.to_string()).unwrap();
            }

            assert_eq!(output.unwrap(), expected);
        }
    }
}
