pub mod parser;

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
    Expire {
        key: String,
        ttl: i64,
    },
    // string commands
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
    // list commands
    Lindex {
        key: String,
        index: i64,
    },
    Llen {
        key: String,
    },
    Lpop {
        key: String,
    },
    Lpush {
        key: String,
        values: Vec<String>,
    },
    Lrange {
        key: String,
        begin: i64,
        end: i64,
    },
    Lrem {
        key: String,
        count: i64,
        element: String,
    },
    Lset {
        key: String,
        index: i64,
        element: String,
    },
    Ltrim {
        key: String,
        begin: i64,
        end: i64,
    },
    Rpop {
        key: String,
    },
    Rpush {
        key: String,
        values: Vec<String>,
    },
}
