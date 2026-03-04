use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::{app_state::AppState, db::now_rfc3339, error::InternalError};

#[derive(Debug, Clone)]
pub struct CachedArchive {
    pub cache_key: String,
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheSummaryDto {
    pub archive_count: usize,
    pub total_bytes: i64,
    pub cache_path: String,
    pub has_active_downloads: bool,
}

pub fn get_cached_archive(
    connection: &Connection,
    version_id: &str,
) -> Result<Option<CachedArchive>, InternalError> {
    connection
        .query_row(
            "SELECT cache_key, relative_path
             FROM cached_archives
             WHERE version_id = ?1",
            params![version_id],
            |row| {
                Ok(CachedArchive {
                    cache_key: row.get(0)?,
                    relative_path: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(InternalError::from)
}

pub fn verify_cached_archive(
    state: &AppState,
    connection: &Connection,
    version_id: &str,
) -> Result<Option<CachedArchive>, InternalError> {
    let Some(cached) = get_cached_archive(connection, version_id)? else {
        return Ok(None);
    };

    let archive_path = cached_archive_path(state, &cached.relative_path);

    if archive_path.is_file() {
        return Ok(Some(cached));
    }

    connection.execute(
        "DELETE FROM cached_archives WHERE cache_key = ?1",
        params![cached.cache_key],
    )?;

    Ok(None)
}

pub fn cached_archive_relative_path(version_id: &str) -> String {
    format!("thunderstore/{version_id}.zip")
}

pub fn cached_archive_path(state: &AppState, relative_path: &str) -> PathBuf {
    state.cache_archives_dir.join(relative_path)
}

pub fn thunderstore_archive_path(state: &AppState, version_id: &str) -> PathBuf {
    cached_archive_path(state, &cached_archive_relative_path(version_id))
}

pub fn upsert_cached_archive(
    connection: &Connection,
    package_id: &str,
    version_id: &str,
    sha256: &str,
    archive_name: &str,
    relative_path: &str,
    file_size: i64,
    source_url: &str,
) -> Result<(), InternalError> {
    let now = now_rfc3339()?;

    connection.execute(
        "INSERT INTO cached_archives (
            cache_key,
            source_kind,
            package_id,
            version_id,
            sha256,
            archive_name,
            relative_path,
            file_size,
            source_url,
            first_cached_at,
            last_used_at
         ) VALUES (?1, 'thunderstore', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
         ON CONFLICT(cache_key) DO UPDATE SET
            package_id = excluded.package_id,
            version_id = excluded.version_id,
            sha256 = excluded.sha256,
            archive_name = excluded.archive_name,
            relative_path = excluded.relative_path,
            file_size = excluded.file_size,
            source_url = excluded.source_url,
            last_used_at = excluded.last_used_at",
        params![
            version_id,
            package_id,
            version_id,
            sha256,
            archive_name,
            relative_path,
            file_size,
            source_url,
            now,
        ],
    )?;

    Ok(())
}

pub fn mark_archive_used(connection: &Connection, version_id: &str) -> Result<(), InternalError> {
    connection.execute(
        "UPDATE cached_archives
         SET last_used_at = ?2
         WHERE version_id = ?1",
        params![version_id, now_rfc3339()?],
    )?;
    Ok(())
}

pub fn get_cache_summary(
    state: &AppState,
    connection: &Connection,
) -> Result<CacheSummaryDto, InternalError> {
    let (archive_count, total_bytes) = connection.query_row(
        "SELECT COUNT(*), COALESCE(SUM(file_size), 0)
         FROM cached_archives",
        [],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    )?;

    let has_active_downloads = connection.query_row(
        "SELECT EXISTS(
            SELECT 1
            FROM install_tasks
            WHERE kind = 'cache_version'
              AND status IN ('queued', 'running')
        )",
        [],
        |row| row.get::<_, i64>(0),
    )? != 0;

    Ok(CacheSummaryDto {
        archive_count: archive_count.max(0) as usize,
        total_bytes,
        cache_path: state.cache_dir.display().to_string(),
        has_active_downloads,
    })
}

pub fn open_cache_folder(state: &AppState) -> Result<(), InternalError> {
    fs::create_dir_all(&state.cache_dir)?;

    let status = if cfg!(target_os = "windows") {
        Command::new("explorer").arg(&state.cache_dir).status()?
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(&state.cache_dir).status()?
    } else {
        Command::new("xdg-open").arg(&state.cache_dir).status()?
    };

    if status.success() {
        Ok(())
    } else {
        Err(InternalError::app(
            "OPEN_CACHE_FOLDER_FAILED",
            "Failed to open the cache folder in the system file explorer.",
        ))
    }
}

pub fn clear_cache(
    state: &AppState,
    connection: &Connection,
) -> Result<CacheSummaryDto, InternalError> {
    let has_active_downloads = connection.query_row(
        "SELECT EXISTS(
            SELECT 1
            FROM install_tasks
            WHERE kind = 'cache_version'
              AND status IN ('queued', 'running')
        )",
        [],
        |row| row.get::<_, i64>(0),
    )? != 0;

    if has_active_downloads {
        return Err(InternalError::app(
            "CACHE_CLEAR_BLOCKED",
            "Cannot clear the cache while downloads are active.",
        ));
    }

    clear_directory_contents(&state.cache_archives_dir)?;
    clear_directory_contents(&state.cache_tmp_dir)?;

    connection.execute(
        "DELETE FROM download_jobs
         WHERE task_id IN (
           SELECT id FROM install_tasks WHERE kind = 'cache_version'
         )",
        [],
    )?;
    connection.execute("DELETE FROM install_tasks WHERE kind = 'cache_version'", [])?;
    connection.execute("DELETE FROM cached_archives", [])?;

    get_cache_summary(state, connection)
}

pub fn clear_cache_files(state: &AppState) -> Result<(), InternalError> {
    clear_directory_contents(&state.cache_archives_dir)?;
    clear_directory_contents(&state.cache_tmp_dir)?;
    Ok(())
}

fn clear_directory_contents(path: &Path) -> Result<(), InternalError> {
    fs::create_dir_all(path)?;

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();

        if entry_path.is_dir() {
            fs::remove_dir_all(&entry_path)?;
        } else if entry_path.exists() {
            fs::remove_file(&entry_path)?;
        }
    }

    Ok(())
}
