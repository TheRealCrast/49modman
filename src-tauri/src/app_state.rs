use std::sync::Mutex;

use reqwest::blocking::Client;
use rusqlite::Connection;
use tauri::{AppHandle, Manager};

use crate::{
    db,
    error::InternalError,
    resources::bundled_reference::{load_bundled_reference_library, BundledReferenceLibrary},
};

pub struct AppState {
    pub connection: Mutex<Connection>,
    pub http_client: Client,
    pub bundled_references: BundledReferenceLibrary,
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

        Ok(Self {
            connection: Mutex::new(connection),
            http_client: Client::builder()
                .user_agent("49modman/0.0.1")
                .build()
                .map_err(InternalError::from)?,
            bundled_references: load_bundled_reference_library()?,
        })
    }
}
