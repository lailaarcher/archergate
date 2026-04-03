//! Error types for the license SDK.

use std::fmt;

/// Every way a license check can fail.
///
/// Match on this in your application startup to decide what UI to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LicenseError {
    /// License key is not recognized by the server.
    Invalid,
    /// License was valid but has passed its expiration date.
    Expired,
    /// License is bound to a different machine.
    MachineMismatch,
    /// Could not reach the license server and no valid offline cache exists.
    NetworkError(String),
    /// Trial period has ended.
    TrialExpired,
    /// Activation limit reached — too many machines for this license.
    ActivationLimitReached,
}

impl fmt::Display for LicenseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid => write!(f, "license key is invalid"),
            Self::Expired => write!(f, "license has expired"),
            Self::MachineMismatch => write!(f, "license is bound to a different machine"),
            Self::NetworkError(msg) => write!(f, "network error: {msg}"),
            Self::TrialExpired => write!(f, "trial period has expired"),
            Self::ActivationLimitReached => {
                write!(f, "activation limit reached for this license")
            }
        }
    }
}

impl std::error::Error for LicenseError {}

/// Integer error codes for the C FFI layer.
impl LicenseError {
    /// Convert to a stable integer code for FFI.
    ///
    /// - `0` = success (not an error)
    /// - `-1` = Invalid
    /// - `-2` = Expired
    /// - `-3` = MachineMismatch
    /// - `-4` = NetworkError
    /// - `-5` = TrialExpired
    /// - `-6` = ActivationLimitReached
    pub fn to_code(&self) -> i32 {
        match self {
            Self::Invalid => -1,
            Self::Expired => -2,
            Self::MachineMismatch => -3,
            Self::NetworkError(_) => -4,
            Self::TrialExpired => -5,
            Self::ActivationLimitReached => -6,
        }
    }

    /// Human-readable error string for a given code.
    pub fn message_for_code(code: i32) -> &'static str {
        match code {
            0 => "success",
            -1 => "license key is invalid",
            -2 => "license has expired",
            -3 => "license is bound to a different machine",
            -4 => "network error (no internet and no cached license)",
            -5 => "trial period has expired",
            -6 => "activation limit reached for this license",
            _ => "unknown error",
        }
    }
}
