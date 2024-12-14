# beeminder-rs

A Rust client library for the [Beeminder](https://www.beeminder.com/) API.

## Installation

Add to your `Cargo.toml`:
```toml
[dependencies]
beeminder-rs = "0.1.0"
```

## Usage

```rust
use beeminder::{BeeminderClient, types::CreateDatapoint};
use time::OffsetDateTime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = BeeminderClient::new(std::env::var("BEEMINDER_API_KEY")?);
    
    // Create a datapoint
    let datapoint = CreateDatapoint::new(42.0)
        .with_timestamp(OffsetDateTime::now_utc())
        .with_comment("Meditation session");
        
    client.create_datapoint("username", "meditation", &datapoint).await?;
    
    // Fetch recent datapoints
    let datapoints = client
        .get_datapoints("username", "meditation", Some("timestamp"), Some(10))
        .await?;
        
    Ok(())
}
```

## Requirements

- Valid Beeminder API key

## License

MIT

## Contributing

Pull requests welcome!
