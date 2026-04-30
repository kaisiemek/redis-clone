use anyhow::{Context, Result};

enum ParserState {
    ArrayLength,
    BulkStringLength,
    String { length: usize },
}

pub struct RequestParser {
    argv: Vec<String>,
    argc: usize,
    current_state: ParserState,
}

impl RequestParser {
    pub fn new() -> Self {
        Self {
            argv: Vec::new(),
            argc: 0,
            current_state: ParserState::ArrayLength,
        }
    }

    pub fn feed_line(&mut self, line: String) -> Result<Option<Vec<String>>> {
        let result = self.feed_line_inner(line);
        // reset the parser if the command parsing is complete or an error occurred
        match result {
            Err(_) | Ok(Some(_)) => self.reset(),
            Ok(None) => {}
        }
        result
    }

    fn feed_line_inner(&mut self, mut line: String) -> Result<Option<Vec<String>>> {
        // remove the line ending
        line = line.trim_end().to_string();

        match self.current_state {
            ParserState::ArrayLength => self.read_array_length(line)?,
            ParserState::BulkStringLength => self.read_bulk_string_length(line)?,
            ParserState::String { length } => self.read_bulk_string(line, length)?,
        }

        if self.argc == 0 {
            Ok(Some(Vec::new()))
        } else if self.argv.len() == self.argc {
            let argv = std::mem::take(&mut self.argv);
            Ok(Some(argv))
        } else {
            Ok(None)
        }
    }

    fn read_array_length(&mut self, line: String) -> Result<()> {
        // redis is very lenient and also takes commands not encoded properly
        // as an RESP array
        if !line.starts_with('*') {
            self.read_one_line_command(line);
            return Ok(());
        }

        self.argc = line[1..]
            .parse()
            .context("ERR Protocol error: invalid multibulk length")?;

        // just ignore empty arrays and wait for the next one
        if self.argc != 0 {
            self.current_state = ParserState::BulkStringLength;
        }
        Ok(())
    }

    fn read_bulk_string_length(&mut self, line: String) -> Result<()> {
        // ignore unexpected lines
        if !line.starts_with('$') {
            return Ok(());
        }
        let string_length: usize = line[1..]
            .parse()
            .context("ERR Protocol error: invalid bulkstring length")?;
        self.current_state = ParserState::String {
            length: string_length,
        };
        Ok(())
    }

    fn read_bulk_string(&mut self, mut line: String, expected_length: usize) -> Result<()> {
        // redis is again fairly lenient and just truncates the line and discards the
        // rest if the bulk string is too long
        line.truncate(expected_length);
        self.argv.push(line);
        self.current_state = ParserState::BulkStringLength;
        Ok(())
    }

    fn read_one_line_command(&mut self, line: String) {
        let mut arg = String::new();
        let mut quoted = false;

        // allow whitespace in between quotes
        for c in line.chars() {
            match c {
                '"' | '\'' => {
                    quoted = !quoted;
                }
                ' ' | '\t' | '\n' | '\r' => {
                    if quoted {
                        arg.push(c);
                    } else if !arg.is_empty() {
                        self.argv.push(std::mem::take(&mut arg));
                    }
                }
                _ => {
                    arg.push(c);
                }
            }
        }

        if !arg.is_empty() {
            self.argv.push(arg);
        }
        self.argc = self.argv.len();
    }

    fn reset(&mut self) {
        self.argc = 0;
        self.current_state = ParserState::ArrayLength;
        self.argv.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsing() {
        let inputs = vec![
            "*1\r\n$4\r\ntest\r\n",
            "*1\r\ntest\r\n$4\r\ntest\r\n",
            "*2\r\n$4\r\ntest\r\n$5\r\ntest2\r\n",
            "*1\r\n$10\r\ntest\r\n",
            "*1\r\n$3\r\ntest\r\n",
            "one line command\r\n",
            "one \"line command\"\r\n",
            "$10\r\n",
        ];
        let expected_results: Vec<Vec<String>> = vec![
            vec!["test".into()],
            vec!["test".into()],
            vec!["test".into(), "test2".into()],
            vec!["test".into()],
            vec!["tes".into()],
            vec!["one".into(), "line".into(), "command".into()],
            vec!["one".into(), "line command".into()],
            vec!["$10".into()],
        ];

        let mut parser = RequestParser::new();
        for (input, expected) in inputs.into_iter().zip(expected_results) {
            let mut output = None;
            for line in input.split_inclusive("\r\n") {
                output = parser.feed_line(line.to_string()).unwrap();
            }
            assert_eq!(output.unwrap(), expected);
        }
    }

    #[test]
    fn test_invalid_inputs() {
        let inputs = vec![
            "*1\r\n$\r\ntesttest\r\n", // no string length
            "*1\r\n$xx\r\ntest",       // invalid string length
            "*1\r\n$-10\r\ntest",      // negative string length
            "*\r\n",                   // no array length
            "*-1\r\n",                 // negative array length
            "*xx\r\n",                 // invalid array length
        ];

        let mut parser = RequestParser::new();
        for input in inputs {
            let mut error_occurred = false;
            for line in input.split_inclusive("\r\n") {
                error_occurred |= parser.feed_line(line.to_string()).is_err();
            }
            assert!(error_occurred, "no error occured for input {}", input);
        }
    }
}
