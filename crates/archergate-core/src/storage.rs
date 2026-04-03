//! Storage Layer — SQLite for sessions, events, provenance, snapshots.
//! Port of legacy/archergate-engine/src/storage.js
//!
//! Owns: all database reads and writes. Schema migration. Disk persistence.
//! Does NOT: interpret data or run analytics. That's session_analytics.rs.
//! Thread safety: Storage is NOT Send/Sync. Wrap in a Mutex if shared across tasks.
//!
//! Constraint: every decision a producer makes gets persisted.
//! Nothing is thrown away. The raw data is the ground truth;
//! the model is derived from it and can be rebuilt at any time.

use std::path::{Path, PathBuf};
use rusqlite::{Connection, params};
use crate::types::*;

pub struct Storage {
    conn: Connection,
    #[allow(dead_code)]
    path: PathBuf,
}

impl Storage {
    /// Open or create the ARCHERGATE database.
    /// Runs migrations inline — schema is always up to date after this call.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        let mut storage = Self {
            conn,
            path: path.to_path_buf(),
        };
        storage.migrate()?;
        Ok(storage)
    }

    /// Open an in-memory database (for testing).
    pub fn open_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let mut storage = Self {
            conn,
            path: PathBuf::from(":memory:"),
        };
        storage.migrate()?;
        Ok(storage)
    }

    fn migrate(&mut self) -> Result<()> {
        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS sessions (
                id            TEXT PRIMARY KEY,
                started_at    INTEGER NOT NULL,
                ended_at      INTEGER,
                bpm_start     REAL,
                bpm_end       REAL,
                key_detected  INTEGER,
                mode_detected INTEGER,
                event_count   INTEGER DEFAULT 0,
                genre         TEXT
            );

            CREATE TABLE IF NOT EXISTS events (
                id               INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id       TEXT NOT NULL,
                note             INTEGER NOT NULL,
                velocity         INTEGER NOT NULL,
                beat_position    REAL NOT NULL,
                duration_ms      INTEGER NOT NULL,
                bpm              REAL NOT NULL,
                key_detected     INTEGER NOT NULL,
                mode_detected    INTEGER NOT NULL,
                interval_val     INTEGER NOT NULL,
                time_since_last  INTEGER NOT NULL,
                channel          INTEGER NOT NULL,
                is_drum          INTEGER NOT NULL,
                session_minute   INTEGER NOT NULL,
                hour_of_day      INTEGER NOT NULL,
                looped_bar       INTEGER NOT NULL,
                was_deleted      INTEGER NOT NULL,
                timestamp_ms     INTEGER NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            );

            CREATE INDEX IF NOT EXISTS idx_events_session ON events(session_id);
            CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp_ms);

            CREATE TABLE IF NOT EXISTS model_snapshots (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                snapshot   BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            );

            CREATE TABLE IF NOT EXISTS provenance (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                pattern_hash TEXT NOT NULL,
                session_id   TEXT NOT NULL,
                timestamp_ms INTEGER NOT NULL,
                note_sequence TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            );

            CREATE INDEX IF NOT EXISTS idx_provenance_hash ON provenance(pattern_hash);
        ")?;
        Ok(())
    }

    // --- Sessions ---

    pub fn start_session(&self, genre: Option<&str>) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_ms();
        self.conn.execute(
            "INSERT INTO sessions (id, started_at, genre) VALUES (?1, ?2, ?3)",
            params![id, now, genre],
        )?;
        Ok(id)
    }

    pub fn end_session(&self, session_id: &str, stats: &SessionStats) -> Result<()> {
        let now = now_ms();
        self.conn.execute(
            "UPDATE sessions SET ended_at = ?1, bpm_start = ?2, bpm_end = ?3,
             key_detected = ?4, mode_detected = ?5, event_count = ?6 WHERE id = ?7",
            params![
                now, stats.bpm_start, stats.bpm_end,
                stats.key, stats.mode, stats.event_count, session_id
            ],
        )?;
        Ok(())
    }

    // --- Events ---

    pub fn insert_event(&self, vec: &DecisionVector) -> Result<()> {
        self.conn.execute(
            "INSERT INTO events (
                session_id, note, velocity, beat_position, duration_ms,
                bpm, key_detected, mode_detected, interval_val, time_since_last,
                channel, is_drum, session_minute, hour_of_day,
                looped_bar, was_deleted, timestamp_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                vec.session_id, vec.note, vec.velocity, vec.beat_position, vec.duration_ms,
                vec.bpm, vec.key, vec.mode, vec.interval, vec.time_since_last_ms,
                vec.channel, vec.is_drum as i32, vec.session_minute, vec.hour_of_day,
                vec.looped_bar as i32, vec.was_deleted as i32, vec.timestamp_ms
            ],
        )?;
        Ok(())
    }

    pub fn insert_events(&mut self, events: &[DecisionVector]) -> Result<()> {
        let tx = self.conn.transaction()?;
        for vec in events {
            tx.execute(
                "INSERT INTO events (
                    session_id, note, velocity, beat_position, duration_ms,
                    bpm, key_detected, mode_detected, interval_val, time_since_last,
                    channel, is_drum, session_minute, hour_of_day,
                    looped_bar, was_deleted, timestamp_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                params![
                    vec.session_id, vec.note, vec.velocity, vec.beat_position, vec.duration_ms,
                    vec.bpm, vec.key, vec.mode, vec.interval, vec.time_since_last_ms,
                    vec.channel, vec.is_drum as i32, vec.session_minute, vec.hour_of_day,
                    vec.looped_bar as i32, vec.was_deleted as i32, vec.timestamp_ms
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_session_events(&self, session_id: &str) -> Result<Vec<StoredEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM events WHERE session_id = ?1 ORDER BY timestamp_ms"
        )?;
        let rows = stmt.query_map(params![session_id], |row| {
            Ok(StoredEvent {
                note: row.get("note")?,
                velocity: row.get("velocity")?,
                beat_position: row.get("beat_position")?,
                bpm: row.get("bpm")?,
                session_minute: row.get("session_minute")?,
                timestamp_ms: row.get("timestamp_ms")?,
                was_deleted: row.get::<_, i32>("was_deleted")? != 0,
                channel: row.get("channel")?,
                interval_val: row.get("interval_val")?,
            })
        })?;
        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }

    // --- Provenance ---

    pub fn record_provenance(&self, pattern_hash: &str, session_id: &str, note_sequence: &[u8]) -> Result<()> {
        let now = now_ms();
        let seq_json = serde_json::to_string(note_sequence)?;
        self.conn.execute(
            "INSERT INTO provenance (pattern_hash, session_id, timestamp_ms, note_sequence) VALUES (?1, ?2, ?3, ?4)",
            params![pattern_hash, session_id, now, seq_json],
        )?;
        Ok(())
    }

    pub fn lookup_provenance(&self, pattern_hash: &str) -> Result<Option<ProvenanceRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM provenance WHERE pattern_hash = ?1 ORDER BY timestamp_ms ASC LIMIT 1"
        )?;
        let mut rows = stmt.query_map(params![pattern_hash], |row| {
            let seq_str: String = row.get("note_sequence")?;
            let note_sequence: Vec<u8> = serde_json::from_str(&seq_str).unwrap_or_default();
            Ok(ProvenanceRecord {
                pattern_hash: row.get("pattern_hash")?,
                session_id: row.get("session_id")?,
                timestamp_ms: row.get("timestamp_ms")?,
                note_sequence,
            })
        })?;
        match rows.next() {
            Some(Ok(record)) => Ok(Some(record)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    // --- Model Snapshots ---

    pub fn save_snapshot(&self, session_id: &str, snapshot: &[u8]) -> Result<()> {
        let now = now_ms();
        self.conn.execute(
            "INSERT INTO model_snapshots (session_id, snapshot, created_at) VALUES (?1, ?2, ?3)",
            params![session_id, snapshot, now],
        )?;
        Ok(())
    }

    pub fn latest_snapshot(&self) -> Result<Option<Snapshot>> {
        let mut stmt = self.conn.prepare(
            "SELECT session_id, snapshot, created_at FROM model_snapshots ORDER BY created_at DESC LIMIT 1"
        )?;
        let mut rows = stmt.query_map([], |row| {
            Ok(Snapshot {
                session_id: row.get(0)?,
                snapshot: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?;
        match rows.next() {
            Some(Ok(snap)) => Ok(Some(snap)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    // --- Session Analytics Queries ---

    pub fn all_sessions(&self) -> Result<Vec<Session>> {
        let mut stmt = self.conn.prepare("SELECT * FROM sessions ORDER BY started_at DESC")?;
        let rows = stmt.query_map([], |row| {
            Ok(Session {
                id: row.get("id")?,
                started_at: row.get("started_at")?,
                ended_at: row.get("ended_at")?,
                bpm_start: row.get("bpm_start")?,
                bpm_end: row.get("bpm_end")?,
                key_detected: row.get("key_detected")?,
                mode_detected: row.get("mode_detected")?,
                event_count: row.get::<_, Option<u32>>("event_count")?.unwrap_or(0),
                genre: row.get("genre")?,
            })
        })?;
        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row?);
        }
        Ok(sessions)
    }

    pub fn total_session_count(&self) -> Result<u32> {
        let count: u32 = self.conn.query_row(
            "SELECT COUNT(*) FROM sessions", [], |row| row.get(0)
        )?;
        Ok(count)
    }

    pub fn global_stats(&self) -> Result<GlobalStats> {
        let (total_events, total_sessions, avg_bpm, min_bpm, max_bpm, avg_velocity) = self.conn.query_row(
            "SELECT COUNT(*) as te, COUNT(DISTINCT session_id) as ts,
                    AVG(bpm), MIN(bpm), MAX(bpm), AVG(velocity) FROM events",
            [],
            |row| Ok((
                row.get::<_, u64>(0)?,
                row.get::<_, u64>(1)?,
                row.get::<_, Option<f64>>(2)?,
                row.get::<_, Option<f64>>(3)?,
                row.get::<_, Option<f64>>(4)?,
                row.get::<_, Option<f64>>(5)?,
            )),
        )?;

        let mut top_stmt = self.conn.prepare(
            "SELECT note, COUNT(*) as count FROM events GROUP BY note ORDER BY count DESC LIMIT 10"
        )?;
        let top_notes: Vec<NoteCount> = top_stmt.query_map([], |row| {
            Ok(NoteCount { note: row.get(0)?, count: row.get(1)? })
        })?.filter_map(|r| r.ok()).collect();

        let mut hour_stmt = self.conn.prepare(
            "SELECT hour_of_day, COUNT(*) as count FROM events GROUP BY hour_of_day ORDER BY count DESC LIMIT 5"
        )?;
        let active_hours: Vec<HourCount> = hour_stmt.query_map([], |row| {
            Ok(HourCount { hour_of_day: row.get(0)?, count: row.get(1)? })
        })?.filter_map(|r| r.ok()).collect();

        Ok(GlobalStats {
            total_events, total_sessions,
            avg_bpm, min_bpm, max_bpm, avg_velocity,
            top_notes, active_hours,
        })
    }

    pub fn close(self) {
        drop(self.conn);
    }
}

/// Stored event row — subset of fields needed by session analytics.
#[derive(Debug, Clone)]
pub struct StoredEvent {
    pub note: u8,
    pub velocity: u8,
    pub beat_position: f64,
    pub bpm: f64,
    pub session_minute: u32,
    pub timestamp_ms: u64,
    pub was_deleted: bool,
    pub channel: u8,
    pub interval_val: i8,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0) // safe: system clock is always after epoch
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_vec(session_id: &str, note: u8, i: u32) -> DecisionVector {
        DecisionVector {
            note, velocity: 80 + (i % 40) as u8, beat_position: (i % 16) as f64 / 16.0,
            duration_ms: 100, bpm: 140.0, key: 0, mode: 0,
            interval: if i > 0 { 1 } else { 0 }, time_since_last_ms: 100,
            channel: 9, is_drum: true, session_minute: i / 10,
            hour_of_day: 14, looped_bar: false, was_deleted: false,
            timestamp_ms: 1000 + (i as u64) * 100,
            session_id: session_id.to_string(),
        }
    }

    #[test]
    fn creates_session_and_inserts_events() {
        let storage = Storage::open_memory().expect("open");
        let sid = storage.start_session(Some("trap")).expect("start");

        storage.insert_event(&make_vec(&sid, 36, 0)).expect("insert");

        let events = storage.get_session_events(&sid).expect("get");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].note, 36);
    }

    #[test]
    fn batch_inserts_100_events() {
        let mut storage = Storage::open_memory().expect("open");
        let sid = storage.start_session(Some("drill")).expect("start");

        let events: Vec<DecisionVector> = (0..100)
            .map(|i| make_vec(&sid, 36 + (i % 12) as u8, i))
            .collect();
        storage.insert_events(&events).expect("batch");

        assert_eq!(storage.get_session_events(&sid).expect("get").len(), 100);
    }

    #[test]
    fn tracks_provenance() {
        let storage = Storage::open_memory().expect("open");
        let sid = storage.start_session(None).expect("start");

        storage.record_provenance("abc123", &sid, &[36, 42, 38, 42]).expect("record");
        let record = storage.lookup_provenance("abc123").expect("lookup");
        assert!(record.is_some());
    }

    #[test]
    fn returns_global_stats() {
        let mut storage = Storage::open_memory().expect("open");
        let sid = storage.start_session(Some("trap")).expect("start");

        let events: Vec<DecisionVector> = (0..50)
            .map(|i| make_vec(&sid, 36 + (i % 12) as u8, i))
            .collect();
        storage.insert_events(&events).expect("batch");

        let stats = storage.global_stats().expect("stats");
        assert!(stats.total_events > 0);
        assert!(!stats.top_notes.is_empty());
    }

    #[test]
    fn saves_and_loads_snapshot() {
        let storage = Storage::open_memory().expect("open");
        let sid = storage.start_session(None).expect("start");

        let data = b"model snapshot data";
        storage.save_snapshot(&sid, data).expect("save");

        let snap = storage.latest_snapshot().expect("load");
        assert!(snap.is_some());
        assert_eq!(snap.as_ref().map(|s| s.session_id.as_str()), Some(sid.as_str()));
    }
}
