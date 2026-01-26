mod common;

use common::mock_server::BeeminderMock;

#[tokio::test]
async fn test_get_auth_token_valid() {
    let mock = BeeminderMock::start().await;
    mock.mount_fixture("auth/get_token_valid.json").await;

    let client = mock.client();
    let result = client.get_auth_token().await.unwrap();
    assert_eq!(result.auth_token.as_deref(), Some("abc123"));
}
