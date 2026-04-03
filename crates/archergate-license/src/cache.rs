//! Local license cache — persists validated licenses to disk with HMAC signatures.
//!
//! Location: `~/.archergate/licenses/{plugin_id}.json`
//! Trials:   `~/.archergate/trials/{plugin_id}.json`
//!
//! Each cache file is accompanied by a `.sig` file containing an HMAC-SHA256
//! signature. If the JSON is edited manually, the signature won't match
//! and the cache is treated as missing.

use std::path::PathBuf;

use chrono::{DateTime, Utc};

use crate::integrity;
use crate::types::{CachedLicense, CachedTrial};

/// Default offline grace period: 30 days.
const OFFLINE_GRACE_DAYS: i64 = 30;

fn licenses_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".archergate").join("licenses"))
}

fn trials_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".archergate").join("trials"))
}

fn license_path(plugin_id: &str) -> Option<PathBuf> {
    licenses_dir().map(|d| d.join(format!("{plugin_id}.json")))
}

fn trial_path(plugin_id: &str) -> Option<PathBuf> {
    trials_dir().map(|d| d.join(format!("{plugin_id}.json")))
}

fn sig_path(json_path: &std::path::Path) -> PathBuf {
    json_path.with_extension("sig")
}

/// Save a validated license to disk with an HMAC signature.
pub fn save_license(plugin_id: &str, cached: &CachedLicense) -> Result<(), String> {
    let path = license_path(plugin_id).ok_or("cannot determine home directory")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
    }
    let json = serde_json::to_string_pretty(cached).map_err(|e| format!("serialize: {e}"))?;
    let sig = integrity::sign_cache(json.as_bytes());
    std::fs::write(&path, &json).map_err(|e| format!("write: {e}"))?;
    std::fs::write(sig_path(&path), hex::encode(sig)).map_err(|e| format!("write sig: {e}"))?;
    Ok(())
}

/// Load a cached license from disk. Returns `None` if missing, unreadable, or tampered.
pub fn load_license(plugin_id: &str) -> Option<CachedLicense> {
    let path = license_path(plugin_id)?;
    let data = std::fs::read_to_string(&path).ok()?;

    // Verify signature — tampered files are treated as missing
    if let Ok(sig_hex) = std::fs::read_to_string(sig_path(&path)) {
        if let Ok(sig_bytes) = hex::decode(sig_hex.trim()) {
            if sig_bytes.len() == 32 {
                let mut sig = [0u8; 32];
                sig.copy_from_slice(&sig_bytes);
                if !integrity::verify_cache(data.as_bytes(), &sig) {
                    return None; // tampered
                }
            } else {
                return None; // invalid sig format
            }
        } else {
            return None; // invalid hex
        }
    }
    // If no sig file exists, accept the cache (backward compat with pre-signature caches)

    serde_json::from_str(&data).ok()
}

/// Check if the cached license is still within the offline grace period.
pub fn is_within_grace_period(cached: &CachedLicense, now: DateTime<Utc>) -> bool {
    let cutoff = cached.validated_at + chrono::Duration::days(OFFLINE_GRACE_DAYS);
    now < cutoff && now < cached.expires_at
}

/// Save a trial record to disk with an HMAC signature.
pub fn save_trial(plugin_id: &str, trial: &CachedTrial) -> Result<(), String> {
    let path = trial_path(plugin_id).ok_or("cannot determine home directory")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
    }
    let json = serde_json::to_string_pretty(trial).map_err(|e| format!("serialize: {e}"))?;
    let sig = integrity::sign_cache(json.as_bytes());
    std::fs::write(&path, &json).map_err(|e| format!("write: {e}"))?;
    std::fs::write(sig_path(&path), hex::encode(sig)).map_err(|e| format!("write sig: {e}"))?;
    Ok(())
}

/// Load a cached trial from disk. Returns `None` if missing, unreadable, or tampered.
pub fn load_trial(plugin_id: &str) -> Option<CachedTrial> {
    let path = trial_path(plugin_id)?;
    let data = std::fs::read_to_string(&path).ok()?;

    // Verify signature
    if let Ok(sig_hex) = std::fs::read_to_string(sig_path(&path)) {
        if let Ok(sig_bytes) = hex::decode(sig_hex.trim()) {
            if sig_bytes.len() == 32 {
                let mut sig = [0u8; 32];
                sig.copy_from_slice(&sig_bytes);
                if !integrity::verify_cache(data.as_bytes(), &sig) {
                    return None; // tampered
                }
            } else {
                return None;
            }
        } else {
            return None;
        }
    }

    serde_json::from_str(&data).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn make_cached() -> CachedLicense {
        let now = Utc::now();
        CachedLicense {
            license_key: "TEST-KEY-1234".into(),
            machine_fingerprint: "abc123".into(),
            validated_at: now,
            expires_at: now + Duration::days(365),
            offline_token: "tok".into(),
        }
    }

    #[test]
    fn grace_period_valid_when_recent() {
        let cached = make_cached();
        assert!(is_within_grace_period(&cached, Utc::now()));
    }

    #[test]
    fn grace_period_expired_after_30_days() {
        let cached = make_cached();
        let future = Utc::now() + Duration::days(31);
        assert!(!is_within_grace_period(&cached, future));
    }

    #[test]
    fn grace_period_expired_when_license_expired() {
        let now = Utc::now();
        let cached = CachedLicense {
            license_key: "K".into(),
            machine_fingerprint: "F".into(),
            validated_at: now,
            expires_at: now + Duration::days(5),
            offline_token: "tok".into(),
        };
        let after_expiry = now + Duration::days(6);
        assert!(!is_within_grace_period(&cached, after_expiry));
    }

    #[test]
    fn round_trip_to_disk() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test-plugin.json");

        let cached = make_cached();
        let json = serde_json::to_string_pretty(&cached).expect("ser");
        let sig = integrity::sign_cache(json.as_bytes());
        std::fs::write(&path, &json).expect("write");
        std::fs::write(sig_path(&path), hex::encode(sig)).expect("write sig");

        let loaded: CachedLicense =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("de");
        assert_eq!(loaded.license_key, cached.license_key);
        assert_eq!(loaded.machine_fingerprint, cached.machine_fingerprint);
    }

    #[test]
    fn tampered_cache_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");

        // Write a valid signed cache
        let cached = make_cached();
        let json = serde_json::to_string_pretty(&cached).expect("ser");
        let sig = integrity::sign_cache(json.as_bytes());

        let json_path = dir.path().join("tampered.json");
        std::fs::write(&json_path, &json).expect("write");
        std::fs::write(sig_path(&json_path), hex::encode(sig)).expect("write sig");

        // Tamper with the JSON
        let tampered = json.replace("TEST-KEY-1234", "CRACKED-KEY");
        std::fs::write(&json_path, tampered).expect("overwrite");

        // Verify the signature now fails
        let data = std::fs::read_to_string(&json_path).expect("read");
        let sig_hex = std::fs::read_to_string(sig_path(&json_path)).expect("read sig");
        let sig_bytes = hex::decode(sig_hex.trim()).expect("decode");
        let mut sig_arr = [0u8; 32];
        sig_arr.copy_from_slice(&sig_bytes);
        assert!(!integrity::verify_cache(data.as_bytes(), &sig_arr));
    }
}
