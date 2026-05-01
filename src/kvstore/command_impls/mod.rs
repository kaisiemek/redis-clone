mod generic;
mod helpers;
mod list;
mod server;
mod string;

use crate::{
    kvstore::{KVStore, commands::Command},
    resp::RespData,
};

impl KVStore {
    pub fn handle_command(&mut self, command: Command) -> RespData {
        log::debug!("[kvstore] running command: {:?}", command);
        match command {
            Command::Shutdown => self.shutdown(),
            Command::Ping { message } => Self::ping(message),
            Command::Echo { message } => Self::echo(message),
            Command::Ttl { key } => self.ttl(&key),
            Command::Pttl { key } => self.pttl(&key),
            // generic commands
            Command::Del { keys } => self.del(&keys),
            Command::Exists { keys } => self.exists(&keys),
            Command::Expire { key, ttl } => self.expire(key, ttl),
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
            // line commands
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
        }
    }
}
