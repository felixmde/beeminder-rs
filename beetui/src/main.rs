use anyhow::{Context, Result};
use beeconfig::BeeConfig;

fn main() -> Result<()> {
    let _config =
        BeeConfig::load_or_onboard().with_context(|| "Failed to load beeminder config")?;
    println!("beetui: TUI dashboard (not yet implemented)");
    Ok(())
}
