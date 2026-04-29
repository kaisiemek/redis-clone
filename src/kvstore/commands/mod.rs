pub mod parser;

use std::time::Instant;

#[derive(Debug, PartialEq)]
pub enum Command {
    Shutdown,
    Ping {
        message: Option<String>,
    },
    Echo {
        message: String,
    },
    Ttl {
        key: String,
    },
    Pttl {
        key: String,
    },
    // generic commands
    Del {
        keys: Vec<String>,
    },
    Exists {
        keys: Vec<String>,
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
    Getset {
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
    Mget {
        keys: Vec<String>,
    },
    Mset {
        keys: Vec<String>,
        values: Vec<String>,
    },
    Msetnx {
        keys: Vec<String>,
        values: Vec<String>,
    },
    Set {
        key: String,
        value: String,
        expiry: Option<Instant>,
    },
    Setnx {
        key: String,
        value: String,
    },
    Substring {
        key: String,
        begin: i64,
        end: i64,
    },
}
