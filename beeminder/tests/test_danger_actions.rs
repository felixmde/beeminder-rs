mod common;

use common::mock_server::BeeminderMock;

#[tokio::test]
async fn test_shortcircuit_valid() {
    let mock = BeeminderMock::start().await;
    mock.mount_fixture("danger/shortcircuit_valid.json").await;

    let client = mock.client();
    let result = client.shortcircuit("testgoal").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_stepdown_valid() {
    let mock = BeeminderMock::start().await;
    mock.mount_fixture("danger/stepdown_valid.json").await;

    let client = mock.client();
    let result = client.stepdown("testgoal").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_cancel_stepdown_valid() {
    let mock = BeeminderMock::start().await;
    mock.mount_fixture("danger/cancel_stepdown_valid.json")
        .await;

    let client = mock.client();
    let result = client.cancel_stepdown("testgoal").await;
    assert!(result.is_ok());
}
