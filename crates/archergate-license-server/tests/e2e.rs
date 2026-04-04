//! End-to-end integration test: SDK client ↔ real Axum server (in-process).
//!
//! This is the single most important test in the project. It proves that the
//! archergate-license SDK and archergate-license-server crates actually work
//! together over real HTTP.

use std::sync::Arc;

use archergate_license::LicenseClient;
use archergate_license_server::db::Db;
use archergate_license_server::handlers;

/// Start an in-process Axum server on a random port and return the base URL.
async fn start_server(db: Arc<Db>) -> (String, tokio::task::JoinHandle<()>) {
    let app = handlers::build_router(db);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://127.0.0.1:{}", addr.port());

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give the server a moment to start accepting connections
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    (url, handle)
}

#[tokio::test]
async fn full_lifecycle_validate() {
    // 1. Set up in-memory database and start server
    let db = Arc::new(Db::open_memory().unwrap());
    let (_, api_key) = db.create_api_key("dev@test.com").unwrap();
    let license = db
        .create_license("com.e2e.synth", Some("user@test.com"), None, 3, &api_key.id)
        .unwrap();

    let (url, server_handle) = start_server(db.clone()).await;

    // 2. Create SDK client pointing at our test server
    let plugin_id = format!("com.e2e.synth.{}", std::process::id());
    // Use a unique plugin_id for the client to avoid cache conflicts,
    // but the server license is for "com.e2e.synth"
    let client = LicenseClient::new("test-api-key", "com.e2e.synth")
        .with_api_url(&url);

    // 3. Validate — should succeed (first activation)
    let result = client.validate(&license.license_key);
    assert!(result.is_ok(), "first validate should succeed: {result:?}");

    // 4. Validate again — should use cache (server still running, but cache is valid)
    let result2 = client.validate(&license.license_key);
    assert!(result2.is_ok(), "second validate should succeed via cache: {result2:?}");

    // 5. Validate with receipt — should work and receipt should verify
    let receipt = client.validate_with_receipt(&license.license_key);
    assert!(receipt.is_ok(), "validate_with_receipt should succeed: {receipt:?}");
    let receipt = receipt.unwrap();
    let fp = LicenseClient::machine_fingerprint();
    assert!(
        receipt.verify(&license.license_key, &fp, 60),
        "receipt should verify with correct key and fingerprint"
    );

    // 6. Verify activation was recorded in DB
    let count = db.activation_count(&license.id);
    assert_eq!(count, 1, "should have exactly 1 activation");

    // Clean up
    server_handle.abort();
    cleanup_license("com.e2e.synth");
}

#[tokio::test]
async fn full_lifecycle_activate_then_validate() {
    let db = Arc::new(Db::open_memory().unwrap());
    let (_, api_key) = db.create_api_key("dev@test.com").unwrap();
    let license = db
        .create_license("com.e2e.activate", None, None, 2, &api_key.id)
        .unwrap();

    let (url, server_handle) = start_server(db.clone()).await;

    let client = LicenseClient::new("test-api-key", "com.e2e.activate")
        .with_api_url(&url);

    // 1. Activate
    let activate_result = client.activate(&license.license_key, "user@example.com");
    assert!(activate_result.is_ok(), "activate should succeed: {activate_result:?}");
    let resp = activate_result.unwrap();
    assert!(!resp.token.is_empty(), "should return a token");
    assert!(!resp.offline_token.is_empty(), "should return an offline token");

    // 2. Validate after activation (should succeed, uses cache from activate)
    let validate_result = client.validate(&license.license_key);
    assert!(validate_result.is_ok(), "validate after activate should succeed: {validate_result:?}");

    // 3. Check DB state
    assert_eq!(db.activation_count(&license.id), 1);

    server_handle.abort();
    cleanup_license("com.e2e.activate");
}

#[tokio::test]
async fn validates_invalid_key_against_real_server() {
    let db = Arc::new(Db::open_memory().unwrap());
    let (url, server_handle) = start_server(db).await;

    let client = LicenseClient::new("test-api-key", "com.e2e.invalid")
        .with_api_url(&url);

    let result = client.validate("FAKE-0000-0000-0000");
    assert!(
        matches!(result, Err(archergate_license::LicenseError::Invalid)),
        "should return Invalid for nonexistent key: {result:?}"
    );

    server_handle.abort();
}

#[tokio::test]
async fn validates_expired_license_against_real_server() {
    let db = Arc::new(Db::open_memory().unwrap());
    let (_, api_key) = db.create_api_key("dev@test.com").unwrap();
    let license = db
        .create_license(
            "com.e2e.expired",
            None,
            Some("2020-01-01T00:00:00Z"),
            3,
            &api_key.id,
        )
        .unwrap();

    let (url, server_handle) = start_server(db).await;

    let client = LicenseClient::new("test-api-key", "com.e2e.expired")
        .with_api_url(&url);

    let result = client.validate(&license.license_key);
    assert!(
        matches!(result, Err(archergate_license::LicenseError::Expired)),
        "should return Expired: {result:?}"
    );

    server_handle.abort();
}

#[tokio::test]
async fn activation_limit_enforced_end_to_end() {
    let db = Arc::new(Db::open_memory().unwrap());
    let (_, api_key) = db.create_api_key("dev@test.com").unwrap();
    // max_machines = 1
    let license = db
        .create_license("com.e2e.limit", None, None, 1, &api_key.id)
        .unwrap();

    // Pre-fill the single activation slot with a different machine
    db.upsert_activation(&license.id, "other_machine_fingerprint")
        .unwrap();

    let (url, server_handle) = start_server(db).await;

    let client = LicenseClient::new("test-api-key", "com.e2e.limit")
        .with_api_url(&url);

    // Our machine fingerprint is different from "other_machine_fingerprint",
    // so this should fail with machine_mismatch (activation limit hit)
    let result = client.validate(&license.license_key);
    assert!(
        matches!(
            result,
            Err(archergate_license::LicenseError::MachineMismatch)
                | Err(archergate_license::LicenseError::ActivationLimitReached)
        ),
        "should reject due to machine limit: {result:?}"
    );

    server_handle.abort();
}

fn cleanup_license(plugin_id: &str) {
    if let Some(home) = dirs::home_dir() {
        let _ = std::fs::remove_file(
            home.join(".archergate")
                .join("licenses")
                .join(format!("{plugin_id}.json")),
        );
        let _ = std::fs::remove_file(
            home.join(".archergate")
                .join("licenses")
                .join(format!("{plugin_id}.json.sig")),
        );
    }
}
