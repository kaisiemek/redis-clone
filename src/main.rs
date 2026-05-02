mod application;
mod kvstore;
mod network;
mod resp;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    log4rs::init_file("config/log4rs.yaml", Default::default())?;
    application::start().await
}
