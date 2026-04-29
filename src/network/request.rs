use std::net::SocketAddr;

use tokio::sync::oneshot;

use crate::resp::RespData;

#[derive(Debug)]
pub struct Request {
    pub connection: SocketAddr,
    pub argv: Vec<String>,
    pub reply_buf: String,
    pub reply_channel: oneshot::Sender<String>,
}

impl Request {
    pub fn add_reply(&mut self, reply: RespData) {
        self.reply_buf = reply.encode_resp_data();
    }

    pub fn send_reply(self) {
        if self.reply_channel.send(self.reply_buf).is_err() {
            log::error!("[client {}] couldn't reply to the event!", self.connection);
        }
    }
}
