mod common;

use common::mock_server::BeeminderMock;

#[tokio::test]
async fn test_get_goal_valid() {
    let mock = BeeminderMock::start().await;
    mock.mount_fixture("goals/get_goal_valid.json").await;

    let client = mock.client();
    let goal = client.get_goal("exercise", false).await.unwrap();

    assert_eq!(goal.id, "abc123def456");
    assert_eq!(goal.slug, "exercise");
    assert!((goal.pledge - 5.0).abs() < f64::EPSILON);
    assert!(!goal.frozen);
}

#[tokio::test]
async fn test_get_goal_not_found() {
    let mock = BeeminderMock::start().await;
    mock.mount_fixture("goals/get_goal_not_found.json").await;

    let client = mock.client();
    let result = client.get_goal("nonexistent", false).await;

    assert!(result.is_err());
}
