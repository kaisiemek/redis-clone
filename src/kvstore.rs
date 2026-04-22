use anyhow::Result;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub enum Command {
    Quit,
}

#[derive(Debug)]
pub struct Event {
    pub command: Command,
    pub reply_channel: oneshot::Sender<Result<String>>,
}

pub struct KVStore {
    event_channel: mpsc::UnboundedReceiver<Event>,
    cancellation_token: CancellationToken,
}

impl KVStore {
    pub fn new(
        event_channel: mpsc::UnboundedReceiver<Event>,
        cancellation_token: CancellationToken,
    ) -> Self {
        KVStore {
            event_channel,
            cancellation_token,
        }
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        log::info!("running event loop");
        loop {
            tokio::select! {
                Some(event) = self.event_channel.recv() => {
                    self.handle_event(event);
                }
                _ = self.cancellation_token.cancelled() => {
                        break;
                }
            }
        }
        log::info!("event loop has finished");
        Ok(())
    }

    fn handle_event(&self, event: Event) {
        log::debug!("handling event {:?}", event.command);
        let reply = match event.command {
            Command::Quit => {
                self.cancellation_token.cancel();
                String::new()
            }
        };
        if event.reply_channel.send(Ok(reply)).is_err() {
            log::error!("couldn't reply to the event!");
        }
    }
}
