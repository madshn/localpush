//! SQLite-based application configuration store

use rusqlite::{Connection, params};
use std::sync::Mutex;
use crate::traits::LedgerError;

pub struct AppConfig {
    conn: Mutex<Connection>,
}

impl AppConfig {
    /// Create config table in an existing database connection
    pub fn init_table(conn: &Connection) -> Result<(), LedgerError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS app_config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );"
        ).map_err(|e| LedgerError::DatabaseError(e.to_string()))
    }

    /// Open standalone in-memory config (for testing)
    pub fn open_in_memory() -> Result<Self, LedgerError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| LedgerError::DatabaseError(e.to_string()))?;
        Self::init_table(&conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Wrap an existing connection (config table must already be initialized)
    pub fn from_connection(conn: Connection) -> Self {
        Self { conn: Mutex::new(conn) }
    }

    pub fn get(&self, key: &str) -> Result<Option<String>, LedgerError> {
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO app_config (key, value, updated_at) VALUES (?1, ?2, ?3)",
            params![key, value, now],
        ).map_err(|e| LedgerError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    pub fn delete(&self, key: &str) -> Result<(), LedgerError> {
        tracing::debug!(key = %key, "Config delete");
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM app_config WHERE key = ?1",
            params![key],
        ).map_err(|e| LedgerError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    pub fn get_bool(&self, key: &str) -> Result<bool, LedgerError> {
        Ok(self.get(key)?.map(|v| v == "true").unwrap_or(false))
    }

    /// Get all key-value pairs where the key starts with the given prefix
    pub fn get_by_prefix(&self, prefix: &str) -> Result<Vec<(String, String)>, LedgerError> {
        let conn = self.conn.lock().unwrap();
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

        config.set("webhook_url", "https://example.com/webhook").unwrap();
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
