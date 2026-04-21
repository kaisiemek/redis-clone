use anyhow::Result;

fn main() -> Result<()> {
    log4rs::init_file("config/log4rs.yaml", Default::default())?;
    log::info!("server starting...");
    Ok(())
}
