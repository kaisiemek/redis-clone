use crate::{
    kvstore::{KVStore, commands::Command},
    resp::RespData,
};

mod generic;
mod server;
mod string;

impl KVStore {
    pub fn handle_command(&mut self, command: Command) -> RespData {
        match command {
            Command::Shutdown => self.shutdown(),
            Command::Ping { message } => Self::ping(message),
            Command::Echo { message } => Self::echo(message),
            Command::Ttl { key } => self.ttl(&key),
            Command::Pttl { key } => self.pttl(&key),
            // generic commands
            Command::Del { keys } => self.del(&keys),
            Command::Exists { keys } => self.exists(&keys),
            // string commands
            Command::Append { key, value } => self.append(key, value),
            Command::Decr { key } => self.decr(key),
            Command::Decrby { key, operand } => self.decrby(key, operand),
            Command::Get { key } => self.get(&key),
            Command::Getset { key, value } => self.getset(key, value),
            Command::Incr { key } => self.incr(key),
            Command::Incrby { key, operand } => self.incrby(key, operand),
            Command::Mget { keys } => self.mget(keys),
            Command::Mset { keys, values } => self.mset(keys, values),
            Command::Msetnx { keys, values } => self.msetnx(keys, values),
            Command::Set { key, value, expiry } => self.set(key, value, expiry),
            Command::Setnx { key, value } => self.setnx(key, value),
            Command::Substring { key, begin, end } => self.substring(key, begin, end),
        }
    }
}
