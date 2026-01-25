use beeminder::types::{CreateDatapoint, UpdateDatapoint};
use beeminder::BeeminderClient;
use std::env;
use time::{Duration, OffsetDateTime};

#[tokio::main]
async fn main() {
    let api_key =
        env::var("BEEMINDER_API_KEY").expect("BEEMINDER_API_KEY environment variable not set");

    let client = BeeminderClient::new(api_key);
    match client.get_user().await {
        Ok(user) => println!("{user:#?}"),
        Err(e) => println!("{e:#?}"),
    }

    let since = OffsetDateTime::now_utc() - Duration::days(2);
    match client.get_user_diff(since).await {
        Ok(user) => println!("{user:#?}"),
        Err(e) => println!("{e:#?}"),
    }

    let new_datapoint = CreateDatapoint::new(20.0)
        .with_comment("I did some pushups!")
        .with_requestid("unique-pushup-id-42");
    match client.create_datapoint("pushups", &new_datapoint).await {
        Ok(datapoint) => println!("Added: {datapoint:#?}"),
        Err(e) => println!("{e:#?}"),
    }

    let goal_name = "pushups";
    match client
        .get_datapoints(goal_name, None, Some(3), None, None)
        .await
    {
        Ok(datapoints) => {
            if let Some(first_datapoint) = datapoints.first() {
                let update_datapoint = UpdateDatapoint::from(first_datapoint)
                    .with_value(40.0)
                    .with_comment("Much better.");

                match client.update_datapoint(goal_name, &update_datapoint).await {
                    Ok(datapoint) => println!("Updated: {datapoint:#?}"),
                    Err(e) => println!("Update error: {e:#?}"),
                }

                match client
                    .delete_datapoint(goal_name, &update_datapoint.id)
                    .await
                {
                    Ok(datapoint) => println!("Deleted: {datapoint:#?}"),
                    Err(e) => println!("Delete error: {e:#?}"),
                }
            } else {
                println!("No datapoints found");
            }
        }
        Err(e) => println!("Get datapoints error: {e:#?}"),
    }
}
