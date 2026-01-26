mod common;

use common::mock_server::BeeminderMock;

#[tokio::test]
async fn test_get_datapoints_valid() {
    let mock = BeeminderMock::start().await;
    mock.mount_fixture("datapoints/get_datapoints_valid.json")
        .await;

    let client = mock.client();
    let datapoints = client
        .get_datapoints("exercise", None, None, None, None)
        .await
        .unwrap();

    assert_eq!(datapoints.len(), 2);
    assert_eq!(datapoints[0].id, "dp1234567890");
    assert!((datapoints[0].value - 1.0).abs() < f64::EPSILON);
    assert_eq!(datapoints[0].comment, Some("Morning workout".to_string()));
}
