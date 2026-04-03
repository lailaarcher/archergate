//! ARCHERGATE Core — the creative memory engine.
//!
//! Pure Rust library. No UI dependencies.
//! Every module mirrors the JS prototype in `legacy/archergate-engine/`.

pub mod types;
pub mod decision_vector;
pub mod ngram;
pub mod blend;
pub mod storage;
pub mod dna_crypto;
pub mod session_analytics;
pub mod provenance;

pub use types::*;
