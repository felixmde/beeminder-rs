mod common;

use beeminder::types::CreateDatapoint;
use common::mock_server::BeeminderMock;

#[tokio::test]
async fn test_create_datapoint_valid() {
    let mock = BeeminderMock::start().await;
    mock.mount_fixture("datapoints/create_datapoint_valid.json")
        .await;

    let client = mock.client();
    let datapoint = CreateDatapoint::new(2.5).with_comment("Test datapoint");
    let result = client
        .create_datapoint("exercise", &datapoint)
        .await
        .unwrap();

    assert_eq!(result.id, "dp_new_12345");
    assert!((result.value - 2.5).abs() < f64::EPSILON);
    assert_eq!(result.comment, Some("Test datapoint".to_string()));
}
