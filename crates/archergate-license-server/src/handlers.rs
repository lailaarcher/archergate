//! Route handlers for the license API.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Router;
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::{Db, License};

pub type AppState = Arc<Db>;

/// Check if a license has expired. Returns the expiry string if expired, None otherwise.
fn check_expired(license: &License) -> Option<String> {
    let exp = license.expires_at.as_ref()?;
    let exp_dt = exp.parse::<chrono::DateTime<Utc>>().ok()?;
    if Utc::now() > exp_dt {
        Some(exp.clone())
    } else {
        None
    }
}

// ── POST /validate ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ValidateReq {
    pub license_key: String,
    pub machine_fingerprint: String,
    pub plugin_id: String,
}

#[derive(Debug, Serialize)]
pub struct ValidateResp {
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub async fn validate(
    State(db): State<AppState>,
    Json(req): Json<ValidateReq>,
) -> impl IntoResponse {
    let license = match db.find_license(&req.license_key, &req.plugin_id) {
        Some(l) => l,
        None => {
            return (
                StatusCode::OK,
                Json(ValidateResp {
                    valid: false,
                    expires_at: None,
                    error: Some("invalid".into()),
                }),
            );
        }
    };

    if let Some(exp) = check_expired(&license) {
        return (
            StatusCode::OK,
            Json(ValidateResp {
                valid: false,
                expires_at: Some(exp),
                error: Some("expired".into()),
            }),
        );
    }

    // Check machine binding
    if db
        .find_activation(&license.id, &req.machine_fingerprint)
        .is_none()
    {
        let count = db.activation_count(&license.id);
        if count >= license.max_machines {
            return (
                StatusCode::OK,
                Json(ValidateResp {
                    valid: false,
                    expires_at: license.expires_at.clone(),
                    error: Some("machine_mismatch".into()),
                }),
            );
        }
    }

    // Create or update activation (touch last_seen)
    let _ = db.upsert_activation(&license.id, &req.machine_fingerprint);

    (
        StatusCode::OK,
        Json(ValidateResp {
            valid: true,
            expires_at: license.expires_at,
            error: None,
        }),
    )
}

// ── POST /activate ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // email stored for developer records
pub struct ActivateReq {
    pub license_key: String,
    pub machine_fingerprint: String,
    pub plugin_id: String,
    pub email: String,
}

pub async fn activate(
    State(db): State<AppState>,
    Json(req): Json<ActivateReq>,
) -> impl IntoResponse {
    let license = match db.find_license(&req.license_key, &req.plugin_id) {
        Some(l) => l,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "valid": false, "error": "invalid" })),
            );
        }
    };

    if check_expired(&license).is_some() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "valid": false, "error": "expired" })),
        );
    }

    // Already activated — refresh
    if db
        .find_activation(&license.id, &req.machine_fingerprint)
        .is_some()
    {
        let _ = db.upsert_activation(&license.id, &req.machine_fingerprint);
        return (
            StatusCode::OK,
            Json(serde_json::json!({
                "token": uuid::Uuid::new_v4().to_string(),
                "offline_token": uuid::Uuid::new_v4().to_string(),
            })),
        );
    }

    // Check activation limit
    let count = db.activation_count(&license.id);
    if count >= license.max_machines {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "valid": false, "error": "activation_limit" })),
        );
    }

    let _ = db.upsert_activation(&license.id, &req.machine_fingerprint);

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "token": uuid::Uuid::new_v4().to_string(),
            "offline_token": uuid::Uuid::new_v4().to_string(),
        })),
    )
}

// ── POST /licenses (admin) ───────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateLicenseReq {
    pub plugin_id: String,
    pub email: Option<String>,
    pub expires_at: Option<String>,
    pub max_machines: Option<i32>,
}

pub async fn create_license(
    State(db): State<AppState>,
    Json(req): Json<CreateLicenseReq>,
) -> impl IntoResponse {
    let max = req.max_machines.unwrap_or(3);
    match db.create_license(
        &req.plugin_id,
        req.email.as_deref(),
        req.expires_at.as_deref(),
        max,
        "admin",
    ) {
        Ok(license) => (
            StatusCode::CREATED,
            Json(serde_json::to_value(license).unwrap()),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}

// ── Health check ─────────────────────────────────────────────────────

pub async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}

/// Build the Axum router with all routes. Used by main and tests.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/validate", axum::routing::post(validate))
        .route("/activate", axum::routing::post(activate))
        .route("/licenses", axum::routing::post(create_license))
        .route("/health", axum::routing::get(health))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::util::ServiceExt;

    fn test_app() -> (Router, Arc<Db>) {
        let db = Arc::new(Db::open_memory().unwrap());
        let router = build_router(db.clone());
        (router, db)
    }

    async fn json_post(app: &Router, path: &str, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
        let req = Request::builder()
            .method("POST")
            .uri(path)
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = ServiceExt::<Request<Body>>::oneshot(app.clone(), req).await.unwrap();
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        (status, json)
    }

    // ── Health ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn health_returns_ok() {
        let (app, _) = test_app();
        let req = Request::builder()
            .method("GET")
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let resp = ServiceExt::<Request<Body>>::oneshot(app, req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["status"], "ok");
        assert!(json["version"].as_str().unwrap().starts_with("0.1"));
    }

    // ── Validate ──────────────────��──────────────────────────────────

    #[tokio::test]
    async fn validate_invalid_key_returns_error() {
        let (app, _) = test_app();
        let (status, json) = json_post(&app, "/validate", serde_json::json!({
            "license_key": "DOES-NOT-EXIST",
            "machine_fingerprint": "fp_abc",
            "plugin_id": "com.test.plugin"
        })).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["valid"], false);
        assert_eq!(json["error"], "invalid");
    }

    #[tokio::test]
    async fn validate_valid_key_returns_success() {
        let (app, db) = test_app();
        let (_, api_key) = db.create_api_key("dev@test.com").unwrap();
        let license = db.create_license("com.test.synth", None, None, 3, &api_key.id).unwrap();

        let (status, json) = json_post(&app, "/validate", serde_json::json!({
            "license_key": license.license_key,
            "machine_fingerprint": "fp_machine_1",
            "plugin_id": "com.test.synth"
        })).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["valid"], true);
        // Activation should have been created
        assert_eq!(db.activation_count(&license.id), 1);
    }

    #[tokio::test]
    async fn validate_expired_key_returns_expired() {
        let (app, db) = test_app();
        let (_, api_key) = db.create_api_key("dev@test.com").unwrap();
        let license = db.create_license(
            "com.test.synth",
            None,
            Some("2020-01-01T00:00:00Z"), // far in the past
            3,
            &api_key.id,
        ).unwrap();

        let (status, json) = json_post(&app, "/validate", serde_json::json!({
            "license_key": license.license_key,
            "machine_fingerprint": "fp_machine_1",
            "plugin_id": "com.test.synth"
        })).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["valid"], false);
        assert_eq!(json["error"], "expired");
    }

    #[tokio::test]
    async fn validate_exceeds_machine_limit() {
        let (app, db) = test_app();
        let (_, api_key) = db.create_api_key("dev@test.com").unwrap();
        let license = db.create_license("com.test.synth", None, None, 2, &api_key.id).unwrap();

        // Activate on 2 machines (at the limit)
        db.upsert_activation(&license.id, "fp_machine_1").unwrap();
        db.upsert_activation(&license.id, "fp_machine_2").unwrap();

        // 3rd machine should be rejected
        let (status, json) = json_post(&app, "/validate", serde_json::json!({
            "license_key": license.license_key,
            "machine_fingerprint": "fp_machine_3",
            "plugin_id": "com.test.synth"
        })).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["valid"], false);
        assert_eq!(json["error"], "machine_mismatch");
    }

    #[tokio::test]
    async fn validate_same_machine_succeeds_at_limit() {
        let (app, db) = test_app();
        let (_, api_key) = db.create_api_key("dev@test.com").unwrap();
        let license = db.create_license("com.test.synth", None, None, 1, &api_key.id).unwrap();

        // Activate on 1 machine (at the limit)
        db.upsert_activation(&license.id, "fp_machine_1").unwrap();

        // Same machine should still succeed
        let (_, json) = json_post(&app, "/validate", serde_json::json!({
            "license_key": license.license_key,
            "machine_fingerprint": "fp_machine_1",
            "plugin_id": "com.test.synth"
        })).await;

        assert_eq!(json["valid"], true);
    }

    // ── Activate ────���─────────��──────────────────────────────────────

    #[tokio::test]
    async fn activate_creates_new_activation() {
        let (app, db) = test_app();
        let (_, api_key) = db.create_api_key("dev@test.com").unwrap();
        let license = db.create_license("com.test.synth", None, None, 3, &api_key.id).unwrap();

        let (status, json) = json_post(&app, "/activate", serde_json::json!({
            "license_key": license.license_key,
            "machine_fingerprint": "fp_new_machine",
            "plugin_id": "com.test.synth",
            "email": "user@example.com"
        })).await;

        assert_eq!(status, StatusCode::OK);
        assert!(json["token"].as_str().is_some());
        assert!(json["offline_token"].as_str().is_some());
        assert_eq!(db.activation_count(&license.id), 1);
    }

    #[tokio::test]
    async fn activate_rejects_over_machine_limit() {
        let (app, db) = test_app();
        let (_, api_key) = db.create_api_key("dev@test.com").unwrap();
        let license = db.create_license("com.test.synth", None, None, 1, &api_key.id).unwrap();

        // Fill the single slot
        db.upsert_activation(&license.id, "fp_existing").unwrap();

        let (status, json) = json_post(&app, "/activate", serde_json::json!({
            "license_key": license.license_key,
            "machine_fingerprint": "fp_new",
            "plugin_id": "com.test.synth",
            "email": "user@example.com"
        })).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json["error"], "activation_limit");
    }

    #[tokio::test]
    async fn activate_refreshes_existing_machine() {
        let (app, db) = test_app();
        let (_, api_key) = db.create_api_key("dev@test.com").unwrap();
        let license = db.create_license("com.test.synth", None, None, 1, &api_key.id).unwrap();

        // First activation
        db.upsert_activation(&license.id, "fp_machine_1").unwrap();

        // Re-activate same machine (should refresh, not reject)
        let (status, json) = json_post(&app, "/activate", serde_json::json!({
            "license_key": license.license_key,
            "machine_fingerprint": "fp_machine_1",
            "plugin_id": "com.test.synth",
            "email": "user@example.com"
        })).await;

        assert_eq!(status, StatusCode::OK);
        assert!(json["token"].as_str().is_some());
        assert_eq!(db.activation_count(&license.id), 1); // still 1, not 2
    }

    #[tokio::test]
    async fn activate_rejects_expired_license() {
        let (app, db) = test_app();
        let (_, api_key) = db.create_api_key("dev@test.com").unwrap();
        let license = db.create_license(
            "com.test.synth",
            None,
            Some("2020-01-01T00:00:00Z"),
            3,
            &api_key.id,
        ).unwrap();

        let (status, json) = json_post(&app, "/activate", serde_json::json!({
            "license_key": license.license_key,
            "machine_fingerprint": "fp_machine",
            "plugin_id": "com.test.synth",
            "email": "user@example.com"
        })).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json["error"], "expired");
    }

    // ── Create License ──────────────���────────────────────────────────

    #[tokio::test]
    async fn create_license_with_defaults() {
        let (app, db) = test_app();
        // The handler hardcodes api_key_id="admin", so seed it.
        // NOTE: This is a known issue — the /licenses endpoint should
        // require an API key in the request, not hardcode "admin".
        seed_admin_key(&db);

        let (status, json) = json_post(&app, "/licenses", serde_json::json!({
            "plugin_id": "com.test.new"
        })).await;

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(json["plugin_id"], "com.test.new");
        assert_eq!(json["max_machines"], 3); // default
        let key = json["license_key"].as_str().unwrap();
        assert_eq!(key.len(), 19); // XXXX-XXXX-XXXX-XXXX
        assert_eq!(key.chars().filter(|c| *c == '-').count(), 3);
    }

    #[tokio::test]
    async fn create_license_with_custom_machines() {
        let (app, db) = test_app();
        seed_admin_key(&db);

        let (_, json) = json_post(&app, "/licenses", serde_json::json!({
            "plugin_id": "com.test.custom",
            "max_machines": 5,
            "email": "buyer@example.com"
        })).await;

        assert_eq!(json["max_machines"], 5);
        assert_eq!(json["email"], "buyer@example.com");
    }

    /// Seed a dummy API key with id="admin" so the create_license handler works.
    fn seed_admin_key(db: &Db) {
        use rusqlite::params;
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO api_keys (id, key_hash, developer_email, created_at) VALUES (?1, ?2, ?3, ?4)",
            params!["admin", "hash_placeholder", "admin@archergate.io", "2026-01-01T00:00:00Z"],
        ).unwrap();
    }
}
