//! SQLite-based application configuration store

use crate::traits::LedgerError;
use rusqlite::{params, Connection, OpenFlags};
use std::path::Path;
use std::sync::Mutex;

pub struct AppConfig {
    writer: Mutex<Connection>,
    reader: Mutex<Connection>,
}

fn apply_writer_pragmas(conn: &Connection) -> Result<(), LedgerError> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 5000;",
    )
    .map_err(|e| LedgerError::DatabaseError(e.to_string()))
}

fn apply_reader_pragmas(conn: &Connection) -> Result<(), LedgerError> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA busy_timeout = 5000;
         PRAGMA query_only = ON;",
    )
    .map_err(|e| LedgerError::DatabaseError(e.to_string()))
}

fn init_table_on(conn: &Connection) -> Result<(), LedgerError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS app_config (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        );",
    )
    .map_err(|e| LedgerError::DatabaseError(e.to_string()))
}

impl AppConfig {
    /// Create config table in an existing database connection (legacy helper used by tests/migrations)
    pub fn init_table(conn: &Connection) -> Result<(), LedgerError> {
        init_table_on(conn)
    }

    /// Open or create a config database at the given path.
    pub fn open(path: &Path) -> Result<Self, LedgerError> {
        let writer =
            Connection::open(path).map_err(|e| LedgerError::DatabaseError(e.to_string()))?;
        apply_writer_pragmas(&writer)?;
        init_table_on(&writer)?;

        let reader =
            Connection::open(path).map_err(|e| LedgerError::DatabaseError(e.to_string()))?;
        apply_reader_pragmas(&reader)?;

        Ok(Self {
            writer: Mutex::new(writer),
            reader: Mutex::new(reader),
        })
    }

    /// Open standalone in-memory config (for testing).
    ///
    /// Uses a unique named shared-cache URI so both connections see the same in-memory database
    /// while remaining isolated from other test instances running in parallel.
    pub fn open_in_memory() -> Result<Self, LedgerError> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let uri = format!("file:config_mem_{}?mode=memory&cache=shared", id);

        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_SHARED_CACHE;

        let writer = Connection::open_with_flags(&uri, flags)
            .map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        writer
            .execute_batch("PRAGMA busy_timeout = 5000;")
            .map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        init_table_on(&writer)?;

        let reader = Connection::open_with_flags(&uri, flags)
            .map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        reader
            .execute_batch(
                "PRAGMA busy_timeout = 5000;
             PRAGMA query_only = ON;",
            )
            .map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        Ok(Self {
            writer: Mutex::new(writer),
            reader: Mutex::new(reader),
        })
    }

    pub fn get(&self, key: &str) -> Result<Option<String>, LedgerError> {
        let conn = self.reader.lock().unwrap();
        let result = conn.query_row(
            "SELECT value FROM app_config WHERE key = ?1",
            params![key],
            |row| row.get(0),
        );
        let ret = match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(LedgerError::DatabaseError(e.to_string())),
        };
        tracing::debug!(key = %key, found = ret.as_ref().ok().and_then(|v| v.as_ref()).is_some(), "Config get");
        ret
    }

    pub fn set(&self, key: &str, value: &str) -> Result<(), LedgerError> {
        tracing::debug!(key = %key, "Config set");
        let now = chrono::Utc::now().timestamp();
        let conn = self.writer.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO app_config (key, value, updated_at) VALUES (?1, ?2, ?3)",
            params![key, value, now],
        )
        .map_err(|e| LedgerError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    pub fn delete(&self, key: &str) -> Result<(), LedgerError> {
        tracing::debug!(key = %key, "Config delete");
        let conn = self.writer.lock().unwrap();
        conn.execute("DELETE FROM app_config WHERE key = ?1", params![key])
            .map_err(|e| LedgerError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    pub fn get_bool(&self, key: &str) -> Result<bool, LedgerError> {
        Ok(self.get(key)?.map(|v| v == "true").unwrap_or(false))
    }

    /// Get all key-value pairs where the key starts with the given prefix
    pub fn get_by_prefix(&self, prefix: &str) -> Result<Vec<(String, String)>, LedgerError> {
        let conn = self.reader.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT key, value FROM app_config WHERE key LIKE ?1")
            .map_err(|e| LedgerError::DatabaseError(e.to_string()))?;
        let rows = stmt
            .query_map([format!("{}%", prefix)], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| LedgerError::DatabaseError(e.to_string()))?);
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get() {
        let config = AppConfig::open_in_memory().unwrap();

        config
            .set("webhook_url", "https://example.com/webhook")
            .unwrap();
        let value = config.get("webhook_url").unwrap();

        assert_eq!(value, Some("https://example.com/webhook".to_string()));
    }

    #[test]
    fn test_get_missing_key() {
        let config = AppConfig::open_in_memory().unwrap();

        let value = config.get("nonexistent_key").unwrap();

        assert_eq!(value, None);
    }

    #[test]
    fn test_delete() {
        let config = AppConfig::open_in_memory().unwrap();

        config.set("temp_key", "temp_value").unwrap();
        assert!(config.get("temp_key").unwrap().is_some());

        config.delete("temp_key").unwrap();
        let value = config.get("temp_key").unwrap();

        assert_eq!(value, None);
    }

    #[test]
    fn test_set_overwrites() {
        let config = AppConfig::open_in_memory().unwrap();

        config.set("key", "original").unwrap();
        config.set("key", "updated").unwrap();

        let value = config.get("key").unwrap();

        assert_eq!(value, Some("updated".to_string()));
    }

    #[test]
    fn test_get_by_prefix() {
        let config = AppConfig::open_in_memory().unwrap();

        config.set("binding.src1.ep1", "value1").unwrap();
        config.set("binding.src1.ep2", "value2").unwrap();
        config.set("binding.src2.ep3", "value3").unwrap();
        config.set("other.key", "unrelated").unwrap();

        let results = config.get_by_prefix("binding.src1.").unwrap();
        assert_eq!(results.len(), 2);

        let all_bindings = config.get_by_prefix("binding.").unwrap();
        assert_eq!(all_bindings.len(), 3);

        let empty = config.get_by_prefix("nonexistent.").unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn test_get_bool() {
        let config = AppConfig::open_in_memory().unwrap();

        // Test true value
        config.set("enabled", "true").unwrap();
        assert!(config.get_bool("enabled").unwrap());

        // Test false value
        config.set("enabled", "false").unwrap();
        assert!(!config.get_bool("enabled").unwrap());

        // Test missing key (defaults to false)
        assert!(!config.get_bool("missing").unwrap());

        // Test non-boolean value (defaults to false)
        config.set("enabled", "not_a_bool").unwrap();
        assert!(!config.get_bool("enabled").unwrap());
    }
}
