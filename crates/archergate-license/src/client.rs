//! The main license client — the only thing developers interact with.

use chrono::{Duration, Utc};

use crate::cache;
use crate::error::LicenseError;
use crate::fingerprint;
use crate::integrity::ValidationReceipt;
use crate::types::*;

/// Default Archergate license API base URL.
const DEFAULT_API_URL: &str = "https://api.archergate.com";

/// User-Agent header sent with every API request.
const USER_AGENT: &str = concat!("archergate-license/", env!("CARGO_PKG_VERSION"));

/// License client for a single application.
///
/// Create one at application startup and call [`validate`](Self::validate)
/// on startup. That's it.
///
/// ```no_run
/// use archergate_license::LicenseClient;
///
/// let client = LicenseClient::new("your-api-key", "com.you.your-plugin");
/// match client.validate("XXXX-XXXX-XXXX-XXXX") {
///     Ok(()) => { /* proceed normally */ }
///     Err(e) => eprintln!("License error: {e}"),
/// }
/// ```
pub struct LicenseClient {
    api_key: String,
    plugin_id: String,
    api_url: String,
}

impl LicenseClient {
    /// Create a new license client.
    ///
    /// - `api_key`: Your Archergate developer API key.
    /// - `plugin_id`: A unique identifier for your plugin (e.g. `"com.yourname.synth"`).
    pub fn new(api_key: &str, plugin_id: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            plugin_id: plugin_id.to_string(),
            api_url: DEFAULT_API_URL.to_string(),
        }
    }

    /// Override the API base URL (for self-hosted servers or testing).
    pub fn with_api_url(mut self, url: &str) -> Self {
        self.api_url = url.trim_end_matches('/').to_string();
        self
    }

    /// Validate a license key.
    ///
    /// 1. Checks the local cache first.
    /// 2. If the cache is fresh (validated within 30 days and not expired), returns `Ok(())`.
    /// 3. If the cache is stale or missing, calls the Archergate API.
    /// 4. On network failure, falls back to the cache if within the 30-day grace period.
    ///
    /// Returns `Ok(())` if the license is valid. Returns a specific [`LicenseError`] otherwise.
    pub fn validate(&self, license_key: &str) -> Result<(), LicenseError> {
        let fp = fingerprint::machine_fingerprint();

        // Check local cache first
        if let Some(cached) = cache::load_license(&self.plugin_id) {
            if cached.license_key == license_key && cached.machine_fingerprint == fp {
                if cache::is_within_grace_period(&cached, Utc::now()) {
                    return Ok(());
                }
                // Cache expired — need to re-validate online
            } else if cached.license_key == license_key {
                return Err(LicenseError::MachineMismatch);
            }
        }

        // Call the API
        match self.call_validate(license_key, &fp) {
            Ok(resp) => Self::handle_validate_response(resp, license_key, fp, &self.plugin_id),
            Err(net_err) => {
                // Network failed — check if we have a valid offline cache
                if let Some(cached) = cache::load_license(&self.plugin_id) {
                    if cached.license_key == license_key
                        && cached.machine_fingerprint == fp
                        && cache::is_within_grace_period(&cached, Utc::now())
                    {
                        return Ok(());
                    }
                }
                Err(LicenseError::NetworkError(net_err))
            }
        }
    }

    /// Activate a license on this machine.
    ///
    /// Call this once when the user first enters their license key.
    /// On success, the activation is cached locally and future [`validate`](Self::validate)
    /// calls will use the cache.
    ///
    /// - `license_key`: The license key to activate.
    /// - `email`: Customer email address (for the developer's records).
    pub fn activate(
        &self,
        license_key: &str,
        email: &str,
    ) -> Result<ActivateResponse, LicenseError> {
        let fp = fingerprint::machine_fingerprint();
        let url = format!("{}/activate", self.api_url);
        let body = ActivateRequest {
            license_key: license_key.to_string(),
            machine_fingerprint: fp.clone(),
            plugin_id: self.plugin_id.clone(),
            email: email.to_string(),
        };

        let resp = ureq::post(&url)
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .set("User-Agent", USER_AGENT)
            .set("Content-Type", "application/json")
            .send_json(serde_json::to_value(&body).map_err(|e| LicenseError::NetworkError(e.to_string()))?)
            .map_err(|e| {
                // Parse activation-specific errors from 4xx responses
                if let ureq::Error::Status(status, resp) = e {
                    if let Ok(err_resp) = resp.into_json::<ValidateResponse>() {
                        return match err_resp.error.as_deref() {
                            Some("activation_limit") => LicenseError::ActivationLimitReached,
                            Some("expired") => LicenseError::Expired,
                            Some("machine_mismatch") => LicenseError::MachineMismatch,
                            _ => LicenseError::Invalid,
                        };
                    }
                    return LicenseError::NetworkError(format!("HTTP {status}"));
                }
                LicenseError::NetworkError(e.to_string())
            })?;

        let activate_resp: ActivateResponse = resp
            .into_json()
            .map_err(|e| LicenseError::NetworkError(e.to_string()))?;

        // Cache the activation
        let cached = CachedLicense {
            license_key: license_key.to_string(),
            machine_fingerprint: fp,
            validated_at: Utc::now(),
            expires_at: Utc::now() + Duration::days(365),
            offline_token: activate_resp.offline_token.clone(),
        };
        let _ = cache::save_license(&self.plugin_id, &cached);

        Ok(activate_resp)
    }

    /// Returns the machine fingerprint for this installation.
    ///
    /// SHA-256 of (CPU brand string + OS install ID). Lowercase hex, 64 chars.
    /// Stable across reboots. Changes only when hardware changes.
    pub fn machine_fingerprint() -> String {
        fingerprint::machine_fingerprint()
    }

    /// Start a 14-day trial. Stored locally, no API call needed.
    ///
    /// If a trial already exists for this plugin, returns the existing trial's
    /// remaining time. If the trial has expired, returns [`LicenseError::TrialExpired`].
    pub fn start_trial(&self) -> Result<TrialLicense, LicenseError> {
        if let Some(existing) = cache::load_trial(&self.plugin_id) {
            let now = Utc::now();
            if now >= existing.expires_at {
                return Err(LicenseError::TrialExpired);
            }
            let remaining = (existing.expires_at - now).num_days().max(0) as u32;
            return Ok(TrialLicense {
                expires_at: existing.expires_at,
                days_remaining: remaining,
            });
        }

        let now = Utc::now();
        let expires = now + Duration::days(14);

        let trial = CachedTrial {
            plugin_id: self.plugin_id.clone(),
            started_at: now,
            expires_at: expires,
        };

        cache::save_trial(&self.plugin_id, &trial)
            .map_err(|e| LicenseError::NetworkError(format!("failed to save trial: {e}")))?;

        let remaining = (expires - now).num_days().max(0) as u32;
        Ok(TrialLicense {
            expires_at: expires,
            days_remaining: remaining,
        })
    }

    /// Validate and return a cryptographic receipt proving the check ran.
    ///
    /// Use this for defense-in-depth: store the receipt and call
    /// [`ValidationReceipt::verify`] at other points in your plugin to
    /// detect binary patching.
    ///
    /// ```no_run
    /// use archergate_license::LicenseClient;
    ///
    /// let client = LicenseClient::new("key", "com.you.synth");
    /// let receipt = client.validate_with_receipt("LICENSE-KEY").unwrap();
    ///
    /// // Later, in your audio processing callback:
    /// let fp = LicenseClient::machine_fingerprint();
    /// assert!(receipt.verify("LICENSE-KEY", &fp, 86400));
    /// ```
    pub fn validate_with_receipt(
        &self,
        license_key: &str,
    ) -> Result<ValidationReceipt, LicenseError> {
        self.validate(license_key)?;
        let fp = fingerprint::machine_fingerprint();
        Ok(ValidationReceipt::issue(license_key, &fp))
    }

    /// Returns the plugin ID this client was created for.
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    fn handle_validate_response(
        resp: ValidateResponse,
        license_key: &str,
        fp: String,
        plugin_id: &str,
    ) -> Result<(), LicenseError> {
        if resp.valid {
            let expires_at = resp
                .expires_at
                .as_deref()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| Utc::now() + Duration::days(365));

            let cached = CachedLicense {
                license_key: license_key.to_string(),
                machine_fingerprint: fp,
                validated_at: Utc::now(),
                expires_at,
                offline_token: String::new(),
            };
            let _ = cache::save_license(plugin_id, &cached);
            Ok(())
        } else {
            match resp.error.as_deref() {
                Some("expired") => Err(LicenseError::Expired),
                Some("machine_mismatch") => Err(LicenseError::MachineMismatch),
                Some("activation_limit") => Err(LicenseError::ActivationLimitReached),
                _ => Err(LicenseError::Invalid),
            }
        }
    }

    fn call_validate(&self, license_key: &str, fp: &str) -> Result<ValidateResponse, String> {
        let url = format!("{}/validate", self.api_url);
        let body = ValidateRequest {
            license_key: license_key.to_string(),
            machine_fingerprint: fp.to_string(),
            plugin_id: self.plugin_id.clone(),
        };

        let resp = ureq::post(&url)
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .set("User-Agent", USER_AGENT)
            .set("Content-Type", "application/json")
            .send_json(serde_json::to_value(&body).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;

        resp.into_json::<ValidateResponse>()
            .map_err(|e| e.to_string())
    }
}
