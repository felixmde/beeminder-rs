mod common;

use beeminder::types::{CreateGoal, UpdateGoal};
use common::mock_server::BeeminderMock;
use time::OffsetDateTime;

#[tokio::test]
async fn test_create_goal_valid() {
    let mock = BeeminderMock::start().await;
    mock.mount_fixture("goals/create_goal_valid.json").await;

    let client = mock.client();
    let mut goal = CreateGoal::new("testgoal", "Test Goal", "hustler");
    goal.gunits = Some("units".to_string());
    goal.rate = Some(1.0);
    goal.goaldate = Some(OffsetDateTime::from_unix_timestamp(1_705_000_000).unwrap());

    let result = client.create_goal(&goal).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_create_goal_missing_required() {
    let mock = BeeminderMock::start().await;
    mock.mount_fixture("goals/create_goal_missing_required.json")
        .await;

    let client = mock.client();
    let goal = CreateGoal::new("incomplete", "Test Goal", "hustler");
    let result = client.create_goal(&goal).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_update_goal_valid() {
    let mock = BeeminderMock::start().await;
    mock.mount_fixture("goals/update_goal_valid.json").await;

    let client = mock.client();
    let mut update = UpdateGoal::new();
    update.title = Some("Updated Title".to_string());

    let result = client.update_goal("testgoal", &update).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_refresh_graph_valid() {
    let mock = BeeminderMock::start().await;
    mock.mount_fixture("goals/refresh_graph_valid.json").await;

    let client = mock.client();
    let result = client.refresh_graph("testgoal").await;
    assert!(result.unwrap());
}
