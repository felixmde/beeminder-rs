mod common;

use beeminder::types::CreateDatapoint;
use common::mock_server::BeeminderMock;

#[tokio::test]
async fn test_create_all_datapoints_valid() {
    let mock = BeeminderMock::start().await;
    mock.mount_fixture("datapoints/create_all_valid.json").await;

    let client = mock.client();
    let datapoints = vec![
        CreateDatapoint::new(1.5).with_comment("batch1"),
        CreateDatapoint::new(2.5).with_comment("batch2"),
    ];

    let result = client.create_all_datapoints("testgoal", &datapoints).await;
    assert!(result.is_ok());
}
