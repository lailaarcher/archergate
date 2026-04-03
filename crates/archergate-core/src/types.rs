//! Core types for the ARCHERGATE engine.
//!
//! Every struct here mirrors the JS engine's data model exactly.
//! Serde derives enable JSON serialization for Tauri commands
//! and .agdna export/import.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// --- Raw Input ---

/// Raw MIDI event from the plugin's WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawMidiEvent {
    pub note: u8,
    pub velocity: u8,
    pub channel: u8,
    pub timestamp_ms: u64,
}

// --- Decision Vector ---

/// The full fingerprint of a single musical decision.
/// 15 fields capturing not just the note, but the full context
/// of when, how hard, how long, what came before, and where
/// in the session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionVector {
    pub note: u8,
    pub velocity: u8,
    pub beat_position: f64,
    pub duration_ms: u64,
    pub bpm: f64,
    pub key: u8,
    pub mode: u8,
    pub interval: i8,
    pub time_since_last_ms: u64,
    pub channel: u8,
    pub is_drum: bool,
    pub session_minute: u32,
    pub hour_of_day: u8,
    pub looped_bar: bool,
    pub was_deleted: bool,
    pub timestamp_ms: u64,
    pub session_id: String,
}

/// Optional context overrides for event processing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventContext {
    pub bpm: Option<f64>,
    pub key: Option<u8>,
    pub mode: Option<u8>,
    pub looped_bar: Option<bool>,
    pub was_deleted: Option<bool>,
    pub duration_ms: Option<u64>,
}

// --- Predictions ---

/// A single prediction from the engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub note: u8,
    pub probability: f64,
    pub velocity: u8,
    pub beat_position: f64,
}

/// Result of processing a single event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventResult {
    pub predictions: Vec<Prediction>,
    pub bpm: f64,
    pub key: u8,
    pub mode: u8,
    pub dna_percent: u8,
    pub dna_stage: u8,
    pub latency_ms: f64,
    pub session_minute: u32,
}

// --- N-gram Model ---

/// Exported transition table: context_key → { value → weight }.
pub type TransitionTable = HashMap<String, HashMap<String, f64>>;

/// All five sub-models exported for DNA packaging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelExport {
    pub version: u32,
    pub rhythm: TransitionTable,
    pub harmony: TransitionTable,
    pub velocity: TransitionTable,
    pub workflow: TransitionTable,
    pub tempo: TransitionTable,
    pub meta: ModelMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMeta {
    #[serde(rename = "totalEvents")]
    pub total_events: u64,
    #[serde(rename = "sessionCount")]
    pub session_count: u32,
    #[serde(rename = "dnaPercent")]
    pub dna_percent: u8,
    #[serde(rename = "exportedAt")]
    pub exported_at: u64,
}

/// Engine summary for the DNA profile view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineSummary {
    pub total_events: u64,
    pub session_count: u32,
    pub dna_percent: u8,
    pub dna_stage: u8,
    pub harmony_contexts: usize,
    pub rhythm_contexts: usize,
    pub average_confidence: f64,
}

// --- DNA ---

/// Metadata envelope for a .agdna file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnaEnvelope {
    pub publisher_id: String,
    pub producer_tag: String,
    pub model_version: u32,
    pub session_count: u32,
    pub note_count: u64,
    pub genre_tags: Vec<String>,
    pub created_at: String,
    pub preview_hash: Option<String>,
    pub dna_percent: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

/// Result of verifying a .agdna file.
#[derive(Debug)]
pub struct VerifiedDna {
    pub is_valid: bool,
    pub error: Option<String>,
    pub envelope: DnaEnvelope,
    pub salt: Vec<u8>,
    pub nonce: Vec<u8>,
    pub auth_tag: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

/// Parameters for DNA export.
pub struct ExportParams {
    pub models: ModelExport,
    pub publisher_id: String,
    pub producer_tag: String,
    pub genre_tags: Vec<String>,
    pub master_secret: Vec<u8>,
}

/// Result of DNA export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub filename: String,
    pub envelope: DnaEnvelope,
}

/// Result of DNA import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub success: bool,
    pub error: Option<String>,
    pub envelope: Option<DnaEnvelope>,
}

// --- Session Analytics ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TempoPoint {
    pub minute: u32,
    pub avg_bpm: f64,
    pub min_bpm: f64,
    pub max_bpm: f64,
    pub stability: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyPoint {
    pub minute: u32,
    pub avg_velocity: u8,
    pub peak_velocity: u8,
    pub low_velocity: u8,
    pub event_count: u32,
    pub intensity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatmapGrid {
    /// 12 rows (pitch classes C-B) × 16 columns (16th-note slots).
    pub grid: [[u32; 16]; 12],
    pub top_cells: Vec<HeatmapCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatmapCell {
    pub pitch_class: u8,
    pub beat_slot: u8,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    #[serde(rename = "type")]
    pub anomaly_type: String,
    pub minute: u32,
    pub timestamp_ms: u64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub total_events: usize,
    pub duration_minutes: u32,
    pub avg_bpm: f64,
    pub bpm_drift: f64,
    pub avg_velocity: u8,
    pub unique_notes: usize,
    pub events_per_minute: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionAnalytics {
    pub tempo_arc: Vec<TempoPoint>,
    pub energy_map: Vec<EnergyPoint>,
    pub pattern_heatmap: HeatmapGrid,
    pub anomalies: Vec<Anomaly>,
    pub summary: Option<SessionSummary>,
}

// --- Storage ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub started_at: u64,
    pub ended_at: Option<u64>,
    pub bpm_start: Option<f64>,
    pub bpm_end: Option<f64>,
    pub key_detected: Option<u8>,
    pub mode_detected: Option<u8>,
    pub event_count: u32,
    pub genre: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub bpm_start: Option<f64>,
    pub bpm_end: Option<f64>,
    pub key: Option<u8>,
    pub mode: Option<u8>,
    pub event_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalStats {
    pub total_events: u64,
    pub total_sessions: u64,
    pub avg_bpm: Option<f64>,
    pub min_bpm: Option<f64>,
    pub max_bpm: Option<f64>,
    pub avg_velocity: Option<f64>,
    pub top_notes: Vec<NoteCount>,
    pub active_hours: Vec<HourCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteCount {
    pub note: u8,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourCount {
    pub hour_of_day: u8,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    pub pattern_hash: String,
    pub session_id: String,
    pub timestamp_ms: u64,
    pub note_sequence: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub session_id: String,
    pub snapshot: Vec<u8>,
    pub created_at: u64,
}

// --- DNA Profile (composite) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnaProfile {
    pub publisher_id: String,
    pub machine_id: String,
    pub total_events: u64,
    pub session_count: u32,
    pub dna_percent: u8,
    pub dna_stage: u8,
    pub harmony_contexts: usize,
    pub rhythm_contexts: usize,
    pub average_confidence: f64,
    pub global_stats: GlobalStats,
    pub sessions: Vec<Session>,
}

// --- Errors ---

#[derive(Debug, thiserror::Error)]
pub enum ArchergateError {
    #[error("storage error: {0}")]
    Storage(#[from] rusqlite::Error),

    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("invalid DNA file: {0}")]
    InvalidDna(String),

    #[error("engine error: {0}")]
    Engine(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, ArchergateError>;
