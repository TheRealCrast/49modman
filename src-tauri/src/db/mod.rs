use rusqlite::{params, Connection, OptionalExtension};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::error::InternalError;

const MIGRATION_0001: &str = include_str!("../../migrations/0001_init.sql");

pub fn migrate(connection: &Connection) -> Result<(), InternalError> {
    connection.execute_batch(MIGRATION_0001)?;
    Ok(())
}

pub fn seed_defaults(connection: &Connection) -> Result<(), InternalError> {
    let now = now_rfc3339()?;

    connection.execute(
        "INSERT OR IGNORE INTO settings (key, value_json, updated_at) VALUES (?1, ?2, ?3)",
        params!["warning.red", serde_json::to_string(&true)?, now.clone()],
    )?;
    connection.execute(
        "INSERT OR IGNORE INTO settings (key, value_json, updated_at) VALUES (?1, ?2, ?3)",
        params!["warning.broken", serde_json::to_string(&true)?, now],
    )?;

    Ok(())
}

pub fn upsert_setting(
    connection: &Connection,
    key: &str,
    value_json: &str,
    updated_at: &str,
) -> Result<(), InternalError> {
    connection.execute(
        "INSERT INTO settings (key, value_json, updated_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
        params![key, value_json, updated_at],
    )?;

    Ok(())
}

pub fn get_setting(connection: &Connection, key: &str) -> Result<Option<String>, InternalError> {
    Ok(connection
        .query_row(
            "SELECT value_json FROM settings WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()?)
}

pub fn now_rfc3339() -> Result<String, InternalError> {
    Ok(OffsetDateTime::now_utc().format(&Rfc3339)?)
}
