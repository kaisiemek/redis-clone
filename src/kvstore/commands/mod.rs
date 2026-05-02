mod impls;
pub mod parser;

use crate::{
    kvstore::{KVStore, commands::parser::parse_command},
    resp::RespData,
};

#[derive(Debug, PartialEq)]
pub enum Command {
    // server commands
    Echo {
        message: String,
    },
    Dbsize,
    Flushdb,
    Ping {
        message: Option<String>,
    },
    Shutdown,
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
    Ttl {
        key: String,
    },
    Pttl {
        key: String,
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
    // transaction commands
    Exec,
    Multi,
}

impl Command {
    pub fn is_queueable(&self) -> bool {
        match self {
            Self::Multi | Self::Exec => false,
            _ => true,
        }
    }
}

impl KVStore {
    pub(in crate::kvstore) fn process_command(&mut self, argv: Vec<String>) -> RespData {
        let current_transaction = self.get_current_transaction();
        let command = match parse_command(argv) {
            Ok(command) => command,
            Err(err) => {
                if let Some(transaction) = current_transaction {
                    transaction.abort();
                }
                return err.into();
            }
        };

        if let Some(transaction) = current_transaction
            && command.is_queueable()
        {
            transaction.queue_command(command)
        } else {
            self.run_command(command)
        }
    }

    pub(in crate::kvstore) fn run_command(&mut self, command: Command) -> RespData {
        log::debug!("[kvstore] running command: {:?}", command);
        match command {
            // server commands
            Command::Dbsize => self.dbsize(),
            Command::Echo { message } => Self::echo(message),
            Command::Flushdb => self.flushdb(),
            Command::Ping { message } => Self::ping(message),
            Command::Shutdown => self.shutdown(),
            // generic commands
            Command::Del { keys } => self.del(&keys),
            Command::Exists { keys } => self.exists(&keys),
            Command::Expire { key, ttl } => self.expire(key, ttl),
            Command::Ttl { key } => self.ttl(&key),
            Command::Pttl { key } => self.pttl(&key),
            // string commands
            Command::Decr { key } => self.decr(key),
            Command::Decrby { key, operand } => self.decrby(key, operand),
            Command::Get { key } => self.gets(key),
            Command::Getset { key, value } => self.getset(key, value),
            Command::Incr { key } => self.incr(key),
            Command::Incrby { key, operand } => self.incrby(key, operand),
            Command::Mget { keys } => self.mget(keys),
            Command::Mset { keys, values } => self.mset(keys, values),
            Command::Msetnx { keys, values } => self.msetnx(keys, values),
            Command::Set { key, value } => self.set(key, value),
            Command::Setnx { key, value } => self.setnx(key, value),
            Command::Substring { key, begin, end } => self.substring(key, begin, end),
            // list commands
            Command::Lindex { key, index } => self.lindex(key, index),
            Command::Llen { key } => self.llen(key),
            Command::Lpop { key } => self.lpop(key),
            Command::Lpush { key, values } => self.lpush(key, values),
            Command::Lrange { key, begin, end } => self.lrange(key, begin, end),
            Command::Lrem {
                key,
                count,
                element,
            } => self.lrem(key, count, element),
            Command::Lset {
                key,
                index,
                element,
            } => self.lset(key, index, element),
            Command::Ltrim { key, begin, end } => self.ltrim(key, begin, end),
            Command::Rpop { key } => self.rpop(key),
            Command::Rpush { key, values } => self.rpush(key, values),
            // transaction commands
            Command::Exec => self.exec(),
            Command::Multi => self.multi(),
        }
    }
}
