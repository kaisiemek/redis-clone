mod kvstore;
mod resp;
mod server;

use anyhow::Result;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<()> {
    log4rs::init_file("config/log4rs.yaml", Default::default())?;

    log::info!("application starting...");
    if let Err(err) = run_threads().await {
        log::error!(
            "an error occurred, forcing the application to quit: {}",
            err
        );
        Err(err)
    } else {
        log::info!("all threads finished gracefully, shutting down...");
        Ok(())
    }
}

async fn run_threads() -> Result<()> {
    let (event_tx, event_rx) = mpsc::unbounded_channel::<kvstore::Event>();
    let cancellation_token = CancellationToken::new();

    let server = server::Server::new(event_tx, cancellation_token.clone());
    let mut kvstore = kvstore::KVStore::new(event_rx, cancellation_token);

    let event_thread = tokio::spawn(async move { kvstore.run_event_loop().await });
    let server_thread = tokio::spawn(async move { server.run().await });
    let (server_res, event_res) = tokio::join!(server_thread, event_thread);

    server_res??;
    event_res?
}
