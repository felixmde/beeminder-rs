use beeminder::types::CreateDatapoint;
use beeminder::BeeminderClient;
use std::env;
use time::macros::datetime;

#[tokio::main]
async fn main() {
    let api_key =
        env::var("BEEMINDER_API_KEY").expect("BEEMINDER_API_KEY environment variable not set");

    let client = BeeminderClient::new(api_key);
    match client.get_user("me").await {
        Ok(user) => println!("{user:#?}"),
        Err(e) => println!("{e:#?}"),
    }

    let since = datetime!(2024-12-13 20:00 UTC);
    match client.get_user_diff("me", since).await {
        Ok(user) => println!("{user:#?}"),
        Err(e) => println!("{e:#?}"),
    }

    match client
        .get_datapoints("me", "meditation", None, Some(10))
        .await
    {
        Ok(datapoints) => println!("{datapoints:#?}"),
        Err(e) => println!("{e:#?}"),
    }

    let d = CreateDatapoint::new(1.0)
        .with_comment("Test #hashtag datapoint")
        .with_requestid("unique-id-42");
    match client.create_datapoint("me", "meditation", &d).await {
        Ok(datapoint) => println!("Added: {datapoint:#?}"),
        Err(e) => println!("{e:#?}"),
    }
}
