//! Anti-tamper integrity checks.
//!
//! These make binary patching significantly harder by adding multiple
//! independent verification paths. A cracker can't just NOP out a single
//! `validate()` call — they have to find and disable all of these.
//!
//! Defense layers:
//! 1. **Function checksum**: validate() computes a result hash that downstream
//!    code can verify. Patching validate() to always return Ok changes the hash.
//! 2. **Heartbeat token**: a time-based token that proves validate() actually ran
//!    (not just that its return value was spoofed).
//! 3. **Cache signature**: the on-disk cache file is HMAC-signed. Editing the JSON
//!    directly invalidates the signature.
//! 4. **Decoy paths**: multiple code paths that look like the real check but aren't,
//!    making static analysis harder.

use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::sync::atomic::{AtomicU64, Ordering};

type HmacSha256 = Hmac<Sha256>;

/// Monotonically increasing heartbeat counter. If this stops incrementing,
/// the validate function is being bypassed.
static HEARTBEAT: AtomicU64 = AtomicU64::new(0);

/// A validation receipt that proves the check actually happened.
/// Plugin developers can call `receipt.verify()` as a second-layer check.
#[derive(Debug, Clone)]
pub struct ValidationReceipt {
    /// HMAC of (license_key || fingerprint || timestamp).
    pub signature: [u8; 32],
    /// Monotonic counter value when this receipt was issued.
    pub heartbeat: u64,
    /// Timestamp when validation occurred.
    pub timestamp: u64,
}

impl ValidationReceipt {
    /// Create a new receipt after a successful validation.
    pub(crate) fn issue(license_key: &str, fingerprint: &str) -> Self {
        let beat = HEARTBEAT.fetch_add(1, Ordering::SeqCst) + 1;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0); // safe: clock always after epoch

        let mut mac = <HmacSha256 as Mac>::new_from_slice(RECEIPT_KEY)
            .expect("HMAC accepts any key length"); // safe: constant key
        mac.update(license_key.as_bytes());
        mac.update(b"|");
        mac.update(fingerprint.as_bytes());
        mac.update(b"|");
        mac.update(&now.to_le_bytes());

        let result = mac.finalize().into_bytes();
        let mut sig = [0u8; 32];
        sig.copy_from_slice(&result);

        Self {
            signature: sig,
            heartbeat: beat,
            timestamp: now,
        }
    }

    /// Verify that this receipt is authentic and recent.
    ///
    /// Returns `true` if:
    /// - The signature matches the given license key + fingerprint
    /// - The receipt is less than `max_age_secs` old
    pub fn verify(&self, license_key: &str, fingerprint: &str, max_age_secs: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Time check
        if now.saturating_sub(self.timestamp) > max_age_secs {
            return false;
        }

        // Signature check
        let mut mac = <HmacSha256 as Mac>::new_from_slice(RECEIPT_KEY)
            .expect("HMAC accepts any key length"); // safe: constant key
        mac.update(license_key.as_bytes());
        mac.update(b"|");
        mac.update(fingerprint.as_bytes());
        mac.update(b"|");
        mac.update(&self.timestamp.to_le_bytes());
        mac.verify_slice(&self.signature).is_ok()
    }
}

/// Get the current heartbeat count.
/// If this returns 0 after your plugin has been running, validation is being bypassed.
pub fn heartbeat_count() -> u64 {
    HEARTBEAT.load(Ordering::SeqCst)
}

/// Sign a cache file's contents so that manual edits are detectable.
pub(crate) fn sign_cache(data: &[u8]) -> [u8; 32] {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(CACHE_KEY)
        .expect("HMAC accepts any key length"); // safe: constant key
    mac.update(data);
    let result = mac.finalize().into_bytes();
    let mut sig = [0u8; 32];
    sig.copy_from_slice(&result);
    sig
}

/// Verify a cache file signature.
pub(crate) fn verify_cache(data: &[u8], expected_sig: &[u8; 32]) -> bool {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(CACHE_KEY)
        .expect("HMAC accepts any key length"); // safe: constant key
    mac.update(data);
    mac.verify_slice(expected_sig).is_ok()
}

// These keys are compiled into the binary. A cracker would need to extract them
// to forge receipts or cache signatures. They're intentionally not const — the
// compiler won't inline them as easily, making static analysis slightly harder.
static RECEIPT_KEY: &[u8] = b"ag_receipt_v1_do_not_extract";
static CACHE_KEY: &[u8] = b"ag_cache_sig_v1_do_not_extract";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_round_trip() {
        let receipt = ValidationReceipt::issue("TEST-KEY", "abc123fp");
        assert!(receipt.verify("TEST-KEY", "abc123fp", 60));
    }

    #[test]
    fn receipt_wrong_key() {
        let receipt = ValidationReceipt::issue("TEST-KEY", "abc123fp");
        assert!(!receipt.verify("WRONG-KEY", "abc123fp", 60));
    }

    #[test]
    fn receipt_wrong_fingerprint() {
        let receipt = ValidationReceipt::issue("TEST-KEY", "abc123fp");
        assert!(!receipt.verify("TEST-KEY", "wrong_fp", 60));
    }

    #[test]
    fn receipt_expired() {
        let mut receipt = ValidationReceipt::issue("TEST-KEY", "abc123fp");
        receipt.timestamp = 0; // pretend it's from epoch
        assert!(!receipt.verify("TEST-KEY", "abc123fp", 60));
    }

    #[test]
    fn heartbeat_increments() {
        let before = heartbeat_count();
        let _ = ValidationReceipt::issue("K", "F");
        let _ = ValidationReceipt::issue("K", "F");
        let after = heartbeat_count();
        assert_eq!(after, before + 2);
    }

    #[test]
    fn cache_signature_valid() {
        let data = b"some cache data";
        let sig = sign_cache(data);
        assert!(verify_cache(data, &sig));
    }

    #[test]
    fn cache_signature_detects_tamper() {
        let data = b"some cache data";
        let sig = sign_cache(data);
        let tampered = b"some cache DATA";
        assert!(!verify_cache(tampered, &sig));
    }
}
