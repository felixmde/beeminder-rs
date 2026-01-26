# beeminder-rs

A Cargo workspace with Rust tools for [Beeminder](https://www.beeminder.com/).

## Crates

| Crate | Description | Status |
|-------|-------------|--------|
| **beeminder** | Async Rust client library for the Beeminder API | Usable |
| **beeline** | CLI for Beeminder (list, add, edit, backup, goal ops, batch, danger actions) | Usable |
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

Requires a Beeminder API key. You can supply it via the config file (recommended), an env var, or a command.

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

# Create a goal
beeline goal-create reading "Reading" hustler --goalval 10 --rate 1 --runits w --gunits pages

# For most goal types, Beeminder requires exactly two of: --goalval, --rate, --goaldate
# Goal units are also required: --gunits

# Update a goal
beeline goal-update reading --title "Reading (books)" --rate 2

# Refresh a goal's graph (autodata refetch)
beeline goal-refresh reading

# Add multiple datapoints from a JSON array (file or stdin)
beeline add-batch reading datapoints.json
cat datapoints.json | beeline add-batch reading -

# Danger actions
beeline shortcircuit reading
beeline stepdown reading
beeline cancel-stepdown reading
```

Example batch file format (`datapoints.json`):

```json
[
  { "value": 1.0, "comment": "Chapter 1" },
  { "value": 2.0, "timestamp": 1735689600 }
]
```

Config file examples (stored in your standard OS config location for `beeminder`):

```toml
api_key = "YOUR_KEY"

# or
api_key = { env = "BEEMINDER_API_KEY" }

# or
api_key = { cmd = "cat ~/.beeminder_key" }
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
