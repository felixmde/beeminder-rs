use anyhow::{Context, Result};
use beeconfig::BeeConfig;

fn main() -> Result<()> {
    let _config =
        BeeConfig::load_or_onboard().with_context(|| "Failed to load beeminder config")?;
    println!("beemcp: MCP server (not yet implemented)");
    Ok(())
}
