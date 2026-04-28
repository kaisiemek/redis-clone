mod generic;
mod server;
mod string;

use std::time::{Duration, Instant};

use anyhow::{Result, anyhow, bail};

use crate::resp::RespDataType;

#[derive(Debug, PartialEq)]
pub enum Command {
    Shutdown,
    Ping {
        message: Option<String>,
    },
    Echo {
        message: String,
    },
    Del {
        keys: Vec<String>,
    },
    Ttl {
        key: String,
    },
    Pttl {
        key: String,
    },
    // string commands
    Append {
        key: String,
        value: String,
    },
    Decr {
        key: String,
    },
    Decrby {
        key: String,
        operand: i64,
    },
    Get {
        key: String,
    },
    GetSet {
        key: String,
        value: String,
    },
    Incr {
        key: String,
    },
    Incrby {
        key: String,
        operand: i64,
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
            .ok_or(anyhow!("ERR received an empty command"))?;

        let command = match cmd.as_str() {
            "shutdown" => Command::Shutdown,
            "ping" => Command::Ping {
                message: iter.next(),
            },
            "echo" => Command::Echo {
                message: ensure_next_arg(&mut iter, &cmd)?,
            },
            "del" => parse_del_command(&mut iter)?,
            "ttl" => Command::Ttl {
                key: ensure_next_arg(&mut iter, &cmd)?,
            },
            "pttl" => Command::Pttl {
                key: ensure_next_arg(&mut iter, &cmd)?,
            },
            // string commands
            "append" => Command::Append {
                key: ensure_next_arg(&mut iter, &cmd)?,
                value: ensure_next_arg(&mut iter, &cmd)?,
            },
            "decr" => Command::Decr {
                key: ensure_next_arg(&mut iter, &cmd)?,
            },
            "decrby" => Command::Decrby {
                key: ensure_next_arg(&mut iter, &cmd)?,
                operand: ensure_next_arg(&mut iter, &cmd)?
                    .parse()
                    .map_err(|_| anyhow!("ERR value is not an integer or out of range"))?,
            },
            "get" => Command::Get {
                key: ensure_next_arg(&mut iter, &cmd)?,
            },
            "getset" => Command::GetSet {
                key: ensure_next_arg(&mut iter, &cmd)?,
                value: ensure_next_arg(&mut iter, &cmd)?,
            },
            "incr" => Command::Incr {
                key: ensure_next_arg(&mut iter, &cmd)?,
            },
            "incrby" => Command::Incrby {
                key: ensure_next_arg(&mut iter, &cmd)?,
                operand: ensure_next_arg(&mut iter, &cmd)?
                    .parse()
                    .map_err(|_| anyhow!("ERR value is not an integer or out of range"))?,
            },
            "set" => parse_set_command(&mut iter)?,
            _ => bail!("ERR unknown command '{}'", cmd),
        };

        if iter.next().is_some() {
            bail!("ERR wrong number of arguments for '{}' command", cmd);
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
        .map_err(|_| anyhow!("ERR syntax error"))?;
    let expiry = Instant::now()
        + match ttl_option.as_str() {
            "ex" => Duration::from_secs(ttl),
            "px" => Duration::from_millis(ttl),
            _ => bail!("ERR syntax error"),
        };
    Ok(Command::Set {
        key,
        value,
        expiry: Some(expiry),
    })
}

fn parse_del_command<I: Iterator<Item = String>>(iter: &mut I) -> Result<Command> {
    let keys: Vec<String> = iter.collect();
    // need at least one key
    if keys.is_empty() {
        bail!("ERR wrong number of arguments for 'del' command");
    }
    Ok(Command::Del { keys })
}

fn ensure_next_arg<I: Iterator<Item = String>>(iter: &mut I, command: &str) -> Result<String> {
    iter.next().ok_or(anyhow!(
        "ERR wrong number of arguments for '{}' command",
        command
    ))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_invalid_command_parsing() {
        let inputs = vec![
            vec!["unknown-cmd"],
            vec!["shutdown", "arg"],
            vec!["ping", "message", "too many"],
            vec!["echo"],
            vec!["echo", "message", "too many"],
            vec!["ttl", "key", "too many"],
            vec!["ttl"],
            vec!["pttl"],
            vec!["del"],
            vec!["append", "key"],
            vec!["append", "key", "value", "toomany"],
            vec!["get", "key", "too many"],
            vec!["decrby", "key", "xx"],
            vec!["get"],
            vec!["incrby", "key", "xx"],
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
            vec!["shutdown"],
            vec!["ping"],
            vec!["ping", "test"],
            vec!["echo", "test"],
            vec!["ttl", "key"],
            vec!["pttl", "key"],
            vec!["del", "key"],
            vec!["del", "1", "2", "3"],
            vec!["append", "key", "value"],
            vec!["decrby", "key", "10"],
            vec!["get", "key"],
            vec!["getset", "key", "value"],
            vec!["incrby", "key", "10"],
            vec!["set", "key", "value"],
        ];
        let expected_results = vec![
            Command::Shutdown,
            Command::Ping { message: None },
            Command::Ping {
                message: Some(String::from("test")),
            },
            Command::Echo {
                message: String::from("test"),
            },
            Command::Ttl {
                key: String::from("key"),
            },
            Command::Pttl {
                key: String::from("key"),
            },
            Command::Del {
                keys: vec![String::from("key")],
            },
            Command::Del {
                keys: vec![String::from("1"), String::from("2"), String::from("3")],
            },
            Command::Append {
                key: String::from("key"),
                value: String::from("value"),
            },
            Command::Decrby {
                key: String::from("key"),
                operand: 10,
            },
            Command::Get {
                key: String::from("key"),
            },
            Command::GetSet {
                key: String::from("key"),
                value: String::from("value"),
            },
            Command::Incrby {
                key: String::from("key"),
                operand: 10,
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
