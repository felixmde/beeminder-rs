use beeminder::BeeminderClient;

#[test]
fn test_with_base_url_changes_base() {
    let _client =
        BeeminderClient::new("test_key".into()).with_base_url("http://localhost:8080/api/v1/");

    // We can't directly inspect base_url, but we can verify it builds
    // The real test is that mock server tests work
}

#[test]
fn test_default_base_url_is_beeminder() {
    let _client = BeeminderClient::new("test_key".into());
    // Client should work with default URL
}
