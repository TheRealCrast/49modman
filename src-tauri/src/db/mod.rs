use rusqlite::{params, Connection, OptionalExtension};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::error::InternalError;

const MIGRATION_0001: &str = include_str!("../../migrations/0001_init.sql");
const MIGRATION_0002: &str = include_str!("../../migrations/0002_catalog_indexes.sql");
const MIGRATION_0003: &str = include_str!("../../migrations/0003_profiles.sql");

pub fn migrate(connection: &Connection) -> Result<(), InternalError> {
    connection.execute_batch(MIGRATION_0001)?;
    connection.execute_batch(MIGRATION_0002)?;
    connection.execute_batch(MIGRATION_0003)?;
    repair_profiles_schema(connection)?;
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

    connection.execute(
        "INSERT OR IGNORE INTO profiles (
            id,
            name,
            notes,
            game_path,
            launch_mode_default,
            created_at,
            updated_at,
            last_played_at,
            is_builtin_default
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, 1)",
        params![
            "default",
            "Default",
            "Built-in fallback profile.",
            "",
            "steam",
            now_rfc3339()?,
            now_rfc3339()?
        ],
    )?;

    let active_profile_id = connection
        .query_row(
            "SELECT value_json FROM settings WHERE key = 'profiles.active_id'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    let active_profile_id = active_profile_id
        .and_then(|value| serde_json::from_str::<String>(&value).ok())
        .filter(|profile_id| profile_exists(connection, profile_id).unwrap_or(false))
        .unwrap_or_else(|| "default".to_string());

    upsert_setting(
        connection,
        "profiles.active_id",
        &serde_json::to_string(&active_profile_id)?,
        &now_rfc3339()?,
    )?;

    Ok(())
}

pub fn reset_user_data(connection: &Connection) -> Result<(), InternalError> {
    connection.execute_batch(
        "
        DELETE FROM download_jobs;
        DELETE FROM install_tasks;
        DELETE FROM profile_mod_dependencies;
        DELETE FROM profile_mods;
        DELETE FROM local_mods;
        DELETE FROM cached_archives;
        DELETE FROM reference_overrides;
        DELETE FROM package_versions;
        DELETE FROM packages;
        DELETE FROM profiles;
        DELETE FROM settings;
        DELETE FROM sync_state;
        ",
    )?;

    seed_defaults(connection)
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

fn profile_exists(connection: &Connection, profile_id: &str) -> Result<bool, InternalError> {
    Ok(connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM profiles WHERE id = ?1)",
            params![profile_id],
            |row| row.get::<_, i64>(0),
        )?
        != 0)
}

fn repair_profiles_schema(connection: &Connection) -> Result<(), InternalError> {
    let mut statement = connection.prepare("PRAGMA table_info(profiles)")?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;

    if columns.is_empty() {
        return Ok(());
    }

    if !columns.iter().any(|column| column == "is_builtin_default") {
        connection.execute(
            "ALTER TABLE profiles ADD COLUMN is_builtin_default INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }

    connection.execute(
        "UPDATE profiles
         SET is_builtin_default = 1
         WHERE id = 'default' OR lower(name) = 'default'",
        [],
    )?;

    connection.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_profiles_name_nocase
         ON profiles (name COLLATE NOCASE)",
        [],
    )?;

    Ok(())
}
