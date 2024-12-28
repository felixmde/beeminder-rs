# beeminder-rs

An incomplete Rust client library for the [Beeminder](https://www.beeminder.com/) API.

Why use `curl` if you can just use `serde` and `reqwest` to have 200 dependencies?

You'll find that I add endpoints as I need them. Feel free to create an issue if 
you need a specific endpoint.

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

    // username defaults to 'me'; use `with_username` to change it
    // let client = BeeminderClient::new("api-key").with_username("foo");
    
    // Create a datapoint
    let datapoint = CreateDatapoint::new(42.0)
        .with_timestamp(OffsetDateTime::now_utc())
        .with_comment("Meditation session");
        
    client.create_datapoint("meditation", &datapoint).await?;
    
    // Fetch recent datapoints
    let datapoints = client
        .get_datapoints("meditation", Some("timestamp"), Some(10))
        .await?;
        
    Ok(())
}
```

## Requirements

- Valid Beeminder API key

## License

MIT

