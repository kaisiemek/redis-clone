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
        Vec::try_from(respdata)?.try_into()
    }
}

impl TryFrom<Vec<String>> for Command {
    type Error = anyhow::Error;

    fn try_from(array: Vec<String>) -> Result<Command> {
        let mut iter = array.into_iter();
        let cmd = iter
            .next()
            .ok_or(anyhow!("received an empty command array"))?;

        let command = match cmd.as_str() {
            "quit" => Command::Quit,
            "ping" => Command::Ping {
                message: iter.next(),
            },
            "get" => Command::Get {
                key: ensure_next_arg(&mut iter, &cmd)?,
            },
            "set" => Command::Set {
                key: ensure_next_arg(&mut iter, &cmd)?,
                value: ensure_next_arg(&mut iter, &cmd)?,
            },
            _ => bail!("unknown command '{}'", cmd),
        };

        if iter.next().is_some() {
            bail!("wrong number of arguments for '{}' command", cmd);
        }
        Ok(command)
    }
}

fn ensure_next_arg<I: Iterator<Item = String>>(iter: &mut I, command: &str) -> Result<String> {
    iter.next().ok_or(anyhow!(
        "wrong number of arguments for '{}' command",
        command
    ))
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
