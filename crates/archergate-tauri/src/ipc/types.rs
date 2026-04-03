//! IPC message types — the contract between plugin and engine.
//!
//! Owns: serialization format for WebSocket messages.
//! Does NOT: process messages or touch the engine.
//! Thread safety: all types are Send + Sync (plain data).
//! Tauri: v2 — no v1 API usage.

use serde::{Deserialize, Serialize};

/// Incoming message from the plugin (or UI test harness).
/// Each variant maps to a handler in the IPC server.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(dead_code)]
pub enum IncomingMessage {
    Note(NoteEvent),
    Detection(DetectionEvent),
    SessionStart { genre: Option<String> },
    SessionEnd,
    Predict,
    DnaProfile,
    SessionAnalytics { session_id: Option<String> },
    ExportDna { producer_tag: String, genre_tags: Vec<String> },
    ImportDna { agdna: String, label: String },
    SetBlend { label: String, blend: f64 },
    ClearBlend,
    ListModels,
    Provenance { notes: Vec<u8> },
    Summary,
    Reset,
}

/// A single MIDI note event from the plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteEvent {
    pub note: u8,
    pub velocity: u8,
    #[serde(default)]
    pub channel: u8,
    #[serde(rename = "timestampMs")]
    pub timestamp_ms: Option<u64>,
    pub bpm: Option<f64>,
    pub key: Option<u8>,
    pub mode: Option<u8>,
    #[serde(rename = "loopedBar")]
    pub looped_bar: Option<bool>,
    #[serde(rename = "wasDeleted")]
    pub was_deleted: Option<bool>,
    #[serde(rename = "durationMs")]
    pub duration_ms: Option<u64>,
    pub genre: Option<String>,
}

/// BPM/key detection update from the plugin.
#[derive(Debug, Clone, Deserialize)]
pub struct DetectionEvent {
    pub bpm: Option<f64>,
    pub key: Option<u8>,
    pub mode: Option<u8>,
}

/// Outgoing response to the plugin.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(dead_code)]
pub enum OutgoingMessage {
    Init(InitPayload),
    Note(PredictionResponse),
    Detection { ok: bool },
    SessionStart { session_id: String },
    SessionEnd(SessionEndPayload),
    Predict { predictions: Vec<PredictionItem> },
    DnaProfile(serde_json::Value),
    SessionAnalytics(serde_json::Value),
    ExportDna(serde_json::Value),
    ImportDna(serde_json::Value),
    SetBlend(serde_json::Value),
    ClearBlend { ok: bool },
    ListModels { models: Vec<String> },
    Provenance(serde_json::Value),
    Summary(serde_json::Value),
    Reset { ok: bool },
    Error { error: String },
}

/// Sent on initial WebSocket connection.
#[derive(Debug, Clone, Serialize)]
pub struct InitPayload {
    pub bpm: f64,
    pub dna: DnaStatus,
    pub total_transitions: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct DnaStatus {
    pub percent: u8,
    pub stage: u8,
    pub sessions: u32,
}

/// Response after processing a note event.
#[derive(Debug, Clone, Serialize)]
pub struct PredictionResponse {
    pub predictions: Vec<PredictionItem>,
    pub bpm: f64,
    pub key: u8,
    pub mode: u8,
    #[serde(rename = "dnaPercent")]
    pub dna_percent: u8,
    #[serde(rename = "dnaStage")]
    pub dna_stage: u8,
    #[serde(rename = "latencyMs")]
    pub latency_ms: f64,
    #[serde(rename = "sessionMinute")]
    pub session_minute: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct PredictionItem {
    pub note: u8,
    pub probability: f64,
    pub velocity: u8,
    #[serde(rename = "beatPosition")]
    pub beat_position: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionEndPayload {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub analytics: serde_json::Value,
}
