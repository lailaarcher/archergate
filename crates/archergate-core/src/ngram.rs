//! N-gram Engine with Exponential Decay.
//! Port of legacy/archergate-engine/src/ngram-engine.js
//!
//! Constraint: learn in real-time, predict in < 0.5ms, fully offline, < 50MB RAM.
//! Solution: trigram tables with decay. Simplest thing that satisfies it.

use std::collections::{HashMap, VecDeque};
use crate::types::{DecisionVector, ModelExport, ModelMeta, Prediction, TransitionTable, EngineSummary};
use crate::decision_vector::{quantize_beat, quantize_velocity, pitch_class};

/// One n-gram transition table.
/// Maps context keys (stringified tuples of recent events) to
/// weighted distributions over next values.
pub struct NgramTable {
    n: usize,
    decay: f64,
    table: HashMap<String, HashMap<String, f64>>,
    context: VecDeque<String>,
    pub total_observations: u64,
}

impl NgramTable {
    pub fn new(n: usize, decay: f64) -> Self {
        Self {
            n,
            decay,
            table: HashMap::new(),
            context: VecDeque::new(),
            total_observations: 0,
        }
    }

    /// Observe a new value in the stream.
    pub fn observe(&mut self, value: &str) {
        let val = value.to_string();

        if self.context.len() >= self.n - 1 {
            let start = self.context.len() - (self.n - 1);
            let key: String = self.context.iter().skip(start)
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("|");

            let dist = self.table.entry(key).or_default();

            // Decay all existing weights for this context
            for w in dist.values_mut() {
                *w *= self.decay;
            }

            // Increment this transition
            *dist.entry(val.clone()).or_insert(0.0) += 1.0;
        }

        self.context.push_back(val);
        // Keep context bounded
        if self.context.len() > self.n * 2 {
            let drain_count = self.context.len() - self.n;
            self.context.drain(..drain_count);
        }

        self.total_observations += 1;
    }

    /// Predict the top-k most likely next values.
    pub fn predict(&self, top_k: usize) -> Vec<(String, f64)> {
        if self.context.len() < self.n - 1 {
            return vec![];
        }

        let start = self.context.len() - (self.n - 1);
        let key: String = self.context.iter().skip(start)
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("|");

        let dist = match self.table.get(&key) {
            Some(d) if !d.is_empty() => d,
            _ => return vec![],
        };

        let total: f64 = dist.values().sum();

        let mut ranked: Vec<_> = dist.iter()
            .map(|(v, w)| (v.clone(), *w / total))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked.truncate(top_k);
        ranked
    }

    /// Average confidence of the top prediction across all contexts.
    pub fn average_confidence(&self) -> f64 {
        if self.table.is_empty() { return 0.0; }

        let mut total_conf = 0.0;
        let mut count = 0;

        for dist in self.table.values() {
            let total: f64 = dist.values().sum();
            let max = dist.values().cloned().fold(0.0f64, f64::max);
            if total > 0.0 {
                total_conf += max / total;
                count += 1;
            }
        }

        if count > 0 { total_conf / count as f64 } else { 0.0 }
    }

    pub fn context_count(&self) -> usize {
        self.table.len()
    }

    /// Export transition table as plain HashMap.
    pub fn export(&self) -> TransitionTable {
        self.table.iter().map(|(key, dist)| {
            let d: HashMap<String, f64> = dist.iter()
                .map(|(v, w)| (v.clone(), (*w * 1000.0).round() / 1000.0))
                .collect();
            (key.clone(), d)
        }).collect()
    }

    /// Import a transition table (from a purchased DNA file).
    pub fn import(&mut self, data: &TransitionTable) {
        self.table.clear();
        for (key, dist) in data {
            self.table.insert(key.clone(), dist.clone());
        }
    }

    /// Reset context without clearing learned patterns.
    pub fn reset_context(&mut self) {
        self.context.clear();
    }
}

/// The full prediction engine. Five n-gram sub-models.
pub struct PredictionEngine {
    pub rhythm: NgramTable,
    pub harmony: NgramTable,
    pub velocity: NgramTable,
    pub workflow: NgramTable,
    pub tempo: NgramTable,
    pub total_events: u64,
    pub session_count: u32,
    pub first_event_at: Option<u64>,
}

impl PredictionEngine {
    pub fn new(n: usize, decay: f64) -> Self {
        Self {
            rhythm: NgramTable::new(n, decay),
            harmony: NgramTable::new(n, decay),
            velocity: NgramTable::new(n, decay),
            workflow: NgramTable::new(n, decay),
            tempo: NgramTable::new(n, decay),
            total_events: 0,
            session_count: 0,
            first_event_at: None,
        }
    }

}

impl Default for PredictionEngine {
    fn default() -> Self {
        Self::new(3, 0.95)
    }
}

impl PredictionEngine {

    /// Observe a decision vector. Updates all five sub-models.
    pub fn observe(&mut self, vec: &DecisionVector) {
        if self.first_event_at.is_none() {
            self.first_event_at = Some(vec.timestamp_ms);
        }

        // Rhythm: beat position quantized to 16th notes
        self.rhythm.observe(&quantize_beat(vec.beat_position).to_string());

        // Harmony: pitch class + interval
        let harmony_key = format!("{}_{}", pitch_class(vec.note), vec.interval);
        self.harmony.observe(&harmony_key);

        // Velocity: quantized band
        self.velocity.observe(&quantize_velocity(vec.velocity).to_string());

        // Workflow: session minute bucket (5-min windows) + loop/delete flags
        let minute_bucket = vec.session_minute / 5;
        let workflow_val = format!("m{}_l{}_d{}",
            minute_bucket,
            if vec.looped_bar { 1 } else { 0 },
            if vec.was_deleted { 1 } else { 0 },
        );
        self.workflow.observe(&workflow_val);

        // Tempo: BPM rounded to nearest 5
        let bpm_rounded = ((vec.bpm / 5.0).round() * 5.0) as i32;
        self.tempo.observe(&bpm_rounded.to_string());

        self.total_events += 1;
    }

    /// Predict the most likely next note.
    pub fn predict(&self) -> Option<Vec<Prediction>> {
        let harmony_preds = self.harmony.predict(4);
        let velocity_preds = self.velocity.predict(1);
        let rhythm_preds = self.rhythm.predict(1);

        if harmony_preds.is_empty() { return None; }

        let predictions: Vec<Prediction> = harmony_preds.iter().map(|(value, probability)| {
            let parts: Vec<&str> = value.split('_').collect();
            let pc: u8 = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
            let note = 48 + pc; // middle octave estimate

            let vel_band: u8 = velocity_preds.first()
                .and_then(|(v, _): &(String, f64)| v.parse().ok())
                .unwrap_or(2);
            let velocity = [24u8, 52, 86, 115].get(vel_band as usize).copied().unwrap_or(86);

            let beat_slot: f64 = rhythm_preds.first()
                .and_then(|(v, _): &(String, f64)| v.parse::<f64>().ok())
                .unwrap_or(0.0);
            let beat_position = beat_slot / 16.0;

            Prediction { note, probability: *probability, velocity, beat_position }
        }).collect();

        Some(predictions)
    }

    /// DNA percentage (0-100).
    pub fn dna_percent(&self) -> u8 {
        let event_score = (self.total_events as f64 / 5000.0).min(1.0);
        let context_score = (self.harmony.context_count() as f64 / 200.0).min(1.0);
        let confidence_score = self.harmony.average_confidence();
        let session_score = (self.session_count as f64 / 15.0).min(1.0);
        let time_score = self.first_event_at
            .map(|first| {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                ((now - first) as f64 / (30.0 * 24.0 * 60.0 * 60.0 * 1000.0)).min(1.0)
            })
            .unwrap_or(0.0);

        let raw = event_score * 0.25
            + context_score * 0.20
            + confidence_score * 0.25
            + session_score * 0.20
            + time_score * 0.10;

        (raw * 100.0).round() as u8
    }

    /// DNA Stage (0-3).
    pub fn dna_stage(&self) -> u8 {
        let pct = self.dna_percent();
        if pct < 15 { 0 }
        else if pct < 50 { 1 }
        else if pct < 85 { 2 }
        else { 3 }
    }

    /// Start a new session.
    pub fn new_session(&mut self) {
        self.session_count += 1;
        self.rhythm.reset_context();
        self.harmony.reset_context();
        self.velocity.reset_context();
        self.workflow.reset_context();
        self.tempo.reset_context();
    }

    /// Export all sub-models for DNA packaging.
    pub fn export_models(&self) -> ModelExport {
        ModelExport {
            version: 1,
            rhythm: self.rhythm.export(),
            harmony: self.harmony.export(),
            velocity: self.velocity.export(),
            workflow: self.workflow.export(),
            tempo: self.tempo.export(),
            meta: ModelMeta {
                total_events: self.total_events,
                session_count: self.session_count,
                dna_percent: self.dna_percent(),
                exported_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
            },
        }
    }

    /// Import models from a DNA file.
    pub fn import_models(&mut self, data: &ModelExport) {
        self.rhythm.import(&data.rhythm);
        self.harmony.import(&data.harmony);
        self.velocity.import(&data.velocity);
        self.workflow.import(&data.workflow);
        self.tempo.import(&data.tempo);
        self.total_events = data.meta.total_events;
        self.session_count = data.meta.session_count;
    }

    pub fn summary(&self) -> EngineSummary {
        EngineSummary {
            total_events: self.total_events,
            session_count: self.session_count,
            dna_percent: self.dna_percent(),
            dna_stage: self.dna_stage(),
            harmony_contexts: self.harmony.context_count(),
            rhythm_contexts: self.rhythm.context_count(),
            average_confidence: (self.harmony.average_confidence() * 100.0).round() / 100.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_vec(note: u8, interval: i8, beat_position: f64) -> DecisionVector {
        DecisionVector {
            note, velocity: 100, beat_position, duration_ms: 100, bpm: 140.0,
            key: 0, mode: 0, interval, time_since_last_ms: 0, channel: 9,
            is_drum: true, session_minute: 0, hour_of_day: 14, looped_bar: false,
            was_deleted: false, timestamp_ms: 1000, session_id: "test".into(),
        }
    }

    #[test]
    fn learns_repeating_pattern_and_predicts() {
        let mut engine = PredictionEngine::new(3, 0.95);
        let pattern = [
            make_vec(36, 0, 0.0),
            make_vec(42, 6, 0.25),
            make_vec(38, -4, 0.5),
        ];

        for _ in 0..50 {
            for vec in &pattern { engine.observe(vec); }
        }

        let preds = engine.predict();
        assert!(preds.is_some());
        let preds = preds.unwrap();
        assert!(!preds.is_empty());
        assert!(preds[0].probability > 0.5);
    }

    #[test]
    fn converges_within_200_events() {
        let mut engine = PredictionEngine::new(3, 0.95);
        let notes = [
            make_vec(36, 0, 0.0),
            make_vec(42, 6, 0.25),
        ];

        let mut converged = false;
        for i in 0..200 {
            engine.observe(&notes[i % 2]);
            if i > 10 {
                if let Some(preds) = engine.predict() {
                    if !preds.is_empty() && preds[0].probability > 0.8 {
                        converged = true;
                        break;
                    }
                }
            }
        }
        assert!(converged, "Should converge within 200 events");
    }

    #[test]
    fn dna_percent_increases() {
        let mut engine = PredictionEngine::new(3, 0.95);
        let initial = engine.dna_percent();
        for i in 0..100 {
            engine.observe(&make_vec(36 + (i % 12) as u8, if i > 0 { 1 } else { 0 }, 0.0));
        }
        assert!(engine.dna_percent() > initial);
    }

    #[test]
    fn exports_and_imports_models() {
        let mut engine = PredictionEngine::new(3, 0.95);
        for i in 0..100u8 {
            engine.observe(&make_vec(36 + (i % 12), 0, 0.0));
        }

        let exported = engine.export_models();
        let mut engine2 = PredictionEngine::default();
        engine2.import_models(&exported);

        assert!(engine2.harmony.context_count() > 0);
        assert_eq!(engine2.total_events, engine.total_events);
    }

    #[test]
    fn latency_under_1ms() {
        let mut engine = PredictionEngine::new(3, 0.95);
        for i in 0..100u8 {
            engine.observe(&make_vec(36 + (i % 12), 0, 0.0));
        }

        let start = std::time::Instant::now();
        for i in 0..1000u32 {
            engine.observe(&make_vec(36 + (i % 12) as u8, 0, 0.0));
            engine.predict();
        }
        let per_event = start.elapsed().as_secs_f64() * 1000.0 / 1000.0;
        assert!(per_event < 1.0, "Expected < 1ms, got {:.3}ms", per_event);
    }
}
