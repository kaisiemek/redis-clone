use anyhow::anyhow;

use crate::{
    kvstore::{KVStore, transaction::Transaction},
    resp::RespData,
};

impl KVStore {
    pub(in crate::kvstore::commands) fn exec(&mut self) -> RespData {
        let Some(current_client) = self.current_client else {
            return anyhow!("ERR couldn't detect current client to EXEC a MULTI transaction")
                .into();
        };
        match self.transactions.remove(&current_client) {
            Some(transaction) => transaction.execute(self),
            None => anyhow!("ERR EXEC without MULTI").into(),
        }
    }

    pub(in crate::kvstore::commands) fn multi(&mut self) -> RespData {
        let Some(current_client) = self.current_client else {
            return anyhow!("ERR couldn't detect current client to start a MULTI transaction")
                .into();
        };
        if self.transactions.contains_key(&current_client) {
            return anyhow!("ERR MULTI calls can not be nested").into();
        }
        self.transactions.insert(current_client, Transaction::new());
        RespData::ok()
    }
}
