//! Decision Vector — transforms raw MIDI into rich context vectors.
//! Port of legacy/archergate-engine/src/decision-vector.js
//!
//! The note alone is nearly worthless. The full context of when, how hard,
//! how long, what came before, and where in the session — that builds the fingerprint.

use crate::types::{DecisionVector, EventContext, RawMidiEvent};

/// Mutable session state held between vectorize calls.
pub struct SessionState {
    prev: Option<DecisionVector>,
    session_start: u64,
    session_id: String,
    current_bpm: f64,
    current_key: u8,
    current_mode: u8,
}

impl Default for SessionState {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            prev: None,
            session_start: 0,
            session_id: String::new(),
            current_bpm: 120.0,
            current_key: 0,
            current_mode: 0,
        }
    }

    /// Start a new session. Resets context.
    pub fn start_session(&mut self, session_id: String, now: u64) {
        self.prev = None;
        self.session_start = now;
        self.session_id = session_id;
        self.current_bpm = 120.0;
        self.current_key = 0;
        self.current_mode = 0;
    }

    /// Update session-level state (BPM, key detection results).
    pub fn update_state(&mut self, bpm: Option<f64>, key: Option<u8>, mode: Option<u8>) {
        if let Some(b) = bpm { self.current_bpm = b; }
        if let Some(k) = key { self.current_key = k; }
        if let Some(m) = mode { self.current_mode = m; }
    }

    /// Transform a raw MIDI event into a full decision vector.
    pub fn vectorize(&mut self, raw: &RawMidiEvent, context: &EventContext) -> DecisionVector {
        let now = raw.timestamp_ms;
        let bpm = context.bpm.unwrap_or(self.current_bpm);
        let key = context.key.unwrap_or(self.current_key);
        let mode = context.mode.unwrap_or(self.current_mode);

        // Beat position: where in the current beat (0.0 - 1.0)
        let beat_length_ms = 60000.0 / bpm;
        let elapsed = (now - self.session_start) as f64;
        let beat_position = (elapsed % beat_length_ms) / beat_length_ms;

        // Interval from previous note
        let interval = match &self.prev {
            Some(p) => (raw.note as i8) - (p.note as i8),
            None => 0,
        };

        // Time since last event
        let time_since_last_ms = match &self.prev {
            Some(p) => now - p.timestamp_ms,
            None => 0,
        };

        // Session depth
        let session_minute = ((now - self.session_start) / 60000) as u32;

        // Time of day — use chrono for local hour
        let hour_of_day = {
            let secs = (now / 1000) as i64;
            chrono::DateTime::from_timestamp(secs, 0)
                .map(|dt| {
                    use chrono::Timelike;
                    dt.with_timezone(&chrono::Local).hour() as u8
                })
                .unwrap_or(12)
        };

        let vec = DecisionVector {
            note: raw.note,
            velocity: raw.velocity,
            beat_position: (beat_position * 1000.0).round() / 1000.0, // 3 decimal precision
            duration_ms: context.duration_ms.unwrap_or(0),
            bpm,
            key,
            mode,
            interval,
            time_since_last_ms,
            channel: raw.channel,
            is_drum: raw.channel == 9,
            session_minute,
            hour_of_day,
            looped_bar: context.looped_bar.unwrap_or(false),
            was_deleted: context.was_deleted.unwrap_or(false),
            timestamp_ms: now,
            session_id: self.session_id.clone(),
        };

        self.prev = Some(vec.clone());
        vec
    }
}

/// Quantize beat position into 16th-note slots (0-15).
pub fn quantize_beat(beat_position: f64) -> u8 {
    (((beat_position * 16.0).round() as i32) % 16) as u8
}

/// Quantize velocity into 4 bands: ghost (0), soft (1), medium (2), hard (3).
pub fn quantize_velocity(velocity: u8) -> u8 {
    if velocity < 32 { 0 }
    else if velocity < 72 { 1 }
    else if velocity < 100 { 2 }
    else { 3 }
}

/// Get pitch class (0-11) from MIDI note.
pub fn pitch_class(note: u8) -> u8 {
    note % 12
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vectorizes_a_raw_midi_event() {
        let mut state = SessionState::new();
        state.start_session("test-1".into(), 1000);

        let raw = RawMidiEvent { note: 36, velocity: 100, channel: 9, timestamp_ms: 1000 };
        let ctx = EventContext { bpm: Some(140.0), key: Some(6), mode: Some(1), ..Default::default() };
        let vec = state.vectorize(&raw, &ctx);

        assert_eq!(vec.note, 36);
        assert_eq!(vec.velocity, 100);
        assert!(vec.is_drum);
        assert_eq!(vec.interval, 0);
    }

    #[test]
    fn computes_interval_from_previous() {
        let mut state = SessionState::new();
        let now = 1000u64;
        state.start_session("test-2".into(), now);

        let ctx = EventContext::default();
        state.vectorize(&RawMidiEvent { note: 36, velocity: 80, channel: 0, timestamp_ms: now }, &ctx);
        let vec2 = state.vectorize(&RawMidiEvent { note: 42, velocity: 90, channel: 0, timestamp_ms: now + 100 }, &ctx);

        assert_eq!(vec2.interval, 6);
    }

    #[test]
    fn quantizes_correctly() {
        assert_eq!(quantize_beat(0.0), 0);
        assert_eq!(quantize_beat(0.25), 4);
        assert_eq!(quantize_velocity(10), 0);
        assert_eq!(quantize_velocity(120), 3);
        assert_eq!(pitch_class(60), 0);
        assert_eq!(pitch_class(69), 9);
    }
}
