use std::time::{Duration, Instant};

use anyhow::{Result, anyhow, bail};

use crate::resp::RespDataType;

#[derive(Debug, PartialEq)]
pub enum Command {
    Quit,
    Ping {
        message: Option<String>,
    },
    Get {
        key: String,
    },
    Set {
        key: String,
        value: String,
        expiry: Option<Instant>,
    },
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
            "set" => parse_set_command(&mut iter)?,
            _ => bail!("unknown command '{}'", cmd),
        };

        if iter.next().is_some() {
            bail!("wrong number of arguments for '{}' command", cmd);
        }
        Ok(command)
    }
}

fn parse_set_command<I: Iterator<Item = String>>(iter: &mut I) -> Result<Command> {
    let key = ensure_next_arg(iter, "set")?;
    let value = ensure_next_arg(iter, "set")?;
    let ttl_option = match iter.next() {
        Some(ttl_option) => ttl_option.to_ascii_lowercase(),
        None => {
            return Ok(Command::Set {
                key,
                value,
                expiry: None,
            });
        }
    };
    let ttl: u64 = ensure_next_arg(iter, "set")?
        .parse()
        .map_err(|_| anyhow!("syntax error"))?;
    let expiry = Instant::now()
        + match ttl_option.as_str() {
            "ex" => Duration::from_secs(ttl),
            "px" => Duration::from_millis(ttl),
            _ => bail!("syntax error"),
        };
    Ok(Command::Set {
        key,
        value,
        expiry: Some(expiry),
    })
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
            vec!["set", "key", "value", "not-ttl"],
            vec!["set", "key", "value", "ex"],
            vec!["set", "key", "value", "ex", "-10"],
            vec!["set", "key", "value", "ex", "NaN"],
            vec!["set", "key", "value", "px", "NaN"],
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
            vec!["set", "key", "value", "ex", "10"],
            vec!["set", "key", "value", "px", "10"],
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
                expiry: None,
            },
        ];

        for (input, expected_result) in inputs.into_iter().zip(expected_results.into_iter()) {
            assert_eq!(
                Command::try_from(RespDataType::from(input.as_slice())).unwrap(),
                expected_result
            );
        }
    }

    #[test]
    fn test_ttl() {
        let inputs = vec![
            vec!["set", "key", "value", "ex", "1"],
            vec!["set", "key", "value", "px", "1000"],
        ];

        for input in inputs {
            let now = Instant::now();
            let cmd = Command::try_from(RespDataType::from(input.as_slice())).unwrap();
            let (key, value, expiry) = match cmd {
                Command::Set { key, value, expiry } => (key, value, expiry.unwrap()),
                _ => panic!("expected Set command"),
            };

            assert_eq!(key, "key");
            assert_eq!(value, "value");
            // add 1ms of tolerance for test execution/parsing times
            assert!(
                expiry.duration_since(now).abs_diff(Duration::from_secs(1))
                    < Duration::from_millis(1)
            );
        }
    }
}
