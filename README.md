# beeminder-rs

A Cargo workspace with Rust tools for [Beeminder](https://www.beeminder.com/).

## Crates

| Crate | Description | Status |
|-------|-------------|--------|
| **beeminder** | Async Rust client library for the Beeminder API | Usable |
| **beeline** | CLI for Beeminder (list, add, edit, backup) | Usable |
| **beetui** | TUI dashboard | Coming soon |
| **beemcp** | MCP server for AI assistants | Coming soon |

## Installation

### beeline CLI

```bash
cargo install --git https://github.com/felixmde/beeminder-rs beeline
```

### beeminder library

Add to your `Cargo.toml`:
```toml
[dependencies]
beeminder = { git = "https://github.com/felixmde/beeminder-rs" }
```

## Usage

### beeline CLI

Requires the `BEEMINDER_API_KEY` environment variable.

```bash
# List all goals (sorted by urgency, shows today's entries)
beeline list

# Add a datapoint
beeline add meditation 1
beeline add pushups 25 "morning set"

# Edit recent datapoints for a goal (opens in $EDITOR)
beeline edit meditation

# Backup all user data to JSON
beeline backup
beeline backup mybackup.json
```

### beeminder library

```rust
use beeminder::{BeeminderClient, types::CreateDatapoint};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = BeeminderClient::new(std::env::var("BEEMINDER_API_KEY")?);

    // Create a datapoint
    let datapoint = CreateDatapoint::new(42.0)
        .with_comment("Meditation session");
    client.create_datapoint("meditation", &datapoint).await?;

    // Fetch recent datapoints
    let datapoints = client
        .get_datapoints("meditation", None, Some(10), None, None)
        .await?;

    Ok(())
}
```

## Requirements

- Valid Beeminder API key (get yours at https://www.beeminder.com/api/v1/auth_token.json)

## License

MIT
