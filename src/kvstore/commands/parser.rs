use anyhow::{Result, anyhow, bail};

use crate::kvstore::commands::Command;

impl TryFrom<Vec<String>> for Command {
    type Error = anyhow::Error;

    fn try_from(value: Vec<String>) -> Result<Command> {
        parse_command(value)
    }
}

pub fn parse_command(argv: Vec<String>) -> Result<Command> {
    let mut iter = argv.into_iter();
    let cmd = iter
        .next()
        .ok_or(anyhow!("ERR received an empty command"))?;

    let command = match cmd.as_str() {
        // server commands
        "dbsize" => Command::Dbsize,
        "echo" => Command::Echo {
            message: ensure_next_arg(&mut iter, &cmd)?,
        },
        "flushdb" => Command::Flushdb,
        "ping" => Command::Ping {
            message: iter.next(),
        },
        "save" => Command::Save,
        "shutdown" => Command::Shutdown,
        // generic commands
        "del" => Command::Del {
            keys: ensure_arg_list(&mut iter, &cmd)?,
        },
        "exists" => Command::Exists {
            keys: ensure_arg_list(&mut iter, &cmd)?,
        },
        "expire" => Command::Expire {
            key: ensure_next_arg(&mut iter, &cmd)?,
            ttl: ensure_integer_arg(&mut iter, &cmd)?,
        },
        "rename" => Command::Rename {
            key: ensure_next_arg(&mut iter, &cmd)?,
            newkey: ensure_next_arg(&mut iter, &cmd)?,
        },
        "renamenx" => Command::Renamenx {
            key: ensure_next_arg(&mut iter, &cmd)?,
            newkey: ensure_next_arg(&mut iter, &cmd)?,
        },
        "ttl" => Command::Ttl {
            key: ensure_next_arg(&mut iter, &cmd)?,
        },
        "type" => Command::Type {
            key: ensure_next_arg(&mut iter, &cmd)?,
        },
        // string commands
        "decr" => Command::Decr {
            key: ensure_next_arg(&mut iter, &cmd)?,
        },
        "decrby" => Command::Decrby {
            key: ensure_next_arg(&mut iter, &cmd)?,
            operand: ensure_integer_arg(&mut iter, &cmd)?,
        },
        "get" => Command::Get {
            key: ensure_next_arg(&mut iter, &cmd)?,
        },
        "getset" => Command::Getset {
            key: ensure_next_arg(&mut iter, &cmd)?,
            value: ensure_next_arg(&mut iter, &cmd)?,
        },
        "incr" => Command::Incr {
            key: ensure_next_arg(&mut iter, &cmd)?,
        },
        "incrby" => Command::Incrby {
            key: ensure_next_arg(&mut iter, &cmd)?,
            operand: ensure_integer_arg(&mut iter, &cmd)?,
        },
        "mget" => Command::Mget {
            keys: ensure_arg_list(&mut iter, &cmd)?,
        },
        "mset" => {
            let (keys, values) = ensure_key_val_list(&mut iter, &cmd)?;
            Command::Mset { keys, values }
        }
        "msetnx" => {
            let (keys, values) = ensure_key_val_list(&mut iter, &cmd)?;
            Command::Msetnx { keys, values }
        }
        "set" => Command::Set {
            key: ensure_next_arg(&mut iter, &cmd)?,
            value: ensure_next_arg(&mut iter, &cmd)?,
        },
        "setnx" => Command::Setnx {
            key: ensure_next_arg(&mut iter, &cmd)?,
            value: ensure_next_arg(&mut iter, &cmd)?,
        },
        "substring" => Command::Substring {
            key: ensure_next_arg(&mut iter, &cmd)?,
            begin: ensure_integer_arg(&mut iter, &cmd)?,
            end: ensure_integer_arg(&mut iter, &cmd)?,
        },
        // list operations
        "lindex" => Command::Lindex {
            key: ensure_next_arg(&mut iter, &cmd)?,
            index: ensure_integer_arg(&mut iter, &cmd)?,
        },
        "llen" => Command::Llen {
            key: ensure_next_arg(&mut iter, &cmd)?,
        },
        "lpop" => Command::Lpop {
            key: ensure_next_arg(&mut iter, &cmd)?,
        },
        "lpush" => Command::Lpush {
            key: ensure_next_arg(&mut iter, &cmd)?,
            values: ensure_arg_list(&mut iter, &cmd)?,
        },
        "lrange" => Command::Lrange {
            key: ensure_next_arg(&mut iter, &cmd)?,
            begin: ensure_integer_arg(&mut iter, &cmd)?,
            end: ensure_integer_arg(&mut iter, &cmd)?,
        },
        "lrem" => Command::Lrem {
            key: ensure_next_arg(&mut iter, &cmd)?,
            count: ensure_integer_arg(&mut iter, &cmd)?,
            element: ensure_next_arg(&mut iter, &cmd)?,
        },
        "lset" => Command::Lset {
            key: ensure_next_arg(&mut iter, &cmd)?,
            index: ensure_integer_arg(&mut iter, &cmd)?,
            element: ensure_next_arg(&mut iter, &cmd)?,
        },
        "ltrim" => Command::Ltrim {
            key: ensure_next_arg(&mut iter, &cmd)?,
            begin: ensure_integer_arg(&mut iter, &cmd)?,
            end: ensure_integer_arg(&mut iter, &cmd)?,
        },
        "rpop" => Command::Rpop {
            key: ensure_next_arg(&mut iter, &cmd)?,
        },
        "rpush" => Command::Rpush {
            key: ensure_next_arg(&mut iter, &cmd)?,
            values: ensure_arg_list(&mut iter, &cmd)?,
        },
        // transaction commands
        "exec" => Command::Exec,
        "multi" => Command::Multi,
        _ => bail!("ERR unknown command '{}'", cmd),
    };

    if iter.next().is_some() {
        bail!("ERR wrong number of arguments for '{}' command", cmd);
    }
    Ok(command)
}

fn ensure_next_arg<I: Iterator<Item = String>>(iter: &mut I, command: &str) -> Result<String> {
    iter.next().ok_or(anyhow!(
        "ERR wrong number of arguments for '{}' command",
        command
    ))
}

fn ensure_integer_arg<I: Iterator<Item = String>>(iter: &mut I, command: &str) -> Result<i64> {
    ensure_next_arg(iter, command)?
        .parse()
        .map_err(|_| anyhow!("ERR value is not an integer or out of range"))
}

fn ensure_arg_list<I: Iterator<Item = String>>(iter: &mut I, command: &str) -> Result<Vec<String>> {
    let args: Vec<String> = iter.collect();
    // need at least one key
    if args.is_empty() {
        bail!("ERR wrong number of arguments for '{}' command", command);
    }
    Ok(args)
}

fn ensure_key_val_list<I: Iterator<Item = String>>(
    iter: &mut I,
    command: &str,
) -> Result<(Vec<String>, Vec<String>)> {
    let mut keys = Vec::new();
    let mut vals = Vec::new();
    while let Some(key) = iter.next() {
        let val = ensure_next_arg(iter, command)?;
        keys.push(key);
        vals.push(val);
    }
    if keys.is_empty() || vals.is_empty() {
        bail!("ERR wrong number of arguments for '{}' command", command);
    }
    Ok((keys, vals))
}

#[cfg(test)]
mod test {
    use super::*;

    fn make_argv(argv: Vec<&str>) -> Vec<String> {
        argv.iter().map(|arg| arg.to_string()).collect()
    }

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
            vec!["get", "key", "too many"],
            vec!["decrby", "key", "xx"],
            vec!["get"],
            vec!["incrby", "key", "xx"],
            vec!["set"],
            vec!["set", "key"],
            vec!["set", "key", "value", "too many"],
            vec!["mset", "key", "value", "key1"],
            vec!["mset", "key"],
        ];
        for input in inputs {
            Command::try_from(make_argv(input)).unwrap_err();
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
            vec!["del", "key"],
            vec!["del", "1", "2", "3"],
            vec!["decrby", "key", "10"],
            vec!["get", "key"],
            vec!["getset", "key", "value"],
            vec!["incrby", "key", "10"],
            vec!["mget", "1", "2"],
            vec!["mset", "k1", "v1", "k2", "v2"],
            vec!["msetnx", "k1", "v1"],
            vec!["set", "key", "value"],
            vec!["substring", "key", "0", "-1"],
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
            Command::Del {
                keys: vec![String::from("key")],
            },
            Command::Del {
                keys: vec![String::from("1"), String::from("2"), String::from("3")],
            },
            Command::Decrby {
                key: String::from("key"),
                operand: 10,
            },
            Command::Get {
                key: String::from("key"),
            },
            Command::Getset {
                key: String::from("key"),
                value: String::from("value"),
            },
            Command::Incrby {
                key: String::from("key"),
                operand: 10,
            },
            Command::Mget {
                keys: vec![String::from("1"), String::from("2")],
            },
            Command::Mset {
                keys: vec!["k1".into(), "k2".into()],
                values: vec!["v1".into(), "v2".into()],
            },
            Command::Msetnx {
                keys: vec!["k1".into()],
                values: vec!["v1".into()],
            },
            Command::Set {
                key: String::from("key"),
                value: String::from("value"),
            },
            Command::Substring {
                key: String::from("key"),
                begin: 0,
                end: -1,
            },
        ];

        for (input, expected_result) in inputs.into_iter().zip(expected_results) {
            assert_eq!(
                Command::try_from(make_argv(input)).unwrap(),
                expected_result
            );
        }
    }
}
