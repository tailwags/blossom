use anyhow::Result;
use tracing::info;

pub fn uninstall(name: &str) -> Result<()> {
    info!("Removing package: {}", name);
    Ok(())
}
