use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use reqwest::blocking::Client;
use rusqlite::Connection;
use tauri::{AppHandle, Manager};

use crate::{
    db,
    error::InternalError,
    resources::bundled_reference::{load_bundled_reference_library, BundledReferenceLibrary},
    services::dependency_service::{
        new_dependency_catalog_index_cache, SharedDependencyCatalogIndexCache,
    },
};

#[derive(Clone)]
pub struct AppState {
    pub connection: Arc<Mutex<Connection>>,
    pub http_client: Client,
    pub bundled_references: BundledReferenceLibrary,
    pub cache_dir: PathBuf,
    pub cache_archives_dir: PathBuf,
    pub cache_tmp_dir: PathBuf,
    pub dependency_index_cache: SharedDependencyCatalogIndexCache,
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

        let cache_dir = app_data_dir.join("cache");
        let cache_archives_dir = cache_dir.join("archives");
        let cache_tmp_dir = cache_dir.join("tmp");

        std::fs::create_dir_all(app_data_dir.join("db"))?;
        std::fs::create_dir_all(app_data_dir.join("logs"))?;
        std::fs::create_dir_all(app_data_dir.join("state"))?;
        std::fs::create_dir_all(&cache_archives_dir)?;
        std::fs::create_dir_all(cache_archives_dir.join("thunderstore"))?;
        std::fs::create_dir_all(&cache_tmp_dir)?;

        let db_path = app_data_dir.join("db").join("49modman.sqlite3");
        let connection = Connection::open(db_path)?;
        db::migrate(&connection)?;
        db::seed_defaults(&connection)?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            http_client: Client::builder()
                .user_agent("49modman/0.0.1")
                .build()
                .map_err(InternalError::from)?,
            bundled_references: load_bundled_reference_library()?,
            cache_dir,
            cache_archives_dir,
            cache_tmp_dir,
            dependency_index_cache: new_dependency_catalog_index_cache(),
        })
    }
}
