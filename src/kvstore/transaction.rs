use anyhow::anyhow;

use crate::{
    kvstore::{KVStore, commands::Command},
    resp::RespData,
};

pub struct Transaction {
    aborted: bool,
    queued_commands: Vec<Command>,
}

impl Transaction {
    pub fn new() -> Self {
        Self {
            aborted: false,
            queued_commands: Vec::new(),
        }
    }

    pub fn abort(&mut self) {
        self.aborted = true;
    }

    pub fn execute(self, kvstore: &mut KVStore) -> RespData {
        if self.aborted {
            return anyhow!("EXECABORT Transaction discarded because of previous errors.").into();
        }

        let mut replies = Vec::new();
        for command in self.queued_commands {
            replies.push(kvstore.run_command(command));
        }

        replies.into()
    }

    pub fn queue_command(&mut self, command: Command) -> RespData {
        self.queued_commands.push(command);
        RespData::SimpleString(String::from("QUEUED"))
    }
}
