use std::{
    fs::{self, File},
    io::{Read, Write},
    sync::MutexGuard,
    time::{Duration, Instant},
};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    db::now_rfc3339,
    error::InternalError,
    services::cache_service::{
        cached_archive_relative_path, mark_archive_used, thunderstore_archive_path,
        upsert_cached_archive, verify_cached_archive,
    },
};

const FINISHED_GRACE_MILLIS: i128 = 2000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallTaskDto {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub title: String,
    pub detail: String,
    pub progress_step: Option<String>,
    pub progress_current: i64,
    pub progress_total: i64,
    pub error_message: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadJobDto {
    pub id: String,
    pub task_id: String,
    pub package_name: String,
    pub version_label: String,
    pub source_kind: String,
    pub status: String,
    pub cache_hit: bool,
    pub bytes_downloaded: i64,
    pub total_bytes: Option<i64>,
    pub speed_bps: Option<i64>,
    pub progress_label: String,
    pub error_message: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueInstallToCacheInput {
    pub package_id: String,
    pub version_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueInstallToCacheResult {
    pub task_id: String,
}

#[derive(Debug, Clone)]
struct VersionCacheInfo {
    package_id: String,
    version_id: String,
    package_name: String,
    version_number: String,
    download_url: String,
}

pub fn queue_install_to_cache(
    state: &AppState,
    input: QueueInstallToCacheInput,
) -> Result<QueueInstallToCacheResult, InternalError> {
    let info = {
        let connection = state.connection.lock().map_err(|_| {
            InternalError::app("DB_INIT_FAILED", "Failed to lock the SQLite connection.")
        })?;
        load_version_cache_info(&connection, &input.package_id, &input.version_id)?
    };

    {
        let connection = state.connection.lock().map_err(|_| {
            InternalError::app("DB_INIT_FAILED", "Failed to lock the SQLite connection.")
        })?;

        if let Some(existing_task_id) = find_active_task_for_version(&connection, &info.version_id)?
        {
            return Ok(QueueInstallToCacheResult {
                task_id: existing_task_id,
            });
        }
    }

    let task_id = format!("task-{}", Uuid::new_v4());
    let job_id = format!("job-{}", Uuid::new_v4());

    {
        let connection = state.connection.lock().map_err(|_| {
            InternalError::app("DB_INIT_FAILED", "Failed to lock the SQLite connection.")
        })?;
        insert_task_and_job(&connection, &task_id, &job_id, &info)?;
    }

    let worker_state = state.clone();
    let worker_task_id = task_id.clone();
    let worker_job_id = job_id.clone();
    let worker_info = info.clone();

    tauri::async_runtime::spawn(async move {
        let _ = tauri::async_runtime::spawn_blocking(move || {
            process_cache_task(&worker_state, &worker_task_id, &worker_job_id, &worker_info)
        })
        .await;
    });

    Ok(QueueInstallToCacheResult { task_id })
}

pub fn list_active_downloads(
    connection: &Connection,
) -> Result<Vec<DownloadJobDto>, InternalError> {
    let mut statement = connection.prepare(
        "SELECT
            j.id,
            j.task_id,
            j.package_name,
            j.version_label,
            j.source_kind,
            j.status,
            j.cache_hit,
            j.bytes_downloaded,
            j.total_bytes,
            j.speed_bps,
            j.progress_label,
            j.error_message,
            j.updated_at,
            t.status,
            t.finished_at,
            t.created_at
         FROM download_jobs j
         INNER JOIN install_tasks t ON t.id = j.task_id
         WHERE t.kind = 'cache_version'
         ORDER BY t.created_at DESC, j.updated_at DESC",
    )?;

    let now = OffsetDateTime::now_utc();
    let rows = statement.query_map([], |row| {
        Ok((
            DownloadJobDto {
                id: row.get(0)?,
                task_id: row.get(1)?,
                package_name: row.get(2)?,
                version_label: row.get(3)?,
                source_kind: row.get(4)?,
                status: row.get(5)?,
                cache_hit: row.get::<_, i64>(6)? != 0,
                bytes_downloaded: row.get(7)?,
                total_bytes: row.get(8)?,
                speed_bps: row.get(9)?,
                progress_label: row.get(10)?,
                error_message: row.get(11)?,
                updated_at: row.get(12)?,
            },
            row.get::<_, String>(13)?,
            row.get::<_, Option<String>>(14)?,
            row.get::<_, String>(15)?,
        ))
    })?;

    let mut items = Vec::new();

    for row in rows {
        let (job, task_status, finished_at, _created_at) = row?;
        if should_show_job(&task_status, finished_at.as_deref(), now)? {
            items.push(job);
        }
    }

    Ok(items)
}

pub fn get_task(
    connection: &Connection,
    task_id: &str,
) -> Result<Option<InstallTaskDto>, InternalError> {
    connection
        .query_row(
            "SELECT id, kind, status, title, detail, progress_step, progress_current, progress_total, error_message, created_at, started_at, finished_at
             FROM install_tasks
             WHERE id = ?1",
            params![task_id],
            |row| {
                Ok(InstallTaskDto {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    status: row.get(2)?,
                    title: row.get(3)?,
                    detail: row.get(4)?,
                    progress_step: row.get(5)?,
                    progress_current: row.get(6)?,
                    progress_total: row.get(7)?,
                    error_message: row.get(8)?,
                    created_at: row.get(9)?,
                    started_at: row.get(10)?,
                    finished_at: row.get(11)?,
                })
            },
        )
        .optional()
        .map_err(InternalError::from)
}

fn load_version_cache_info(
    connection: &Connection,
    package_id: &str,
    version_id: &str,
) -> Result<VersionCacheInfo, InternalError> {
    connection
        .query_row(
            "SELECT p.id, v.id, p.full_name, v.version_number, v.download_url
             FROM packages p
             INNER JOIN package_versions v ON v.package_id = p.id
             WHERE p.id = ?1
               AND v.id = ?2",
            params![package_id, version_id],
            |row| {
                Ok(VersionCacheInfo {
                    package_id: row.get(0)?,
                    version_id: row.get(1)?,
                    package_name: row.get(2)?,
                    version_number: row.get(3)?,
                    download_url: row.get(4)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| {
            InternalError::app(
                "PACKAGE_NOT_FOUND",
                "That package version is not available in the cached Thunderstore catalog.",
            )
        })
        .and_then(|info| {
            if info.download_url.trim().is_empty() {
                Err(InternalError::app(
                    "PACKAGE_NOT_FOUND",
                    "Version cannot be downloaded from local metadata. Refresh the catalog and try again.",
                ))
            } else {
                Ok(info)
            }
        })
}

fn find_active_task_for_version(
    connection: &Connection,
    version_id: &str,
) -> Result<Option<String>, InternalError> {
    connection
        .query_row(
            "SELECT id
             FROM install_tasks
             WHERE kind = 'cache_version'
               AND detail = ?1
               AND status IN ('queued', 'running')
             ORDER BY created_at DESC
             LIMIT 1",
            params![version_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(InternalError::from)
}

fn insert_task_and_job(
    connection: &Connection,
    task_id: &str,
    job_id: &str,
    info: &VersionCacheInfo,
) -> Result<(), InternalError> {
    let now = now_rfc3339()?;
    let title = format!("Caching {} {}", info.package_name, info.version_number);

    connection.execute(
        "INSERT INTO install_tasks (
            id, profile_id, kind, status, title, detail, progress_step, progress_current,
            progress_total, error_message, created_at, started_at, finished_at
         ) VALUES (?1, NULL, 'cache_version', 'queued', ?2, ?3, 'queued', 0, 4, NULL, ?4, NULL, NULL)",
        params![task_id, title, info.version_id, now],
    )?;

    connection.execute(
        "INSERT INTO download_jobs (
            id, task_id, package_name, version_label, source_kind, status, cache_hit,
            bytes_downloaded, total_bytes, speed_bps, progress_label, error_message, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, 'thunderstore', 'queued', 0, 0, NULL, NULL, 'Queued for cache check', NULL, ?5, ?5)",
        params![job_id, task_id, info.package_name, info.version_number, now],
    )?;

    Ok(())
}

fn process_cache_task(
    state: &AppState,
    task_id: &str,
    job_id: &str,
    info: &VersionCacheInfo,
) -> Result<(), InternalError> {
    update_task_state(
        state,
        task_id,
        TaskUpdate {
            status: Some("running"),
            progress_step: Some("checking_cache"),
            progress_current: Some(1),
            progress_total: Some(4),
            started_at_now: true,
            ..TaskUpdate::default()
        },
    )?;
    update_job_state(
        state,
        job_id,
        JobUpdate {
            status: Some("checking_cache"),
            progress_label: Some("Checking the shared cache"),
            ..JobUpdate::default()
        },
    )?;

    {
        let connection = state.connection.lock().map_err(|_| {
            InternalError::app("DB_INIT_FAILED", "Failed to lock the SQLite connection.")
        })?;
        if verify_cached_archive(state, &connection, &info.version_id)?.is_some() {
            mark_archive_used(&connection, &info.version_id)?;
            drop(connection);

            update_job_state(
                state,
                job_id,
                JobUpdate {
                    status: Some("cached"),
                    cache_hit: Some(true),
                    progress_label: Some("Already cached locally"),
                    bytes_downloaded: Some(0),
                    total_bytes: Some(0),
                    ..JobUpdate::default()
                },
            )?;
            mark_task_finished(state, task_id, true, None, "finalizing", 4, 4)?;
            return Ok(());
        }
    }

    update_task_state(
        state,
        task_id,
        TaskUpdate {
            status: Some("running"),
            progress_step: Some("downloading"),
            progress_current: Some(2),
            progress_total: Some(4),
            ..TaskUpdate::default()
        },
    )?;
    update_job_state(
        state,
        job_id,
        JobUpdate {
            status: Some("downloading"),
            progress_label: Some("Downloading from Thunderstore"),
            ..JobUpdate::default()
        },
    )?;

    if let Err(error) = download_and_cache_archive(state, task_id, job_id, info) {
        let error_message = error.to_string();
        let _ = update_job_state(
            state,
            job_id,
            JobUpdate {
                status: Some("failed"),
                progress_label: Some("Download failed"),
                error_message: Some(error_message.clone()),
                ..JobUpdate::default()
            },
        );
        let _ = mark_task_finished(
            state,
            task_id,
            false,
            Some(error_message),
            "downloading",
            2,
            4,
        );
        return Err(error);
    }

    update_task_state(
        state,
        task_id,
        TaskUpdate {
            status: Some("running"),
            progress_step: Some("verifying"),
            progress_current: Some(3),
            progress_total: Some(4),
            ..TaskUpdate::default()
        },
    )?;
    update_job_state(
        state,
        job_id,
        JobUpdate {
            status: Some("verifying"),
            progress_label: Some("Verifying cached archive"),
            ..JobUpdate::default()
        },
    )?;

    mark_task_finished(state, task_id, true, None, "finalizing", 4, 4)?;
    Ok(())
}

fn download_and_cache_archive(
    state: &AppState,
    task_id: &str,
    job_id: &str,
    info: &VersionCacheInfo,
) -> Result<(), InternalError> {
    let tmp_path = state.cache_tmp_dir.join(format!("{task_id}.part"));
    if let Some(parent) = tmp_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let final_path = thunderstore_archive_path(state, &info.version_id);
    if let Some(parent) = final_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut response = state
        .http_client
        .get(&info.download_url)
        .send()?
        .error_for_status()?;
    let total_bytes = response.content_length().map(|value| value as i64);
    let mut file = File::create(&tmp_path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut downloaded: i64 = 0;
    let mut last_progress = Instant::now();
    let started = Instant::now();

    loop {
        let read = response.read(&mut buffer)?;
        if read == 0 {
            break;
        }

        file.write_all(&buffer[..read])?;
        hasher.update(&buffer[..read]);
        downloaded += read as i64;

        if last_progress.elapsed() >= Duration::from_millis(200) {
            let elapsed_secs = started.elapsed().as_secs_f64().max(0.001);
            let speed_bps = (downloaded as f64 / elapsed_secs) as i64;
            update_job_state(
                state,
                job_id,
                JobUpdate {
                    status: Some("downloading"),
                    bytes_downloaded: Some(downloaded),
                    total_bytes,
                    speed_bps: Some(speed_bps),
                    progress_label: Some("Downloading from Thunderstore"),
                    ..JobUpdate::default()
                },
            )?;
            last_progress = Instant::now();
        }
    }

    file.flush()?;
    drop(file);

    let sha256 = format!("{:x}", hasher.finalize());
    let final_size = fs::metadata(&tmp_path)?.len() as i64;

    if final_path.exists() {
        fs::remove_file(&final_path)?;
    }
    fs::rename(&tmp_path, &final_path)?;

    {
        let connection = state.connection.lock().map_err(|_| {
            InternalError::app("DB_INIT_FAILED", "Failed to lock the SQLite connection.")
        })?;
        upsert_cached_archive(
            &connection,
            &info.package_id,
            &info.version_id,
            &sha256,
            &format!("{}.zip", info.version_id),
            &cached_archive_relative_path(&info.version_id),
            final_size,
            &info.download_url,
        )?;
    }

    update_job_state(
        state,
        job_id,
        JobUpdate {
            status: Some("verifying"),
            bytes_downloaded: Some(downloaded),
            total_bytes: Some(final_size),
            speed_bps: Some(
                (downloaded as f64 / started.elapsed().as_secs_f64().max(0.001)) as i64,
            ),
            progress_label: Some("Archive cached successfully"),
            ..JobUpdate::default()
        },
    )?;

    Ok(())
}

fn should_show_job(
    task_status: &str,
    finished_at: Option<&str>,
    now: OffsetDateTime,
) -> Result<bool, InternalError> {
    match task_status {
        "queued" | "running" | "failed" => Ok(true),
        "succeeded" => {
            let Some(finished_at) = finished_at else {
                return Ok(false);
            };
            let finished_at = OffsetDateTime::parse(finished_at, &Rfc3339)?;
            Ok((now - finished_at).whole_milliseconds() <= FINISHED_GRACE_MILLIS)
        }
        _ => Ok(false),
    }
}

#[derive(Default)]
struct TaskUpdate<'a> {
    status: Option<&'a str>,
    progress_step: Option<&'a str>,
    progress_current: Option<i64>,
    progress_total: Option<i64>,
    error_message: Option<String>,
    started_at_now: bool,
    finished_at_now: bool,
}

#[derive(Default)]
struct JobUpdate<'a> {
    status: Option<&'a str>,
    cache_hit: Option<bool>,
    bytes_downloaded: Option<i64>,
    total_bytes: Option<i64>,
    speed_bps: Option<i64>,
    progress_label: Option<&'a str>,
    error_message: Option<String>,
}

fn mark_task_finished(
    state: &AppState,
    task_id: &str,
    succeeded: bool,
    error_message: Option<String>,
    progress_step: &str,
    progress_current: i64,
    progress_total: i64,
) -> Result<(), InternalError> {
    update_task_state(
        state,
        task_id,
        TaskUpdate {
            status: Some(if succeeded { "succeeded" } else { "failed" }),
            progress_step: Some(progress_step),
            progress_current: Some(progress_current),
            progress_total: Some(progress_total),
            error_message,
            finished_at_now: true,
            ..TaskUpdate::default()
        },
    )
}

fn update_task_state(
    state: &AppState,
    task_id: &str,
    update: TaskUpdate<'_>,
) -> Result<(), InternalError> {
    let connection = state.connection.lock().map_err(|_| {
        InternalError::app("DB_INIT_FAILED", "Failed to lock the SQLite connection.")
    })?;
    apply_task_update(&connection, task_id, update)
}

fn apply_task_update(
    connection: &MutexGuard<'_, Connection>,
    task_id: &str,
    update: TaskUpdate<'_>,
) -> Result<(), InternalError> {
    let mut task = get_task(connection, task_id)?.ok_or_else(|| {
        InternalError::app(
            "DOWNLOAD_TASK_NOT_FOUND",
            "The active cache task could not be found.",
        )
    })?;

    if let Some(status) = update.status {
        task.status = status.to_string();
    }
    if let Some(progress_step) = update.progress_step {
        task.progress_step = Some(progress_step.to_string());
    }
    if let Some(progress_current) = update.progress_current {
        task.progress_current = progress_current;
    }
    if let Some(progress_total) = update.progress_total {
        task.progress_total = progress_total;
    }
    if let Some(error_message) = update.error_message {
        task.error_message = Some(error_message);
    }
    if update.started_at_now && task.started_at.is_none() {
        task.started_at = Some(now_rfc3339()?);
    }
    if update.finished_at_now {
        task.finished_at = Some(now_rfc3339()?);
    }

    connection.execute(
        "UPDATE install_tasks
         SET status = ?2,
             progress_step = ?3,
             progress_current = ?4,
             progress_total = ?5,
             error_message = ?6,
             started_at = ?7,
             finished_at = ?8
         WHERE id = ?1",
        params![
            task.id,
            task.status,
            task.progress_step,
            task.progress_current,
            task.progress_total,
            task.error_message,
            task.started_at,
            task.finished_at
        ],
    )?;

    Ok(())
}

fn update_job_state(
    state: &AppState,
    job_id: &str,
    update: JobUpdate<'_>,
) -> Result<(), InternalError> {
    let connection = state.connection.lock().map_err(|_| {
        InternalError::app("DB_INIT_FAILED", "Failed to lock the SQLite connection.")
    })?;

    let mut job = connection
        .query_row(
            "SELECT id, task_id, package_name, version_label, source_kind, status, cache_hit, bytes_downloaded, total_bytes, speed_bps, progress_label, error_message, updated_at
             FROM download_jobs
             WHERE id = ?1",
            params![job_id],
            |row| {
                Ok(DownloadJobDto {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    package_name: row.get(2)?,
                    version_label: row.get(3)?,
                    source_kind: row.get(4)?,
                    status: row.get(5)?,
                    cache_hit: row.get::<_, i64>(6)? != 0,
                    bytes_downloaded: row.get(7)?,
                    total_bytes: row.get(8)?,
                    speed_bps: row.get(9)?,
                    progress_label: row.get(10)?,
                    error_message: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| {
            InternalError::app("DOWNLOAD_TASK_NOT_FOUND", "The active download job could not be found.")
        })?;

    if let Some(status) = update.status {
        job.status = status.to_string();
    }
    if let Some(cache_hit) = update.cache_hit {
        job.cache_hit = cache_hit;
    }
    if let Some(bytes_downloaded) = update.bytes_downloaded {
        job.bytes_downloaded = bytes_downloaded;
    }
    if let Some(total_bytes) = update.total_bytes {
        job.total_bytes = Some(total_bytes);
    }
    if let Some(speed_bps) = update.speed_bps {
        job.speed_bps = Some(speed_bps);
    }
    if let Some(progress_label) = update.progress_label {
        job.progress_label = progress_label.to_string();
    }
    if let Some(error_message) = update.error_message {
        job.error_message = Some(error_message);
    }
    job.updated_at = now_rfc3339()?;

    connection.execute(
        "UPDATE download_jobs
         SET status = ?2,
             cache_hit = ?3,
             bytes_downloaded = ?4,
             total_bytes = ?5,
             speed_bps = ?6,
             progress_label = ?7,
             error_message = ?8,
             updated_at = ?9
         WHERE id = ?1",
        params![
            job.id,
            job.status,
            if job.cache_hit { 1 } else { 0 },
            job.bytes_downloaded,
            job.total_bytes,
            job.speed_bps,
            job.progress_label,
            job.error_message,
            job.updated_at
        ],
    )?;

    Ok(())
}
