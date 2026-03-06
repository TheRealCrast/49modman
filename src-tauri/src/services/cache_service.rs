use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::{
    app_state::AppState, db::now_rfc3339, error::InternalError,
    services::profile_service::read_profile_manifest_mods,
};

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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CachePruneCandidateDto {
    pub package_id: String,
    pub package_name: String,
    pub version_id: String,
    pub version_number: String,
    pub archive_name: String,
    pub file_size: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CachePrunePreviewDto {
    pub removable_count: usize,
    pub removable_bytes: i64,
    pub candidates: Vec<CachePruneCandidateDto>,
}

#[derive(Debug, Clone)]
struct RemovableArchiveRow {
    cache_key: String,
    relative_path: String,
    version_id: Option<String>,
    candidate: CachePruneCandidateDto,
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
    upsert_cached_archive_entry(
        connection,
        version_id,
        "thunderstore",
        Some(package_id),
        Some(version_id),
        sha256,
        archive_name,
        relative_path,
        file_size,
        Some(source_url),
    )
}

pub fn upsert_local_cached_archive(
    connection: &Connection,
    cache_key: &str,
    sha256: &str,
    archive_name: &str,
    relative_path: &str,
    file_size: i64,
) -> Result<(), InternalError> {
    upsert_cached_archive_entry(
        connection,
        cache_key,
        "local_zip",
        None,
        None,
        sha256,
        archive_name,
        relative_path,
        file_size,
        None,
    )
}

fn upsert_cached_archive_entry(
    connection: &Connection,
    cache_key: &str,
    source_kind: &str,
    package_id: Option<&str>,
    version_id: Option<&str>,
    sha256: &str,
    archive_name: &str,
    relative_path: &str,
    file_size: i64,
    source_url: Option<&str>,
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
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)
         ON CONFLICT(cache_key) DO UPDATE SET
            source_kind = excluded.source_kind,
            package_id = excluded.package_id,
            version_id = excluded.version_id,
            sha256 = excluded.sha256,
            archive_name = excluded.archive_name,
            relative_path = excluded.relative_path,
            file_size = excluded.file_size,
            source_url = excluded.source_url,
            last_used_at = excluded.last_used_at",
        params![
            cache_key,
            source_kind,
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

    let has_active_downloads = has_active_cache_downloads(connection)?;

    Ok(CacheSummaryDto {
        archive_count: archive_count.max(0) as usize,
        total_bytes,
        cache_path: state.cache_dir.display().to_string(),
        has_active_downloads,
    })
}

pub fn open_cache_folder(state: &AppState) -> Result<(), InternalError> {
    fs::create_dir_all(&state.cache_dir)?;

    let mut command = if cfg!(target_os = "windows") {
        let mut command = Command::new("explorer");
        command.arg(&state.cache_dir);
        command
    } else if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(&state.cache_dir);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(&state.cache_dir);
        command
    };

    // Launch the system opener without waiting for the file explorer to close.
    // Reap in a detached thread to avoid leaving a zombie process.
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| {
            InternalError::app(
                "OPEN_CACHE_FOLDER_FAILED",
                "Failed to open the cache folder in the system file explorer.",
            )
        })?;

    std::thread::spawn(move || {
        let _ = child.wait();
    });

    Ok(())
}

pub fn clear_cache(
    state: &AppState,
    connection: &Connection,
) -> Result<CacheSummaryDto, InternalError> {
    let has_active_downloads = has_active_cache_downloads(connection)?;

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

pub fn preview_clear_cache_unreferenced(
    state: &AppState,
    connection: &Connection,
) -> Result<CachePrunePreviewDto, InternalError> {
    if has_active_cache_downloads(connection)? {
        return Err(InternalError::app(
            "CACHE_CLEAR_BLOCKED",
            "Cannot clear the cache while downloads are active.",
        ));
    }

    let removable = list_removable_archives(state, connection)?;
    let removable_bytes = removable.iter().fold(0_i64, |sum, row| {
        sum.saturating_add(row.candidate.file_size)
    });

    Ok(CachePrunePreviewDto {
        removable_count: removable.len(),
        removable_bytes,
        candidates: removable.into_iter().map(|row| row.candidate).collect(),
    })
}

pub fn clear_cache_unreferenced(
    state: &AppState,
    connection: &Connection,
) -> Result<CacheSummaryDto, InternalError> {
    if has_active_cache_downloads(connection)? {
        return Err(InternalError::app(
            "CACHE_CLEAR_BLOCKED",
            "Cannot clear the cache while downloads are active.",
        ));
    }

    let removable = list_removable_archives(state, connection)?;
    let mut removed_version_ids = HashSet::new();

    for row in removable {
        let archive_path = cached_archive_path(state, &row.relative_path);
        if archive_path.is_file() {
            fs::remove_file(&archive_path)?;
        } else if archive_path.is_dir() {
            fs::remove_dir_all(&archive_path)?;
        }

        connection.execute(
            "DELETE FROM cached_archives WHERE cache_key = ?1",
            params![row.cache_key],
        )?;

        if let Some(version_id) = row.version_id {
            removed_version_ids.insert(version_id);
        }
    }

    for version_id in removed_version_ids {
        connection.execute(
            "DELETE FROM download_jobs
             WHERE task_id IN (
               SELECT id
               FROM install_tasks
               WHERE kind = 'cache_version'
                 AND detail = ?1
             )",
            params![version_id],
        )?;
        connection.execute(
            "DELETE FROM install_tasks
             WHERE kind = 'cache_version'
               AND detail = ?1",
            params![version_id],
        )?;
    }

    get_cache_summary(state, connection)
}

pub fn clear_cache_files(state: &AppState) -> Result<(), InternalError> {
    clear_directory_contents(&state.cache_archives_dir)?;
    clear_directory_contents(&state.cache_tmp_dir)?;
    Ok(())
}

fn has_active_cache_downloads(connection: &Connection) -> Result<bool, InternalError> {
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

    Ok(has_active_downloads)
}

fn collect_installed_version_ids(
    state: &AppState,
    connection: &Connection,
) -> Result<HashSet<String>, InternalError> {
    let mut statement = connection.prepare("SELECT id FROM profiles")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;

    let mut installed_version_ids = HashSet::new();
    for row in rows {
        let profile_id = row?;
        let installed_mods = read_profile_manifest_mods(state, &profile_id)?;
        for installed_mod in installed_mods {
            installed_version_ids.insert(installed_mod.version_id);
        }
    }

    Ok(installed_version_ids)
}

fn list_removable_archives(
    state: &AppState,
    connection: &Connection,
) -> Result<Vec<RemovableArchiveRow>, InternalError> {
    let installed_version_ids = collect_installed_version_ids(state, connection)?;
    let mut statement = connection.prepare(
        "SELECT
            ca.cache_key,
            ca.relative_path,
            ca.package_id,
            ca.version_id,
            ca.archive_name,
            ca.file_size,
            p.full_name,
            pv.version_number
         FROM cached_archives ca
         LEFT JOIN packages p ON p.id = ca.package_id
         LEFT JOIN package_versions pv ON pv.id = ca.version_id
         ORDER BY COALESCE(p.full_name, ca.package_id, ca.archive_name) COLLATE NOCASE ASC,
                  COALESCE(pv.version_number, ca.archive_name) COLLATE NOCASE ASC",
    )?;

    let rows = statement.query_map([], |row| {
        let package_id = row.get::<_, Option<String>>(2)?;
        let version_id = row.get::<_, Option<String>>(3)?;
        let archive_name = row.get::<_, String>(4)?;
        let package_name = row.get::<_, Option<String>>(6)?;
        let version_number = row.get::<_, Option<String>>(7)?;

        let fallback_package_id = package_id
            .clone()
            .unwrap_or_else(|| "unknown-package".to_string());
        let fallback_version_id = version_id.clone().unwrap_or_else(|| archive_name.clone());
        let fallback_package_name = package_name
            .or(package_id)
            .unwrap_or_else(|| "Unknown package".to_string());
        let fallback_version_number = version_number
            .or(version_id.clone())
            .unwrap_or_else(|| archive_name.clone());

        Ok(RemovableArchiveRow {
            cache_key: row.get(0)?,
            relative_path: row.get(1)?,
            version_id,
            candidate: CachePruneCandidateDto {
                package_id: fallback_package_id,
                package_name: fallback_package_name,
                version_id: fallback_version_id,
                version_number: fallback_version_number,
                archive_name,
                file_size: row.get(5)?,
            },
        })
    })?;

    let mut removable = Vec::new();
    for row in rows {
        let row = row?;
        let is_installed = row
            .version_id
            .as_ref()
            .map(|version_id| installed_version_ids.contains(version_id))
            .unwrap_or(false);
        if !is_installed {
            removable.push(row);
        }
    }

    Ok(removable)
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
