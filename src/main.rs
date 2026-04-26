mod application;
mod kvstore;
mod network;
mod resp;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    log4rs::init_file("config/log4rs.yaml", Default::default())?;
    application::start().await
}
