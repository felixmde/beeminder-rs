mod common;

use beeminder::types::{CreateDatapoint, CreateGoal, UpdateDatapoint, UpdateGoal};
use common::mock_server::BeeminderMock;
use time::OffsetDateTime;

async fn recorded_mock(fixture: &str) -> BeeminderMock {
    let mock = BeeminderMock::start().await;
    mock.mount_fixture_in("recorded", fixture).await;
    mock
}

#[tokio::test]
async fn recorded_get_user_valid() {
    let mock = recorded_mock("user/get_user_valid.json").await;
    let client = mock.client();
    let user = client.get_user().await.unwrap();
    assert!(!user.username.is_empty());
}

#[tokio::test]
async fn recorded_get_user_emaciated() {
    let mock = recorded_mock("user/get_user_emaciated.json").await;
    let client = mock.client().with_emaciated(true);
    let user = client.get_user().await.unwrap();
    assert!(!user.username.is_empty());
}

#[tokio::test]
async fn recorded_get_user_invalid_auth() {
    let mock = recorded_mock("user/get_user_invalid_auth.json").await;
    let client = mock.client();
    let result = client.get_user().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn recorded_get_auth_token_no_auth() {
    let mock = recorded_mock("auth/get_token_no_auth.json").await;
    let client = mock.client();
    let result = client.get_auth_token().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn recorded_get_user_diff_valid() {
    let mock = recorded_mock("user/get_user_diff_valid.json").await;
    let client = mock.client();
    let diff_since = OffsetDateTime::from_unix_timestamp(1_769_372_923).unwrap();
    let result = client.get_user_diff(diff_since).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn recorded_get_user_diff_no_changes() {
    let mock = recorded_mock("user/get_user_diff_no_changes.json").await;
    let client = mock.client();
    let diff_since = OffsetDateTime::from_unix_timestamp(1_769_376_660).unwrap();
    let result = client.get_user_diff(diff_since).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn recorded_get_goals_valid() {
    let mock = recorded_mock("goals/get_goals_valid.json").await;
    let client = mock.client();
    let goals = client.get_goals().await.unwrap();
    assert!(goals.iter().all(|goal| !goal.slug.is_empty()));
}

#[tokio::test]
async fn recorded_get_goals_emaciated() {
    let mock = recorded_mock("goals/get_goals_emaciated.json").await;
    let client = mock.client().with_emaciated(true);
    let goals = client.get_goals().await.unwrap();
    assert!(goals.iter().all(|goal| !goal.slug.is_empty()));
}

#[tokio::test]
async fn recorded_get_archived_goals() {
    let mock = recorded_mock("goals/get_archived_valid.json").await;
    let client = mock.client();
    let goals = client.get_archived_goals().await.unwrap();
    assert!(goals.iter().all(|goal| !goal.slug.is_empty()));
}

#[tokio::test]
async fn recorded_get_goal_valid() {
    let mock = recorded_mock("goals/get_goal_valid.json").await;
    let client = mock.client();
    let goal = client.get_goal("exercise", false).await.unwrap();
    assert!(!goal.slug.is_empty());
}

#[tokio::test]
async fn recorded_get_goal_emaciated() {
    let mock = recorded_mock("goals/get_goal_emaciated.json").await;
    let client = mock.client().with_emaciated(true);
    let goal = client.get_goal("exercise", false).await.unwrap();
    assert!(!goal.slug.is_empty());
}

#[tokio::test]
async fn recorded_get_goal_with_datapoints() {
    let mock = recorded_mock("goals/get_goal_with_datapoints.json").await;
    let client = mock.client();
    let goal = client.get_goal("exercise", true).await.unwrap();
    assert!(!goal.slug.is_empty());
}

#[tokio::test]
async fn recorded_get_goal_not_found() {
    let mock = recorded_mock("goals/get_goal_not_found.json").await;
    let client = mock.client();
    let result = client.get_goal("missing", false).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn recorded_create_goal_valid() {
    let mock = recorded_mock("goals/create_goal_valid.json").await;
    let client = mock.client();
    let mut goal = CreateGoal::new("apitest1769383656", "REDACTED", "hustler");
    goal.gunits = Some("REDACTED".to_string());
    goal.rate = Some(1.0);
    goal.goaldate = Some(OffsetDateTime::from_unix_timestamp(1_800_919_656).unwrap());
    let result = client.create_goal(&goal).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn recorded_create_goal_missing_required() {
    let mock = recorded_mock("goals/create_goal_missing_required.json").await;
    let client = mock.client();
    let goal = CreateGoal::new("incomplete", "API Test Goal", "hustler");
    let result = client.create_goal(&goal).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn recorded_update_goal_valid() {
    let mock = recorded_mock("goals/update_goal_valid.json").await;
    let client = mock.client();
    let mut update = UpdateGoal::new();
    update.title = Some("REDACTED".to_string());
    let result = client.update_goal("test", &update).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn recorded_refresh_graph_valid() {
    let mock = recorded_mock("goals/refresh_graph_valid.json").await;
    let client = mock.client();
    let result = client.refresh_graph("test").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn recorded_get_datapoints_valid() {
    let mock = recorded_mock("datapoints/get_datapoints_valid.json").await;
    let client = mock.client();
    let datapoints = client
        .get_datapoints("exercise", None, None, None, None)
        .await
        .unwrap();
    assert!(datapoints.iter().all(|dp| !dp.id.is_empty()));
}

#[tokio::test]
async fn recorded_get_datapoints_paginated() {
    let mock = recorded_mock("datapoints/get_datapoints_paginated.json").await;
    let client = mock.client();
    let datapoints = client
        .get_datapoints("exercise", None, None, Some(1), Some(5))
        .await
        .unwrap();
    assert!(datapoints.iter().all(|dp| !dp.id.is_empty()));
}

#[tokio::test]
async fn recorded_get_datapoints_sorted() {
    let mock = recorded_mock("datapoints/get_datapoints_sorted.json").await;
    let client = mock.client();
    let datapoints = client
        .get_datapoints("exercise", Some("daystamp"), Some(5), None, None)
        .await
        .unwrap();
    assert!(datapoints.iter().all(|dp| !dp.id.is_empty()));
}

#[tokio::test]
async fn recorded_get_datapoints_count() {
    let mock = recorded_mock("datapoints/get_datapoints_count.json").await;
    let client = mock.client();
    let datapoints = client
        .get_datapoints("exercise", None, Some(3), None, None)
        .await
        .unwrap();
    assert!(datapoints.iter().all(|dp| !dp.id.is_empty()));
}

#[tokio::test]
async fn recorded_create_datapoint_minimal() {
    let mock = recorded_mock("datapoints/create_datapoint_minimal.json").await;
    let client = mock.client();
    let datapoint = CreateDatapoint::new(1.0);
    let result = client.create_datapoint("exercise", &datapoint).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn recorded_create_datapoint_with_comment() {
    let mock = recorded_mock("datapoints/create_datapoint_with_comment.json").await;
    let client = mock.client();
    let datapoint = CreateDatapoint::new(2.0).with_comment("REDACTED");
    let result = client.create_datapoint("exercise", &datapoint).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn recorded_create_datapoint_with_daystamp() {
    let mock = recorded_mock("datapoints/create_datapoint_with_daystamp.json").await;
    let client = mock.client();
    let datapoint = CreateDatapoint::new(4.0).with_daystamp("20260120");
    let result = client.create_datapoint("exercise", &datapoint).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn recorded_create_datapoint_with_requestid() {
    let mock = recorded_mock("datapoints/create_datapoint_with_requestid.json").await;
    let client = mock.client();
    let datapoint = CreateDatapoint::new(3.0).with_requestid("unique-test-id-123");
    let result = client.create_datapoint("exercise", &datapoint).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn recorded_create_datapoint_duplicate_requestid() {
    let mock = recorded_mock("datapoints/create_datapoint_duplicate_requestid.json").await;
    let client = mock.client();
    let datapoint = CreateDatapoint::new(999.0).with_requestid("unique-test-id-123");
    let result = client.create_datapoint("exercise", &datapoint).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn recorded_create_all_datapoints_valid() {
    let mock = recorded_mock("datapoints/create_all_valid.json").await;
    let client = mock.client();
    let datapoints = vec![
        CreateDatapoint::new(10.0).with_comment("batch1"),
        CreateDatapoint::new(20.0).with_comment("batch2"),
    ];
    let result = client.create_all_datapoints("exercise", &datapoints).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn recorded_update_datapoint_valid() {
    let mock = recorded_mock("datapoints/update_datapoint_valid.json").await;
    let client = mock.client();
    let update = UpdateDatapoint::new("dp123")
        .with_value(42.0)
        .with_comment("REDACTED");
    let result = client.update_datapoint("exercise", &update).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn recorded_update_datapoint_not_found() {
    let mock = recorded_mock("datapoints/update_datapoint_not_found.json").await;
    let client = mock.client();
    let update = UpdateDatapoint::new("dp_missing");
    let result = client.update_datapoint("exercise", &update).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn recorded_delete_datapoint_valid() {
    let mock = recorded_mock("datapoints/delete_datapoint_valid.json").await;
    let client = mock.client();
    let result = client.delete_datapoint("exercise", "dp123").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn recorded_delete_datapoint_not_found() {
    let mock = recorded_mock("datapoints/delete_datapoint_not_found.json").await;
    let client = mock.client();
    let result = client.delete_datapoint("exercise", "dp_missing").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn recorded_shortcircuit_valid() {
    let mock = recorded_mock("danger/shortcircuit_valid.json").await;
    let client = mock.client();
    let result = client.shortcircuit("test").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn recorded_stepdown_valid() {
    let mock = recorded_mock("danger/stepdown_valid.json").await;
    let client = mock.client();
    let result = client.stepdown("free").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn recorded_stepdown_error() {
    let mock = recorded_mock("danger/stepdown_error.json").await;
    let client = mock.client();
    let result = client.stepdown("test").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn recorded_cancel_stepdown_valid() {
    let mock = recorded_mock("danger/cancel_stepdown_valid.json").await;
    let client = mock.client();
    let result = client.cancel_stepdown("free").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn recorded_cancel_stepdown_no_pending() {
    let mock = recorded_mock("danger/cancel_stepdown_no_pending.json").await;
    let client = mock.client();
    let result = client.cancel_stepdown("apitest1769383656").await;
    assert!(result.is_ok());
}
