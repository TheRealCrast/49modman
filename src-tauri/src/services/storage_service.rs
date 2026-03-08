use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use rfd::FileDialog;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::{
    app_state::{AppState, StorageMigrationState},
    db::{now_rfc3339, upsert_setting},
    error::InternalError,
};

const STORAGE_CACHE_DIR_KEY: &str = "storage.cache_dir";
const STORAGE_PROFILES_DIR_KEY: &str = "storage.profiles_dir";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageLocationsDto {
    pub cache_dir: String,
    pub profiles_dir: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartStorageMigrationInput {
    #[serde(default)]
    pub cache_dir: Option<String>,
    #[serde(default)]
    pub profiles_dir: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PickStorageFolderInput {
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageMigrationStatusDto {
    pub phase: String,
    pub message: String,
    pub bytes_copied: i64,
    pub total_bytes: i64,
    pub percent_complete: f64,
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
struct StorageTargetOperation {
    label: &'static str,
    setting_key: &'static str,
    source: PathBuf,
    destination: PathBuf,
}

pub fn get_storage_locations(state: &AppState) -> StorageLocationsDto {
    StorageLocationsDto {
        cache_dir: state.cache_dir.display().to_string(),
        profiles_dir: state.profiles_dir.display().to_string(),
    }
}

pub fn get_storage_migration_status(
    state: &AppState,
) -> Result<StorageMigrationStatusDto, InternalError> {
    let snapshot = state.storage_migration_state.lock().map_err(|_| {
        InternalError::app(
            "STORAGE_MIGRATION_FAILED",
            "Failed to lock storage migration state.",
        )
    })?;
    Ok(map_storage_migration_status(&snapshot))
}

pub fn pick_storage_folder(
    state: &AppState,
    input: PickStorageFolderInput,
) -> Result<Option<String>, InternalError> {
    let (title, initial_path) = match input.kind.as_str() {
        "cache" => ("Choose cache storage folder", state.cache_dir.clone()),
        "profiles" => ("Choose profiles storage folder", state.profiles_dir.clone()),
        _ => {
            return Err(InternalError::app(
                "STORAGE_PATH_INVALID",
                format!("Unsupported storage picker kind: {}", input.kind),
            ))
        }
    };

    let selected = FileDialog::new()
        .set_title(title)
        .set_directory(initial_path)
        .pick_folder();

    Ok(selected.map(|path| path.display().to_string()))
}

pub fn start_storage_migration(
    state: &AppState,
    input: StartStorageMigrationInput,
) -> Result<StorageMigrationStatusDto, InternalError> {
    {
        let migration_state = state.storage_migration_state.lock().map_err(|_| {
            InternalError::app(
                "STORAGE_MIGRATION_FAILED",
                "Failed to lock storage migration state.",
            )
        })?;
        if migration_state.is_active {
            return Err(InternalError::app(
                "STORAGE_MIGRATION_IN_PROGRESS",
                "A storage migration is already in progress.",
            ));
        }
    }

    let mut requested_operations = Vec::new();
    if let Some(destination) = normalize_requested_path(input.cache_dir)? {
        requested_operations.push(StorageTargetOperation {
            label: "cache",
            setting_key: STORAGE_CACHE_DIR_KEY,
            source: state.cache_dir.clone(),
            destination,
        });
    }
    if let Some(destination) = normalize_requested_path(input.profiles_dir)? {
        requested_operations.push(StorageTargetOperation {
            label: "profiles",
            setting_key: STORAGE_PROFILES_DIR_KEY,
            source: state.profiles_dir.clone(),
            destination,
        });
    }

    if requested_operations.is_empty() {
        return Err(InternalError::app(
            "STORAGE_PATH_INVALID",
            "Choose at least one destination path to migrate.",
        ));
    }

    let mut operations = Vec::new();
    for operation in requested_operations {
        if are_paths_equivalent(&operation.source, &operation.destination)? {
            continue;
        }
        validate_source_destination_relationship(&operation.source, &operation.destination)?;
        ensure_destination_is_empty_or_missing(&operation.destination)?;
        operations.push(operation);
    }

    if operations.is_empty() {
        return Err(InternalError::app(
            "STORAGE_PATH_UNCHANGED",
            "The selected destination matches the current storage location.",
        ));
    }

    if operations.len() > 1 {
        for (index, left) in operations.iter().enumerate() {
            for right in operations.iter().skip(index + 1) {
                validate_source_destination_relationship(&left.destination, &right.destination)?;
            }
        }
    }

    {
        let launch_state = state.launch_runtime_state.lock().map_err(|_| {
            InternalError::app("LAUNCH_FAILED", "Failed to lock launch runtime state.")
        })?;
        if launch_state.launch_in_progress {
            return Err(InternalError::app(
                "STORAGE_MIGRATION_FAILED",
                "Cannot migrate storage while a launch is in progress.",
            ));
        }
    }

    {
        let connection = state.connection.lock().map_err(|_| {
            InternalError::app("DB_INIT_FAILED", "Failed to lock the SQLite connection")
        })?;
        if has_active_cache_downloads(&connection)? {
            return Err(InternalError::app(
                "STORAGE_MIGRATION_FAILED",
                "Cannot migrate storage while cache downloads are active.",
            ));
        }
    }

    let total_bytes = operations.iter().try_fold(0_u64, |sum, operation| {
        Ok::<u64, InternalError>(sum.saturating_add(directory_size_bytes(&operation.source)?))
    })?;

    set_storage_migration_state(state, |status| {
        status.phase = "copying".to_string();
        status.message = "Copying storage data to the selected destination...".to_string();
        status.bytes_copied = 0;
        status.total_bytes = total_bytes;
        status.is_active = true;
        status.error = None;
    })?;

    let worker_state = state.clone();
    std::thread::spawn(move || {
        if let Err(error) = run_storage_migration(&worker_state, &operations, total_bytes) {
            let _ = set_storage_migration_state(&worker_state, |status| {
                status.phase = "failed".to_string();
                status.message = "Storage migration failed.".to_string();
                status.is_active = false;
                status.error = Some(error.to_string());
            });
        }
    });

    get_storage_migration_status(state)
}

fn run_storage_migration(
    state: &AppState,
    operations: &[StorageTargetOperation],
    total_bytes: u64,
) -> Result<(), InternalError> {
    let mut bytes_copied = 0_u64;

    for operation in operations {
        fs::create_dir_all(&operation.destination)?;
        copy_directory_recursive(&operation.source, &operation.destination, &mut |bytes| {
            bytes_copied = bytes_copied.saturating_add(bytes);
            set_storage_migration_state(state, |status| {
                status.phase = "copying".to_string();
                status.message = format!(
                    "Copying {} data ({} / {})",
                    operation.label, bytes_copied, total_bytes
                );
                status.bytes_copied = bytes_copied;
                status.total_bytes = total_bytes;
                status.is_active = true;
                status.error = None;
            })
        })?;
    }

    set_storage_migration_state(state, |status| {
        status.phase = "finalizing".to_string();
        status.message = "Finalizing storage migration...".to_string();
        status.bytes_copied = total_bytes;
        status.total_bytes = total_bytes;
        status.is_active = true;
        status.error = None;
    })?;

    {
        let connection = state.connection.lock().map_err(|_| {
            InternalError::app("DB_INIT_FAILED", "Failed to lock the SQLite connection")
        })?;
        for operation in operations {
            upsert_setting(
                &connection,
                operation.setting_key,
                &serde_json::to_string(&operation.destination.display().to_string())?,
                &now_rfc3339()?,
            )?;
        }
    }

    for operation in operations {
        if operation.source.exists() {
            fs::remove_dir_all(&operation.source)?;
        }
    }

    set_storage_migration_state(state, |status| {
        status.phase = "restarting".to_string();
        status.message = "Storage migration complete. Restarting...".to_string();
        status.bytes_copied = total_bytes;
        status.total_bytes = total_bytes;
        status.is_active = true;
        status.error = None;
    })?;

    state.app_handle.restart();
}

fn set_storage_migration_state(
    state: &AppState,
    mutate: impl FnOnce(&mut StorageMigrationState),
) -> Result<(), InternalError> {
    let mut status = state.storage_migration_state.lock().map_err(|_| {
        InternalError::app(
            "STORAGE_MIGRATION_FAILED",
            "Failed to lock storage migration state.",
        )
    })?;
    mutate(&mut status);
    Ok(())
}

fn map_storage_migration_status(status: &StorageMigrationState) -> StorageMigrationStatusDto {
    let clamped_copied = status.bytes_copied.min(i64::MAX as u64) as i64;
    let clamped_total = status.total_bytes.min(i64::MAX as u64) as i64;
    let percent_complete = if status.total_bytes == 0 {
        if status.phase == "restarting" || status.phase == "finalizing" {
            100.0
        } else {
            0.0
        }
    } else {
        ((status.bytes_copied as f64 / status.total_bytes as f64) * 100.0).clamp(0.0, 100.0)
    };

    StorageMigrationStatusDto {
        phase: status.phase.clone(),
        message: status.message.clone(),
        bytes_copied: clamped_copied,
        total_bytes: clamped_total,
        percent_complete,
        is_active: status.is_active,
        error: status.error.clone(),
    }
}

fn normalize_requested_path(value: Option<String>) -> Result<Option<PathBuf>, InternalError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let path = PathBuf::from(trimmed);
    if !path.is_absolute() {
        return Err(InternalError::app(
            "STORAGE_PATH_INVALID",
            "Storage destination must be an absolute path.",
        ));
    }

    Ok(Some(path))
}

fn normalize_for_compare(path: &Path) -> Result<PathBuf, InternalError> {
    if path.exists() {
        return Ok(fs::canonicalize(path)?);
    }
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    Ok(std::env::current_dir()?.join(path))
}

fn are_paths_equivalent(left: &Path, right: &Path) -> Result<bool, InternalError> {
    Ok(normalize_for_compare(left)? == normalize_for_compare(right)?)
}

fn validate_source_destination_relationship(
    source: &Path,
    destination: &Path,
) -> Result<(), InternalError> {
    let source = normalize_for_compare(source)?;
    let destination = normalize_for_compare(destination)?;

    if source == destination {
        return Err(InternalError::app(
            "STORAGE_PATH_UNCHANGED",
            "Storage destination matches the existing path.",
        ));
    }

    if destination.starts_with(&source) || source.starts_with(&destination) {
        return Err(InternalError::app(
            "STORAGE_PATH_INVALID",
            "Destination cannot be nested inside the current storage path (or vice versa).",
        ));
    }

    Ok(())
}

fn ensure_destination_is_empty_or_missing(destination: &Path) -> Result<(), InternalError> {
    if !destination.exists() {
        return Ok(());
    }
    if !destination.is_dir() {
        return Err(InternalError::app(
            "STORAGE_PATH_INVALID",
            "Storage destination must be a directory.",
        ));
    }

    let mut entries = fs::read_dir(destination)?;
    if entries.next().transpose()?.is_some() {
        return Err(InternalError::app(
            "STORAGE_PATH_NOT_EMPTY",
            "Destination directory must be empty before migrating storage.",
        ));
    }

    Ok(())
}

fn directory_size_bytes(path: &Path) -> Result<u64, InternalError> {
    if !path.exists() {
        return Ok(0);
    }
    if !path.is_dir() {
        return Err(InternalError::app(
            "STORAGE_MIGRATION_FAILED",
            "Storage source path is not a directory.",
        ));
    }

    let mut total = 0_u64;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let entry_path = entry.path();
        if file_type.is_symlink() {
            return Err(InternalError::app(
                "STORAGE_MIGRATION_FAILED",
                format!(
                    "Storage migration does not support symbolic links: {}",
                    entry_path.display()
                ),
            ));
        }
        if file_type.is_dir() {
            total = total.saturating_add(directory_size_bytes(&entry_path)?);
        } else if file_type.is_file() {
            total = total.saturating_add(entry.metadata()?.len());
        }
    }
    Ok(total)
}

fn copy_directory_recursive(
    source: &Path,
    destination: &Path,
    on_progress: &mut impl FnMut(u64) -> Result<(), InternalError>,
) -> Result<(), InternalError> {
    if !source.exists() {
        fs::create_dir_all(destination)?;
        return Ok(());
    }

    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());

        if file_type.is_symlink() {
            return Err(InternalError::app(
                "STORAGE_MIGRATION_FAILED",
                format!(
                    "Storage migration does not support symbolic links: {}",
                    source_path.display()
                ),
            ));
        }

        if file_type.is_dir() {
            copy_directory_recursive(&source_path, &destination_path, on_progress)?;
            continue;
        }

        copy_file_with_progress(&source_path, &destination_path, on_progress)?;
    }

    Ok(())
}

fn copy_file_with_progress(
    source: &Path,
    destination: &Path,
    on_progress: &mut impl FnMut(u64) -> Result<(), InternalError>,
) -> Result<(), InternalError> {
    let mut source_file = fs::File::open(source)?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut destination_file = fs::File::create(destination)?;
    let mut buffer = [0_u8; 1024 * 1024];

    loop {
        let bytes_read = source_file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        destination_file.write_all(&buffer[..bytes_read])?;
        on_progress(bytes_read as u64)?;
    }

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

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use time::OffsetDateTime;

    use super::{
        copy_directory_recursive, directory_size_bytes, ensure_destination_is_empty_or_missing,
        validate_source_destination_relationship,
    };

    #[test]
    fn destination_must_be_empty_or_missing() {
        let root = temp_test_dir("storage-destination-validation");
        let missing = root.join("missing");
        ensure_destination_is_empty_or_missing(&missing).expect("missing destination should pass");

        let empty = root.join("empty");
        fs::create_dir_all(&empty).expect("create empty destination");
        ensure_destination_is_empty_or_missing(&empty).expect("empty destination should pass");

        let non_empty = root.join("non-empty");
        fs::create_dir_all(&non_empty).expect("create non-empty destination");
        fs::write(non_empty.join("file.txt"), b"x").expect("write non-empty marker");
        assert!(ensure_destination_is_empty_or_missing(&non_empty).is_err());
    }

    #[test]
    fn relationship_validation_rejects_nested_paths() {
        let root = temp_test_dir("storage-relationship-validation");
        let source = root.join("source");
        let nested = source.join("nested");
        fs::create_dir_all(&nested).expect("create nested path");
        assert!(validate_source_destination_relationship(&source, &nested).is_err());
    }

    #[test]
    fn recursive_copy_tracks_file_bytes() {
        let root = temp_test_dir("storage-recursive-copy");
        let source = root.join("source");
        let destination = root.join("destination");
        fs::create_dir_all(source.join("a/b")).expect("create source tree");
        fs::write(source.join("a/b/file1.bin"), vec![1_u8; 128]).expect("write source file1");
        fs::write(source.join("file2.bin"), vec![2_u8; 64]).expect("write source file2");

        let expected_bytes = directory_size_bytes(&source).expect("get source bytes");
        let mut copied_bytes = 0_u64;
        copy_directory_recursive(&source, &destination, &mut |bytes| {
            copied_bytes = copied_bytes.saturating_add(bytes);
            Ok(())
        })
        .expect("copy directory recursively");

        assert_eq!(copied_bytes, expected_bytes);
        assert_eq!(
            directory_size_bytes(&destination).unwrap_or_default(),
            expected_bytes
        );
    }

    fn temp_test_dir(prefix: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "49modman-{}-{}",
            prefix,
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));

        if path.exists() {
            let _ = fs::remove_dir_all(&path);
        }
        fs::create_dir_all(&path).expect("create temp test dir");
        path
    }
}
