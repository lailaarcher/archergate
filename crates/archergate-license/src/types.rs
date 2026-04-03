//! Wire types for the license validation API and local cache.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Request body for `POST /validate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateRequest {
    /// The license key to validate.
    pub license_key: String,
    /// SHA-256 machine fingerprint of the requesting machine.
    pub machine_fingerprint: String,
    /// Unique plugin identifier (e.g. `"com.yourname.synth"`).
    pub plugin_id: String,
}

/// Response from `POST /validate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateResponse {
    /// `true` if the license is currently valid for this machine.
    pub valid: bool,
    /// ISO 8601 expiration timestamp, if the license has an expiry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    /// Error code string when `valid` is `false`.
    /// One of: `"invalid"`, `"expired"`, `"machine_mismatch"`, `"activation_limit"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Request body for `POST /activate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivateRequest {
    /// The license key to activate.
    pub license_key: String,
    /// SHA-256 machine fingerprint of this installation.
    pub machine_fingerprint: String,
    /// Unique plugin identifier.
    pub plugin_id: String,
    /// Customer email address.
    pub email: String,
}

/// Response from `POST /activate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivateResponse {
    /// Session token for this activation.
    pub token: String,
    /// Offline validation token (cached locally for grace period).
    pub offline_token: String,
}

/// A trial license with its expiration info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialLicense {
    /// When this trial expires (UTC).
    pub expires_at: DateTime<Utc>,
    /// Days remaining in the trial. 0 means it expired today.
    pub days_remaining: u32,
}

/// Cached license data stored at `~/.archergate/licenses/{plugin_id}.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedLicense {
    /// The validated license key.
    pub license_key: String,
    /// Machine fingerprint this license was validated on.
    pub machine_fingerprint: String,
    /// When the server last confirmed this license.
    pub validated_at: DateTime<Utc>,
    /// When the license expires.
    pub expires_at: DateTime<Utc>,
    /// Server-issued offline token for future offline verification.
    pub offline_token: String,
}

/// Cached trial data stored at `~/.archergate/trials/{plugin_id}.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedTrial {
    /// The plugin this trial is for.
    pub plugin_id: String,
    /// When the trial was started.
    pub started_at: DateTime<Utc>,
    /// When the trial expires.
    pub expires_at: DateTime<Utc>,
}
