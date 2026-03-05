use rusqlite::{params, Connection, OptionalExtension};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::error::InternalError;

const MIGRATION_0001: &str = include_str!("../../migrations/0001_init.sql");
const MIGRATION_0002: &str = include_str!("../../migrations/0002_catalog_indexes.sql");
const MIGRATION_0003: &str = include_str!("../../migrations/0003_profiles.sql");
const MIGRATION_0004: &str = include_str!("../../migrations/0004_cache_downloads.sql");

pub fn migrate(connection: &Connection) -> Result<(), InternalError> {
    connection.execute_batch(MIGRATION_0001)?;
    connection.execute_batch(MIGRATION_0002)?;
    connection.execute_batch(MIGRATION_0003)?;
    connection.execute_batch(MIGRATION_0004)?;
    repair_profiles_schema(connection)?;
    repair_cache_schema(connection)?;
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
        "INSERT OR IGNORE INTO settings (key, value_json, updated_at) VALUES (?1, ?2, ?3)",
        params![
            "warning.install_without_dependencies",
            serde_json::to_string(&true)?,
            now_rfc3339()?
        ],
    )?;
    connection.execute(
        "INSERT OR IGNORE INTO settings (key, value_json, updated_at) VALUES (?1, ?2, ?3)",
        params![
            "warning.uninstall_with_dependants",
            serde_json::to_string(&true)?,
            now_rfc3339()?
        ],
    )?;
    connection.execute(
        "INSERT OR IGNORE INTO settings (key, value_json, updated_at) VALUES (?1, ?2, ?3)",
        params![
            "launch.default_mode",
            serde_json::to_string(&"steam")?,
            now_rfc3339()?
        ],
    )?;
    connection.execute(
        "INSERT OR IGNORE INTO settings (key, value_json, updated_at) VALUES (?1, ?2, ?3)",
        params![
            "launch.preferred_game_path",
            serde_json::to_string(&"")?,
            now_rfc3339()?
        ],
    )?;
    connection.execute(
        "INSERT OR IGNORE INTO settings (key, value_json, updated_at) VALUES (?1, ?2, ?3)",
        params![
            "launch.preferred_proton_runtime_id",
            serde_json::to_string(&Option::<String>::None)?,
            now_rfc3339()?
        ],
    )?;
    connection.execute(
        "INSERT OR IGNORE INTO settings (key, value_json, updated_at) VALUES (?1, ?2, ?3)",
        params![
            "launch.v49_signature_hashes",
            serde_json::to_string(&Vec::<String>::new())?,
            now_rfc3339()?
        ],
    )?;
    connection.execute(
        "INSERT OR IGNORE INTO settings (key, value_json, updated_at) VALUES (?1, ?2, ?3)",
        params![
            "launch.last_mode",
            serde_json::to_string(&Option::<String>::None)?,
            now_rfc3339()?
        ],
    )?;
    connection.execute(
        "INSERT OR IGNORE INTO settings (key, value_json, updated_at) VALUES (?1, ?2, ?3)",
        params![
            "launch.last_profile_id",
            serde_json::to_string(&Option::<String>::None)?,
            now_rfc3339()?
        ],
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
        -- Legacy tables from older profile schema; remove if present.
        DROP TABLE IF EXISTS profile_mod_dependencies;
        DROP TABLE IF EXISTS profile_mods;
        DROP TABLE IF EXISTS local_mods;

        DELETE FROM download_jobs;
        DELETE FROM install_tasks;
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
    Ok(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM profiles WHERE id = ?1)",
        params![profile_id],
        |row| row.get::<_, i64>(0),
    )? != 0)
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

fn repair_cache_schema(connection: &Connection) -> Result<(), InternalError> {
    connection.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_cached_archives_version_id
         ON cached_archives(version_id)
         WHERE version_id IS NOT NULL",
        [],
    )?;
    connection.execute(
        "CREATE INDEX IF NOT EXISTS idx_cached_archives_sha256
         ON cached_archives(sha256)",
        [],
    )?;
    connection.execute(
        "CREATE INDEX IF NOT EXISTS idx_cached_archives_last_used_at
         ON cached_archives(last_used_at)",
        [],
    )?;
    connection.execute(
        "CREATE INDEX IF NOT EXISTS idx_install_tasks_kind_status
         ON install_tasks(kind, status)",
        [],
    )?;
    connection.execute(
        "CREATE INDEX IF NOT EXISTS idx_download_jobs_task_id
         ON download_jobs(task_id)",
        [],
    )?;

    Ok(())
}
