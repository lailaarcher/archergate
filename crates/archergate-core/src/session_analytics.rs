//! Session Analytics — the brain behind the Session Mirror.
//! Port of legacy/archergate-engine/src/session-analytics.js
//!
//! Owns: computing tempo arc, energy map, pattern heatmap, anomaly moments.
//! Does NOT: read from the database. Receives events as a slice.
//! Thread safety: pure functions, no shared state.

use std::collections::HashMap;
use crate::types::*;
use crate::storage::StoredEvent;

/// Compute all session analytics from stored events.
pub fn analyze_session(events: &[StoredEvent]) -> SessionAnalytics {
    if events.is_empty() {
        return SessionAnalytics {
            tempo_arc: vec![],
            energy_map: vec![],
            pattern_heatmap: HeatmapGrid { grid: [[0; 16]; 12], top_cells: vec![] },
            anomalies: vec![],
            summary: None,
        };
    }

    SessionAnalytics {
        tempo_arc: compute_tempo_arc(events),
        energy_map: compute_energy_map(events),
        pattern_heatmap: compute_pattern_heatmap(events),
        anomalies: detect_anomalies(events),
        summary: Some(compute_summary(events)),
    }
}

fn compute_tempo_arc(events: &[StoredEvent]) -> Vec<TempoPoint> {
    let mut by_minute: HashMap<u32, Vec<f64>> = HashMap::new();
    for e in events {
        by_minute.entry(e.session_minute).or_default().push(e.bpm);
    }

    let mut arc: Vec<TempoPoint> = by_minute.into_iter().map(|(minute, bpms)| {
        let avg = bpms.iter().sum::<f64>() / bpms.len() as f64;
        let min = bpms.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = bpms.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        TempoPoint {
            minute,
            avg_bpm: (avg * 10.0).round() / 10.0,
            min_bpm: min,
            max_bpm: max,
            stability: max - min,
        }
    }).collect();
    arc.sort_by_key(|p| p.minute);
    arc
}

fn compute_energy_map(events: &[StoredEvent]) -> Vec<EnergyPoint> {
    let mut by_minute: HashMap<u32, Vec<u8>> = HashMap::new();
    for e in events {
        by_minute.entry(e.session_minute).or_default().push(e.velocity);
    }

    let mut map: Vec<EnergyPoint> = by_minute.into_iter().map(|(minute, vels)| {
        let avg = vels.iter().map(|&v| v as f64).sum::<f64>() / vels.len() as f64;
        let peak = *vels.iter().max().unwrap_or(&0);
        let low = *vels.iter().min().unwrap_or(&0);
        let count = vels.len() as u32;
        EnergyPoint {
            minute,
            avg_velocity: avg.round() as u8,
            peak_velocity: peak,
            low_velocity: low,
            event_count: count,
            intensity: (avg * count as f64 / 10.0).round() as u32,
        }
    }).collect();
    map.sort_by_key(|p| p.minute);
    map
}

fn compute_pattern_heatmap(events: &[StoredEvent]) -> HeatmapGrid {
    let mut grid = [[0u32; 16]; 12];

    for e in events {
        let pc = (e.note % 12) as usize;
        let beat_slot = ((e.beat_position * 16.0).round() as usize) % 16;
        grid[pc][beat_slot] += 1;
    }

    let mut cells: Vec<HeatmapCell> = Vec::new();
    for (pc, row) in grid.iter().enumerate() {
        for (slot, &count) in row.iter().enumerate() {
            if count > 0 {
                cells.push(HeatmapCell {
                    pitch_class: pc as u8,
                    beat_slot: slot as u8,
                    count,
                });
            }
        }
    }
    cells.sort_by(|a, b| b.count.cmp(&a.count));
    let top_cells = cells.into_iter().take(20).collect();

    HeatmapGrid { grid, top_cells }
}

fn detect_anomalies(events: &[StoredEvent]) -> Vec<Anomaly> {
    let note_names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let mut anomalies: Vec<Anomaly> = Vec::new();
    let mut recent_notes: Vec<u8> = Vec::new();
    let mut pattern_counts: HashMap<String, u32> = HashMap::new();

    for (i, e) in events.iter().enumerate() {
        recent_notes.push(e.note);

        // Detect repeated loops (same 4-note sequence repeated)
        if recent_notes.len() >= 8 {
            let last4: String = recent_notes[recent_notes.len()-4..].iter()
                .map(|n| n.to_string()).collect::<Vec<_>>().join(",");
            let prev4: String = recent_notes[recent_notes.len()-8..recent_notes.len()-4].iter()
                .map(|n| n.to_string()).collect::<Vec<_>>().join(",");

            if last4 == prev4 {
                let count = pattern_counts.entry(last4).or_insert(0);
                *count += 1;
                if *count == 3 {
                    anomalies.push(Anomaly {
                        anomaly_type: "loop".into(),
                        minute: e.session_minute,
                        timestamp_ms: e.timestamp_ms,
                        description: format!("Looped the same 4-note pattern {} times", *count + 1),
                    });
                }
            }
        }

        // Detect large interval jumps (> octave)
        if i > 0 {
            let interval = (e.note as i16 - events[i - 1].note as i16).unsigned_abs() as u8;
            if interval > 12 {
                let note_name = note_names[(e.note % 12) as usize];
                let octave = (e.note / 12) as i8 - 1;
                anomalies.push(Anomaly {
                    anomaly_type: "new_territory".into(),
                    minute: e.session_minute,
                    timestamp_ms: e.timestamp_ms,
                    description: format!("Large interval jump to {}{} (+{} semitones)", note_name, octave, interval),
                });
            }
        }

        // Detect velocity extremes (sudden dynamics shift)
        if i > 2 {
            let avg_recent = (events[i-1].velocity as f64 + events[i-2].velocity as f64 + events[i-3].velocity as f64) / 3.0;
            let diff = (e.velocity as f64 - avg_recent).abs();
            if diff > 50.0 {
                let desc = if e.velocity as f64 > avg_recent {
                    format!("Sudden intensity spike (velocity {} vs avg {})", e.velocity, avg_recent.round())
                } else {
                    format!("Sudden drop to quiet (velocity {} vs avg {})", e.velocity, avg_recent.round())
                };
                anomalies.push(Anomaly {
                    anomaly_type: "dynamics_shift".into(),
                    minute: e.session_minute,
                    timestamp_ms: e.timestamp_ms,
                    description: desc,
                });
            }
        }

        // Detect deletions
        if e.was_deleted {
            anomalies.push(Anomaly {
                anomaly_type: "deletion".into(),
                minute: e.session_minute,
                timestamp_ms: e.timestamp_ms,
                description: "Deleted/undid a decision".into(),
            });
        }
    }

    // Detect flow states (long uninterrupted stretches)
    let mut flow_start: Option<u32> = None;
    let mut longest_flow = (0u32, 0u32, 0u32); // start, end, duration

    for i in 1..events.len() {
        let gap = events[i].timestamp_ms.saturating_sub(events[i-1].timestamp_ms);
        if gap < 5000 {
            if flow_start.is_none() {
                flow_start = Some(events[i-1].session_minute);
            }
        } else {
            if let Some(start) = flow_start {
                let duration = events[i-1].session_minute.saturating_sub(start);
                if duration > longest_flow.2 {
                    longest_flow = (start, events[i-1].session_minute, duration);
                }
            }
            flow_start = None;
        }
    }

    if longest_flow.2 >= 2 {
        anomalies.push(Anomaly {
            anomaly_type: "flow_state".into(),
            minute: longest_flow.0,
            timestamp_ms: events[0].timestamp_ms,
            description: format!("Longest uninterrupted flow: minute {}-{} ({} min)",
                longest_flow.0, longest_flow.1, longest_flow.2),
        });
    }

    // Deduplicate nearby anomalies (within 30 seconds)
    let mut deduped: Vec<Anomaly> = Vec::new();
    for a in &anomalies {
        let is_dupe = deduped.iter().any(|d| {
            d.anomaly_type == a.anomaly_type
                && (d.timestamp_ms as i64 - a.timestamp_ms as i64).unsigned_abs() < 30000
        });
        if !is_dupe {
            deduped.push(a.clone());
        }
    }
    deduped.sort_by_key(|a| a.timestamp_ms);
    deduped
}

fn compute_summary(events: &[StoredEvent]) -> SessionSummary {
    let duration = events.last().map(|e| e.session_minute).unwrap_or(0)
        - events.first().map(|e| e.session_minute).unwrap_or(0) + 1;
    let bpms: Vec<f64> = events.iter().map(|e| e.bpm).collect();
    let velocities: Vec<u8> = events.iter().map(|e| e.velocity).collect();
    let mut unique_notes = std::collections::HashSet::new();
    for e in events { unique_notes.insert(e.note); }

    let avg_bpm = bpms.iter().sum::<f64>() / bpms.len() as f64;
    let avg_vel = velocities.iter().map(|&v| v as f64).sum::<f64>() / velocities.len() as f64;
    let bpm_drift = bpms.last().unwrap_or(&0.0) - bpms.first().unwrap_or(&0.0);

    SessionSummary {
        total_events: events.len(),
        duration_minutes: duration,
        avg_bpm: (avg_bpm * 10.0).round() / 10.0,
        bpm_drift: (bpm_drift * 10.0).round() / 10.0,
        avg_velocity: avg_vel.round() as u8,
        unique_notes: unique_notes.len(),
        events_per_minute: (events.len() as f64 / duration.max(1) as f64).round() as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_all_four_panels() {
        let base = 1000u64;
        let events: Vec<StoredEvent> = (0..50).map(|i| StoredEvent {
            note: 36 + (i % 12) as u8,
            velocity: 60 + (i % 60) as u8,
            beat_position: (i % 16) as f64 / 16.0,
            bpm: 140.0 + if i > 25 { 2.0 } else { 0.0 },
            session_minute: i / 10,
            timestamp_ms: base + i as u64 * 500,
            was_deleted: i == 30,
            channel: 9,
            interval_val: 0,
        }).collect();

        let result = analyze_session(&events);
        assert!(!result.tempo_arc.is_empty());
        assert!(!result.energy_map.is_empty());
        assert!(result.pattern_heatmap.grid.iter().any(|row| row.iter().any(|&c| c > 0)));
        assert_eq!(result.summary.as_ref().map(|s| s.total_events), Some(50));
    }
}
