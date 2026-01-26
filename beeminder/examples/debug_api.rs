use beeminder::types::{DatapointFull, GoalFull, UserInfo, UserInfoDiff};
use reqwest::Client;
use std::env;
use time::{Duration, OffsetDateTime};

/// Attempts to parse JSON and shows error info on failure
fn try_parse<T: serde::de::DeserializeOwned>(json: &str, type_name: &str) {
    println!("\n>>> Attempting to parse as {type_name} <<<");
    match serde_json::from_str::<T>(json) {
        Ok(_) => println!("SUCCESS: Parsed {type_name} correctly"),
        Err(e) => println!("FAILED: {e}"),
    }
}

#[tokio::main]
async fn main() {
    let api_key =
        env::var("BEEMINDER_API_KEY").expect("BEEMINDER_API_KEY environment variable not set");

    let client = Client::new();
    let base_url = "https://www.beeminder.com/api/v1";

    // Test 1: Get user info
    println!("\n{}", "=".repeat(60));
    println!("TEST 1: GET /users/me.json");
    println!("{}", "=".repeat(60));

    let url = format!("{base_url}/users/me.json?auth_token={api_key}");
    match client.get(&url).send().await {
        Ok(resp) => {
            let text = resp.text().await.unwrap();
            let len = text.len();
            println!("Response length: {len} bytes");
            try_parse::<UserInfo>(&text, "UserInfo");
        }
        Err(e) => println!("Request failed: {e}"),
    }

    // Test 2: Get user diff
    println!("\n{}", "=".repeat(60));
    println!("TEST 2: GET /users/me.json?diff_since=...");
    println!("{}", "=".repeat(60));

    let since = OffsetDateTime::now_utc() - Duration::days(2);
    let diff_since = since.unix_timestamp();
    let url = format!("{base_url}/users/me.json?auth_token={api_key}&diff_since={diff_since}");
    match client.get(&url).send().await {
        Ok(resp) => {
            let text = resp.text().await.unwrap();
            let len = text.len();
            println!("Response length: {len} bytes");
            try_parse::<UserInfoDiff>(&text, "UserInfoDiff");
        }
        Err(e) => println!("Request failed: {e}"),
    }

    // Test 3: Get datapoints for a goal
    println!("\n{}", "=".repeat(60));
    println!("TEST 3: GET /users/me/goals/pushups/datapoints.json");
    println!("{}", "=".repeat(60));

    let url = format!(
        "{base_url}/users/me/goals/pushups/datapoints.json?auth_token={api_key}&count=5"
    );
    match client.get(&url).send().await {
        Ok(resp) => {
            let text = resp.text().await.unwrap();
            let len = text.len();
            println!("Response length: {len} bytes");
            try_parse::<Vec<DatapointFull>>(&text, "Vec<DatapointFull>");
        }
        Err(e) => println!("Request failed: {e}"),
    }

    // Test 4: Get single goal
    println!("\n{}", "=".repeat(60));
    println!("TEST 4: GET /users/me/goals/pushups.json");
    println!("{}", "=".repeat(60));

    let url = format!("{base_url}/users/me/goals/pushups.json?auth_token={api_key}");
    match client.get(&url).send().await {
        Ok(resp) => {
            let text = resp.text().await.unwrap();
            let len = text.len();
            println!("Response length: {len} bytes");
            try_parse::<GoalFull>(&text, "GoalFull");
        }
        Err(e) => println!("Request failed: {e}"),
    }
}
