//! Route handlers for the license API.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
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
