//! Integration tests for archergate-license.
//!
//! Tests cover: fingerprint stability, trial lifecycle, cache behavior,
//! validation with mock server, and every LicenseError variant.

use archergate_license::{LicenseClient, LicenseError};

/// Spin up a one-shot HTTP server that returns `body` as JSON to the first request,
/// then shuts down. Returns the port.
fn mock_server(body: &'static str) -> (u16, std::thread::JoinHandle<()>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();

    let handle = std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            use std::io::{Read, Write};
            // Read the full request (wait for blank line)
            let mut buf = [0u8; 8192];
            let _ = stream.read(&mut buf);

            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(resp.as_bytes());
            let _ = stream.flush();
            // Graceful shutdown — let client read before we drop
            let _ = stream.shutdown(std::net::Shutdown::Write);
        }
    });

    (port, handle)
}

// ── Fingerprint ──────────────────────────────────────────────────────

#[test]
fn fingerprint_deterministic() {
    let a = LicenseClient::machine_fingerprint();
    let b = LicenseClient::machine_fingerprint();
    assert_eq!(a, b);
}

#[test]
fn fingerprint_format() {
    let fp = LicenseClient::machine_fingerprint();
    assert_eq!(fp.len(), 64);
    assert!(fp.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(fp, fp.to_lowercase());
}

// ── Trial ────────────────────────────────────────────────────────────

#[test]
fn trial_starts_with_14_days() {
    let id = format!("test.trial.start.{}", std::process::id());
    let client = LicenseClient::new("test-key", &id);

    let trial = client.start_trial().expect("should start trial");
    assert!(trial.days_remaining >= 13);
    assert!(trial.days_remaining <= 14);

    cleanup_trial(&id);
}

#[test]
fn trial_returns_existing_if_active() {
    let id = format!("test.trial.existing.{}", std::process::id());
    let client = LicenseClient::new("test-key", &id);

    let first = client.start_trial().expect("first");
    let second = client.start_trial().expect("second");
    assert_eq!(first.expires_at, second.expires_at);

    cleanup_trial(&id);
}

// ── Validation (network errors) ──────────────────────────────────────

#[test]
fn validate_returns_network_error_when_no_server() {
    let id = format!("test.validate.noserver.{}", std::process::id());
    let client = LicenseClient::new("test-key", &id)
        .with_api_url("http://127.0.0.1:1");

    let result = client.validate("FAKE-KEY-1234");
    assert!(
        matches!(result, Err(LicenseError::NetworkError(_))),
        "expected NetworkError, got {result:?}"
    );
}

// ── Validation with mock HTTP server ─────────────────────────────────

#[test]
fn validate_succeeds_with_valid_response() {
    let (port, handle) = mock_server(r#"{"valid":true,"expires_at":"2099-01-01T00:00:00Z"}"#);

    let id = format!("test.validate.ok.{}", std::process::id());
    let client = LicenseClient::new("test-key", &id)
        .with_api_url(&format!("http://127.0.0.1:{port}"));

    let result = client.validate("VALID-KEY");
    handle.join().expect("server thread");
    assert!(result.is_ok(), "expected Ok, got {result:?}");

    cleanup_license(&id);
}

#[test]
fn validate_returns_invalid_for_bad_key() {
    let (port, handle) = mock_server(r#"{"valid":false,"error":"invalid"}"#);

    let id = format!("test.validate.invalid.{}", std::process::id());
    let client = LicenseClient::new("test-key", &id)
        .with_api_url(&format!("http://127.0.0.1:{port}"));

    let result = client.validate("BAD-KEY");
    handle.join().expect("server thread");
    assert!(
        matches!(result, Err(LicenseError::Invalid)),
        "expected Invalid, got {result:?}"
    );
}

#[test]
fn validate_returns_expired_for_expired_key() {
    let (port, handle) = mock_server(r#"{"valid":false,"error":"expired"}"#);

    let id = format!("test.validate.expired.{}", std::process::id());
    let client = LicenseClient::new("test-key", &id)
        .with_api_url(&format!("http://127.0.0.1:{port}"));

    let result = client.validate("EXPIRED-KEY");
    handle.join().expect("server thread");
    assert!(
        matches!(result, Err(LicenseError::Expired)),
        "expected Expired, got {result:?}"
    );
}

#[test]
fn validate_returns_machine_mismatch() {
    let (port, handle) = mock_server(r#"{"valid":false,"error":"machine_mismatch"}"#);

    let id = format!("test.validate.mismatch.{}", std::process::id());
    let client = LicenseClient::new("test-key", &id)
        .with_api_url(&format!("http://127.0.0.1:{port}"));

    let result = client.validate("WRONG-MACHINE-KEY");
    handle.join().expect("server thread");
    assert!(
        matches!(result, Err(LicenseError::MachineMismatch)),
        "expected MachineMismatch, got {result:?}"
    );
}

#[test]
fn validate_uses_cache_on_second_call() {
    let (port, handle) = mock_server(r#"{"valid":true,"expires_at":"2099-01-01T00:00:00Z"}"#);

    let id = format!("test.validate.cache.{}", std::process::id());
    let client = LicenseClient::new("test-key", &id)
        .with_api_url(&format!("http://127.0.0.1:{port}"));

    // First call hits the server
    assert!(client.validate("CACHE-KEY").is_ok(), "first call should succeed");
    handle.join().expect("server thread");

    // Second call should hit cache (server is gone)
    assert!(client.validate("CACHE-KEY").is_ok(), "second call should use cache");

    cleanup_license(&id);
}

#[test]
fn validate_offline_grace_period() {
    let id = format!("test.validate.offline.{}", std::process::id());
    let fp = LicenseClient::machine_fingerprint();

    // Pre-seed a cache file
    let cached = archergate_license::api_types::CachedLicense {
        license_key: "OFFLINE-KEY".into(),
        machine_fingerprint: fp,
        validated_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now() + chrono::Duration::days(365),
        offline_token: String::new(),
    };
    let path = dirs::home_dir()
        .expect("home")
        .join(".archergate")
        .join("licenses")
        .join(format!("{id}.json"));
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(&path, serde_json::to_string(&cached).expect("ser")).expect("write");

    // No server running — should succeed via cache
    let client = LicenseClient::new("test-key", &id)
        .with_api_url("http://127.0.0.1:1");

    assert!(client.validate("OFFLINE-KEY").is_ok());

    cleanup_license(&id);
}

// ── Activation ───────────────────────────────────────────────────────

#[test]
fn activate_succeeds_with_valid_response() {
    let (port, handle) = mock_server(
        r#"{"token":"tok_abc123","offline_token":"off_xyz789"}"#,
    );

    let id = format!("test.activate.ok.{}", std::process::id());
    let client = LicenseClient::new("test-key", &id)
        .with_api_url(&format!("http://127.0.0.1:{port}"));

    let result = client.activate("ACTIVATE-KEY", "user@example.com");
    handle.join().expect("server thread");
    let resp = result.expect("activation should succeed");
    assert_eq!(resp.token, "tok_abc123");
    assert_eq!(resp.offline_token, "off_xyz789");

    cleanup_license(&id);
}

#[test]
fn activate_returns_activation_limit_reached() {
    let (port, handle) = mock_server_with_status(
        400,
        r#"{"valid":false,"error":"activation_limit"}"#,
    );

    let id = format!("test.activate.limit.{}", std::process::id());
    let client = LicenseClient::new("test-key", &id)
        .with_api_url(&format!("http://127.0.0.1:{port}"));

    let result = client.activate("LIMIT-KEY", "user@example.com");
    handle.join().expect("server thread");
    assert!(
        matches!(result, Err(LicenseError::ActivationLimitReached)),
        "expected ActivationLimitReached, got {result:?}"
    );
}

// ── Validate with receipt ────────────────────────────────────────────

#[test]
fn validate_with_receipt_returns_verifiable_receipt() {
    let (port, handle) = mock_server(r#"{"valid":true,"expires_at":"2099-01-01T00:00:00Z"}"#);

    let id = format!("test.receipt.ok.{}", std::process::id());
    let client = LicenseClient::new("test-key", &id)
        .with_api_url(&format!("http://127.0.0.1:{port}"));

    let receipt = client.validate_with_receipt("RECEIPT-KEY").expect("should succeed");
    handle.join().expect("server thread");

    let fp = LicenseClient::machine_fingerprint();
    assert!(receipt.verify("RECEIPT-KEY", &fp, 60), "receipt should verify with correct key and fingerprint");
    assert!(!receipt.verify("WRONG-KEY", &fp, 60), "receipt should fail with wrong key");
    assert!(!receipt.verify("RECEIPT-KEY", "wrong_fp", 60), "receipt should fail with wrong fingerprint");

    cleanup_license(&id);
}

// ── Activation limit via validate ────────────────────────────────────

#[test]
fn validate_returns_activation_limit_reached() {
    let (port, handle) = mock_server(r#"{"valid":false,"error":"activation_limit"}"#);

    let id = format!("test.validate.limit.{}", std::process::id());
    let client = LicenseClient::new("test-key", &id)
        .with_api_url(&format!("http://127.0.0.1:{port}"));

    let result = client.validate("LIMIT-KEY");
    handle.join().expect("server thread");
    assert!(
        matches!(result, Err(LicenseError::ActivationLimitReached)),
        "expected ActivationLimitReached, got {result:?}"
    );
}

// ── Trial expiry ─────────────────────────────────────────────────────

#[test]
fn trial_returns_expired_after_period() {
    let id = format!("test.trial.expired.{}", std::process::id());

    // Pre-seed an expired trial cache
    let expired_trial = archergate_license::api_types::CachedTrial {
        plugin_id: id.clone(),
        started_at: chrono::Utc::now() - chrono::Duration::days(15),
        expires_at: chrono::Utc::now() - chrono::Duration::days(1),
    };
    let path = dirs::home_dir()
        .expect("home")
        .join(".archergate")
        .join("trials")
        .join(format!("{id}.json"));
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(&path, serde_json::to_string(&expired_trial).expect("ser")).expect("write");

    let client = LicenseClient::new("test-key", &id);
    let result = client.start_trial();
    assert!(
        matches!(result, Err(LicenseError::TrialExpired)),
        "expected TrialExpired, got {result:?}"
    );

    cleanup_trial(&id);
}

// ── Malformed server response ────────────────────────────────────────

#[test]
fn validate_handles_malformed_json_gracefully() {
    let (port, handle) = mock_server(r#"this is not json"#);

    let id = format!("test.validate.malformed.{}", std::process::id());
    let client = LicenseClient::new("test-key", &id)
        .with_api_url(&format!("http://127.0.0.1:{port}"));

    let result = client.validate("MALFORMED-KEY");
    handle.join().expect("server thread");
    // Should return a NetworkError (deserialization failure), not panic
    assert!(
        matches!(result, Err(LicenseError::NetworkError(_))),
        "expected NetworkError for malformed JSON, got {result:?}"
    );
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Mock server that returns a custom HTTP status code.
fn mock_server_with_status(status: u16, body: &'static str) -> (u16, std::thread::JoinHandle<()>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();

    let handle = std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            use std::io::{Read, Write};
            let mut buf = [0u8; 8192];
            let _ = stream.read(&mut buf);

            let reason = match status {
                200 => "OK",
                400 => "Bad Request",
                _ => "Error",
            };
            let resp = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(resp.as_bytes());
            let _ = stream.flush();
            let _ = stream.shutdown(std::net::Shutdown::Write);
        }
    });

    (port, handle)
}

fn cleanup_license(plugin_id: &str) {
    if let Some(home) = dirs::home_dir() {
        let _ = std::fs::remove_file(
            home.join(".archergate")
                .join("licenses")
                .join(format!("{plugin_id}.json")),
        );
    }
}

fn cleanup_trial(plugin_id: &str) {
    if let Some(home) = dirs::home_dir() {
        let _ = std::fs::remove_file(
            home.join(".archergate")
                .join("trials")
                .join(format!("{plugin_id}.json")),
        );
    }
}
