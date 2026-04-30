use std::net::SocketAddr;

use tokio::sync::oneshot;

use crate::resp::RespData;

#[derive(Debug)]
pub struct Request {
    pub connection: SocketAddr,
    pub argv: Vec<String>,
    pub reply_channel: oneshot::Sender<String>,
}

impl Request {
    // move self into this function so it can be dropped after replying
    pub fn send_reply(self, reply: RespData) {
        if self.reply_channel.send(reply.encode()).is_err() {
            log::error!("[client {}] couldn't reply to the event!", self.connection);
        }
    }
}
