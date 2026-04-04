//! SQLite database layer — licenses, activations, API keys.

use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

/// Thread-safe database handle.
pub struct Db {
    pub(crate) conn: Mutex<Connection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    pub id: String,
    pub license_key: String,
    pub plugin_id: String,
    pub email: Option<String>,
    pub expires_at: Option<String>,
    pub max_machines: i32,
    pub created_at: String,
    pub api_key_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activation {
    pub id: String,
    pub license_id: String,
    pub machine_fingerprint: String,
    pub activated_at: String,
    pub last_seen_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub key_hash: String,
    pub developer_email: String,
    pub created_at: String,
}

impl Db {
    /// Open or create the database at the given path.
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
        }
        let conn = Connection::open(path).map_err(|e| format!("sqlite: {e}"))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| format!("pragma: {e}"))?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    /// Open an in-memory database (for testing).
    pub fn open_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| format!("sqlite: {e}"))?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")
            .map_err(|e| format!("pragma: {e}"))?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS api_keys (
                id              TEXT PRIMARY KEY,
                key_hash        TEXT UNIQUE NOT NULL,
                developer_email TEXT NOT NULL,
                created_at      TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS licenses (
                id            TEXT PRIMARY KEY,
                license_key   TEXT UNIQUE NOT NULL,
                plugin_id     TEXT NOT NULL,
                email         TEXT,
                expires_at    TEXT,
                max_machines  INTEGER NOT NULL DEFAULT 3,
                created_at    TEXT NOT NULL,
                api_key_id    TEXT NOT NULL REFERENCES api_keys(id)
            );

            CREATE INDEX IF NOT EXISTS idx_licenses_key ON licenses(license_key);
            CREATE INDEX IF NOT EXISTS idx_licenses_plugin ON licenses(plugin_id);

            CREATE TABLE IF NOT EXISTS activations (
                id                  TEXT PRIMARY KEY,
                license_id          TEXT NOT NULL REFERENCES licenses(id),
                machine_fingerprint TEXT NOT NULL,
                activated_at        TEXT NOT NULL,
                last_seen_at        TEXT NOT NULL,
                UNIQUE(license_id, machine_fingerprint)
            );
        ",
        )
        .map_err(|e| format!("migrate: {e}"))?;
        Ok(())
    }

    // ── API Keys ─────────────────────────────────────────────────────

    /// Create an API key. Returns the raw key (only shown once) and the DB record.
    pub fn create_api_key(&self, email: &str) -> Result<(String, ApiKey), String> {
        let raw_key = format!("ag_key_{}", uuid::Uuid::new_v4().simple());
        let key_hash = hash_key(&raw_key);
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO api_keys (id, key_hash, developer_email, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, key_hash, email, now],
        )
        .map_err(|e| format!("insert api_key: {e}"))?;

        Ok((
            raw_key,
            ApiKey {
                id,
                key_hash,
                developer_email: email.to_string(),
                created_at: now,
            },
        ))
    }

    /// Look up an API key by its raw value. Returns the record if valid.
    #[allow(dead_code)] // used by auth middleware in future, and tests
    pub fn verify_api_key(&self, raw_key: &str) -> Option<ApiKey> {
        let key_hash = hash_key(raw_key);
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, key_hash, developer_email, created_at FROM api_keys WHERE key_hash = ?1",
            params![key_hash],
            |row| {
                Ok(ApiKey {
                    id: row.get(0)?,
                    key_hash: row.get(1)?,
                    developer_email: row.get(2)?,
                    created_at: row.get(3)?,
                })
            },
        )
        .ok()
    }

    // ── Licenses ─────────────────────────────────────────────────────

    /// Create a license key for a plugin.
    pub fn create_license(
        &self,
        plugin_id: &str,
        email: Option<&str>,
        expires_at: Option<&str>,
        max_machines: i32,
        api_key_id: &str,
    ) -> Result<License, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let license_key = generate_license_key();
        let now = Utc::now().to_rfc3339();

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO licenses (id, license_key, plugin_id, email, expires_at, max_machines, created_at, api_key_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, license_key, plugin_id, email, expires_at, max_machines, now, api_key_id],
        )
        .map_err(|e| format!("insert license: {e}"))?;

        Ok(License {
            id,
            license_key,
            plugin_id: plugin_id.to_string(),
            email: email.map(|s| s.to_string()),
            expires_at: expires_at.map(|s| s.to_string()),
            max_machines,
            created_at: now,
            api_key_id: api_key_id.to_string(),
        })
    }

    /// Find a license by its key and plugin ID.
    pub fn find_license(&self, license_key: &str, plugin_id: &str) -> Option<License> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, license_key, plugin_id, email, expires_at, max_machines, created_at, api_key_id
             FROM licenses WHERE license_key = ?1 AND plugin_id = ?2",
            params![license_key, plugin_id],
            |row| {
                Ok(License {
                    id: row.get(0)?,
                    license_key: row.get(1)?,
                    plugin_id: row.get(2)?,
                    email: row.get(3)?,
                    expires_at: row.get(4)?,
                    max_machines: row.get(5)?,
                    created_at: row.get(6)?,
                    api_key_id: row.get(7)?,
                })
            },
        )
        .ok()
    }

    // ── Activations ──────────────────────────────────────────────────

    /// Count how many machines are activated for a license.
    pub fn activation_count(&self, license_id: &str) -> i32 {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM activations WHERE license_id = ?1",
            params![license_id],
            |row| row.get::<_, i32>(0),
        )
        .unwrap_or(0)
    }

    /// Check if a specific machine is already activated for a license.
    pub fn find_activation(&self, license_id: &str, machine_fp: &str) -> Option<Activation> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, license_id, machine_fingerprint, activated_at, last_seen_at
             FROM activations WHERE license_id = ?1 AND machine_fingerprint = ?2",
            params![license_id, machine_fp],
            |row| {
                Ok(Activation {
                    id: row.get(0)?,
                    license_id: row.get(1)?,
                    machine_fingerprint: row.get(2)?,
                    activated_at: row.get(3)?,
                    last_seen_at: row.get(4)?,
                })
            },
        )
        .ok()
    }

    /// Create or update an activation.
    pub fn upsert_activation(
        &self,
        license_id: &str,
        machine_fp: &str,
    ) -> Result<Activation, String> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();

        // Try update first
        let updated = conn
            .execute(
                "UPDATE activations SET last_seen_at = ?1 WHERE license_id = ?2 AND machine_fingerprint = ?3",
                params![now, license_id, machine_fp],
            )
            .map_err(|e| format!("update activation: {e}"))?;

        if updated > 0 {
            drop(conn);
            return self
                .find_activation(license_id, machine_fp)
                .ok_or_else(|| "activation disappeared".into());
        }

        // Insert new
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO activations (id, license_id, machine_fingerprint, activated_at, last_seen_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            params![id, license_id, machine_fp, now],
        )
        .map_err(|e| format!("insert activation: {e}"))?;

        Ok(Activation {
            id,
            license_id: license_id.to_string(),
            machine_fingerprint: machine_fp.to_string(),
            activated_at: now.clone(),
            last_seen_at: now,
        })
    }
}

/// SHA-256 hash of an API key for storage (never store raw keys).
fn hash_key(raw: &str) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(raw.as_bytes()))
}

/// Generate a formatted license key: XXXX-XXXX-XXXX-XXXX
fn generate_license_key() -> String {
    let id = uuid::Uuid::new_v4().simple().to_string().to_uppercase();
    format!(
        "{}-{}-{}-{}",
        &id[0..4],
        &id[4..8],
        &id[8..12],
        &id[12..16]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_verify_api_key() {
        let db = Db::open_memory().unwrap();
        let (raw_key, record) = db.create_api_key("dev@example.com").unwrap();
        assert!(raw_key.starts_with("ag_key_"));
        let found = db.verify_api_key(&raw_key).unwrap();
        assert_eq!(found.id, record.id);
        assert!(db.verify_api_key("wrong-key").is_none());
    }

    #[test]
    fn create_license_and_activate() {
        let db = Db::open_memory().unwrap();
        let (_, api_key) = db.create_api_key("dev@example.com").unwrap();
        let license = db
            .create_license("com.test.synth", Some("user@example.com"), None, 3, &api_key.id)
            .unwrap();

        assert_eq!(license.max_machines, 3);
        assert_eq!(db.activation_count(&license.id), 0);

        db.upsert_activation(&license.id, "fingerprint_a").unwrap();
        assert_eq!(db.activation_count(&license.id), 1);

        // Same machine again — should update, not duplicate
        db.upsert_activation(&license.id, "fingerprint_a").unwrap();
        assert_eq!(db.activation_count(&license.id), 1);

        // Different machine
        db.upsert_activation(&license.id, "fingerprint_b").unwrap();
        assert_eq!(db.activation_count(&license.id), 2);
    }

    #[test]
    fn license_key_format() {
        let key = generate_license_key();
        assert_eq!(key.len(), 19); // XXXX-XXXX-XXXX-XXXX
        assert_eq!(key.chars().filter(|c| *c == '-').count(), 3);
    }

    #[test]
    fn find_license_by_key_and_plugin() {
        let db = Db::open_memory().unwrap();
        let (_, api_key) = db.create_api_key("dev@example.com").unwrap();
        let license = db
            .create_license("com.test.synth", None, None, 3, &api_key.id)
            .unwrap();

        let found = db
            .find_license(&license.license_key, "com.test.synth")
            .unwrap();
        assert_eq!(found.id, license.id);

        assert!(db
            .find_license(&license.license_key, "com.other.plugin")
            .is_none());
        assert!(db.find_license("WRONG-KEY", "com.test.synth").is_none());
    }
}
