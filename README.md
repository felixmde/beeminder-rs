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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = BeeminderClient::new(std::env::var("BEEMINDER_API_KEY")?);

    // username defaults to 'me'; use `with_username` to change it
    // let client = BeeminderClient::new("api-key").with_username("foo");

    // Create a datapoint (timestamp defaults to now)
    let datapoint = CreateDatapoint::new(42.0)
        .with_comment("Meditation session");

    client.create_datapoint("meditation", &datapoint).await?;

    // Fetch recent datapoints
    // Pagination available via page/per params
    let datapoints = client
        .get_datapoints("meditation", None, Some(10), None, None)
        .await?;

    Ok(())
}
```

## Requirements

- Valid Beeminder API key

## License

MIT

