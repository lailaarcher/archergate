//! # archergate-license
//!
//! License management SDK for indie software developers.
//!
//! Machine binding, offline grace periods, and 14-day trials — in a single crate.
//!
//! ## Quick start (Rust)
//!
//! ```no_run
//! use archergate_license::LicenseClient;
//!
//! let client = LicenseClient::new("your-api-key", "com.you.your-plugin");
//!
//! match client.validate("XXXX-XXXX-XXXX-XXXX") {
//!     Ok(()) => { /* app runs normally */ }
//!     Err(e) => eprintln!("License check failed: {e}"),
//! }
//! ```
//!
//! ## Quick start (C / C++)
//!
//! Link against `archergate_license.lib` (Windows) or `libarchergate_license.a` (macOS/Linux)
//! and include `archergate_license.h`:
//!
//! ```c
//! #include "archergate_license.h"
//!
//! AgLicenseClient* client = ag_license_new("your-api-key", "com.you.synth");
//! int rc = ag_license_validate(client, "XXXX-XXXX-XXXX-XXXX");
//! if (rc != 0) {
//!     const char* msg = ag_license_error_string(rc);
//!     // handle error
//! }
//! ag_license_free(client);
//! ```

#![deny(missing_docs)]

mod cache;
mod client;
mod error;
pub mod ffi;
mod fingerprint;
/// Anti-tamper integrity checks for cracking resistance.
pub mod integrity;
mod types;

pub use client::LicenseClient;
pub use error::LicenseError;
pub use integrity::ValidationReceipt;
pub use types::TrialLicense;

/// Re-exported API wire types for custom integrations.
pub mod api_types {
    pub use crate::types::{
        ActivateRequest, ActivateResponse, CachedLicense, CachedTrial, ValidateRequest,
        ValidateResponse,
    };
}
