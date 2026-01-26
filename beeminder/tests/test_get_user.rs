mod common;

use common::mock_server::BeeminderMock;

#[tokio::test]
async fn test_get_user_valid() {
    let mock = BeeminderMock::start().await;
    mock.mount_fixture("user/get_user_valid.json").await;

    let client = mock.client();
    let user = client.get_user().await.unwrap();

    assert_eq!(user.username, "testuser");
    assert_eq!(user.timezone, "America/Los_Angeles");
    assert!(!user.deadbeat);
    assert_eq!(user.goals.len(), 3);
    assert!(user.goals.contains(&"exercise".to_string()));
}

#[tokio::test]
async fn test_get_user_invalid_auth() {
    let mock = BeeminderMock::start().await;
    mock.mount_fixture("user/get_user_invalid_auth.json").await;

    let client = mock.client();
    let result = client.get_user().await;

    assert!(result.is_err());
}
