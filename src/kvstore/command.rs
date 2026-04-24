use anyhow::{Result, anyhow, bail};

use crate::resp::RespDataType;

#[derive(Debug, PartialEq)]
pub enum Command {
    Quit,
    Ping { message: Option<String> },
    Get { key: String },
    Set { key: String, value: String },
}

impl TryFrom<RespDataType> for Command {
    type Error = anyhow::Error;

    fn try_from(respdata: RespDataType) -> Result<Command> {
        let array: Vec<String> = respdata.try_into()?;
        array.try_into()
    }
}

impl TryFrom<Vec<String>> for Command {
    type Error = anyhow::Error;

    fn try_from(array: Vec<String>) -> Result<Command> {
        let cmd = array
            .first()
            .ok_or(anyhow!("received an empty command array"))?
            .to_ascii_lowercase();
        let arg_count_range = Command::get_arg_count_range(cmd.as_str());
        if array.len() - 1 < arg_count_range.0 || array.len() - 1 > arg_count_range.1 {
            bail!("wrong number of arguments for '{}' command", cmd);
        }

        // skip the command itself
        let mut iter = array.into_iter().skip(1);

        let command = match cmd.as_str() {
            "quit" => Command::Quit,
            "ping" => Command::Ping {
                message: iter.next(),
            },
            "get" => Command::Get {
                key: iter.next().unwrap(),
            },
            "set" => Command::Set {
                key: iter.next().unwrap(),
                value: iter.next().unwrap(),
            },
            _ => bail!("unknown command '{}'", cmd),
        };

        Ok(command)
    }
}

impl Command {
    fn get_arg_count_range(cmd: &str) -> (usize, usize) {
        match cmd {
            "quit" => (0, 0),
            "ping" => (0, 1),
            "get" => (1, 1),
            "set" => (2, 2),
            _ => (0, 0),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_invalid_command_parsing() {
        let inputs = vec![
            vec!["quit", "arg"],
            vec!["ping", "message", "too many"],
            vec!["unknown-cmd"],
            vec!["get", "key", "too many"],
            vec!["get"],
            vec!["set"],
            vec!["set", "key"],
            vec!["set", "key", "value", "toomany"],
        ];
        for input in inputs {
            Command::try_from(RespDataType::from(input.as_slice())).unwrap_err();
        }
    }

    #[test]
    fn test_command_parsing() {
        let inputs = vec![
            vec!["quit"],
            vec!["ping"],
            vec!["ping", "test"],
            vec!["get", "key"],
            vec!["set", "key", "value"],
        ];
        let expected_results = vec![
            Command::Quit,
            Command::Ping { message: None },
            Command::Ping {
                message: Some(String::from("test")),
            },
            Command::Get {
                key: String::from("key"),
            },
            Command::Set {
                key: String::from("key"),
                value: String::from("value"),
            },
        ];

        for (input, expected_result) in inputs.into_iter().zip(expected_results.into_iter()) {
            assert_eq!(
                Command::try_from(RespDataType::from(input.as_slice())).unwrap(),
                expected_result
            );
        }
    }
}
