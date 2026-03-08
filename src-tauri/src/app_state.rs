use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use reqwest::blocking::Client;
use rusqlite::Connection;
use tauri::{AppHandle, Manager};

use crate::{
    db::{self, get_setting},
    error::InternalError,
    resources::bundled_reference::{load_bundled_reference_library, BundledReferenceLibrary},
    services::dependency_service::{
        new_dependency_catalog_index_cache, SharedDependencyCatalogIndexCache,
    },
};

#[derive(Debug, Default)]
pub struct LaunchRuntimeState {
    pub launch_in_progress: bool,
    pub tracked_game_pid: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct StorageMigrationState {
    pub phase: String,
    pub message: String,
    pub bytes_copied: u64,
    pub total_bytes: u64,
    pub is_active: bool,
    pub error: Option<String>,
}

impl Default for StorageMigrationState {
    fn default() -> Self {
        Self {
            phase: "idle".to_string(),
            message: "No storage migration is running.".to_string(),
            bytes_copied: 0,
            total_bytes: 0,
            is_active: false,
            error: None,
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub app_handle: AppHandle,
    pub app_data_dir: PathBuf,
    pub connection: Arc<Mutex<Connection>>,
    pub http_client: Client,
    pub bundled_references: BundledReferenceLibrary,
    pub profiles_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub cache_archives_dir: PathBuf,
    pub cache_tmp_dir: PathBuf,
    pub dependency_index_cache: SharedDependencyCatalogIndexCache,
    pub launch_runtime_state: Arc<Mutex<LaunchRuntimeState>>,
    pub storage_migration_state: Arc<Mutex<StorageMigrationState>>,
}

impl AppState {
    pub fn new(app: &AppHandle) -> Result<Self, InternalError> {
        let app_data_dir = app.path().app_data_dir().map_err(|error| {
            InternalError::with_detail(
                "DB_INIT_FAILED",
                "Could not resolve app data directory",
                error.to_string(),
            )
        })?;

        std::fs::create_dir_all(app_data_dir.join("db"))?;
        std::fs::create_dir_all(app_data_dir.join("logs"))?;
        std::fs::create_dir_all(app_data_dir.join("state"))?;

        let db_path = app_data_dir.join("db").join("49modman.sqlite3");
        let connection = Connection::open(db_path)?;
        db::migrate(&connection)?;
        db::seed_defaults(&connection)?;

        let profiles_dir = resolve_storage_path_setting(
            &connection,
            "storage.profiles_dir",
            app_data_dir.join("profiles"),
        )?;
        let cache_dir = resolve_storage_path_setting(
            &connection,
            "storage.cache_dir",
            app_data_dir.join("cache"),
        )?;
        let cache_archives_dir = cache_dir.join("archives");
        let cache_tmp_dir = cache_dir.join("tmp");

        std::fs::create_dir_all(&profiles_dir)?;
        std::fs::create_dir_all(&cache_archives_dir)?;
        std::fs::create_dir_all(cache_archives_dir.join("thunderstore"))?;
        std::fs::create_dir_all(&cache_tmp_dir)?;

        Ok(Self {
            app_handle: app.clone(),
            app_data_dir,
            connection: Arc::new(Mutex::new(connection)),
            http_client: Client::builder()
                .user_agent("49modman/0.0.1")
                .build()
                .map_err(InternalError::from)?,
            bundled_references: load_bundled_reference_library()?,
            profiles_dir,
            cache_dir,
            cache_archives_dir,
            cache_tmp_dir,
            dependency_index_cache: new_dependency_catalog_index_cache(),
            launch_runtime_state: Arc::new(Mutex::new(LaunchRuntimeState::default())),
            storage_migration_state: Arc::new(Mutex::new(StorageMigrationState::default())),
        })
    }
}

fn resolve_storage_path_setting(
    connection: &Connection,
    key: &str,
    fallback: PathBuf,
) -> Result<PathBuf, InternalError> {
    let configured = get_setting(connection, key)?
        .and_then(|value_json| serde_json::from_str::<String>(&value_json).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);

    Ok(configured.unwrap_or(fallback))
}
