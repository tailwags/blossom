use anyhow::Result;
use tracing::info;

pub fn info(name: &str) -> Result<()> {
    info!("Retrieving info for package: {}", name);
    Ok(())
}
