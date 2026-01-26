mod common;

use common::mock_server::BeeminderMock;

#[tokio::test]
async fn test_get_goals_valid() {
    let mock = BeeminderMock::start().await;
    mock.mount_fixture("goals/get_goals_valid.json").await;

    let client = mock.client();
    let goals = client.get_goals().await.unwrap();

    assert_eq!(goals.len(), 1);
    assert_eq!(goals[0].slug, "exercise");
    assert_eq!(goals[0].title, "Daily Exercise");
    assert_eq!(goals[0].safebuf, 2);
}
