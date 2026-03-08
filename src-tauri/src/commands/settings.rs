use tauri::{async_runtime, State};

use crate::{
    app_state::AppState,
    error::AppError,
    services::{
        settings_service::{
            get_warning_prefs as get_warning_prefs_service,
            set_warning_preference as set_warning_preference_service, WarningPrefsDto,
        },
        storage_service::{
            get_storage_locations as get_storage_locations_service,
            get_storage_migration_status as get_storage_migration_status_service,
            pick_storage_folder as pick_storage_folder_service,
            start_storage_migration as start_storage_migration_service, PickStorageFolderInput,
            StartStorageMigrationInput, StorageLocationsDto, StorageMigrationStatusDto,
        },
    },
};

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetWarningPreferenceInput {
    pub kind: String,
    pub enabled: bool,
}

#[tauri::command]
pub async fn get_warning_prefs(state: State<'_, AppState>) -> Result<WarningPrefsDto, AppError> {
    let connection = state.connection.clone();

    async_runtime::spawn_blocking(move || {
        let connection = connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        get_warning_prefs_service(&connection).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("DB_INIT_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn set_warning_preference(
    state: State<'_, AppState>,
    input: SetWarningPreferenceInput,
) -> Result<WarningPrefsDto, AppError> {
    let connection = state.connection.clone();

    async_runtime::spawn_blocking(move || {
        let connection = connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        set_warning_preference_service(&connection, &input.kind, input.enabled)
            .map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("DB_INIT_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn get_storage_locations(
    state: State<'_, AppState>,
) -> Result<StorageLocationsDto, AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || Ok(get_storage_locations_service(&state)))
        .await
        .map_err(|error| AppError::new("SETTINGS_LOAD_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn get_storage_migration_status(
    state: State<'_, AppState>,
) -> Result<StorageMigrationStatusDto, AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        get_storage_migration_status_service(&state).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("SETTINGS_LOAD_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn pick_storage_folder(
    state: State<'_, AppState>,
    input: PickStorageFolderInput,
) -> Result<Option<String>, AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        pick_storage_folder_service(&state, input).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("SETTINGS_LOAD_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn start_storage_migration(
    state: State<'_, AppState>,
    input: StartStorageMigrationInput,
) -> Result<StorageMigrationStatusDto, AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        start_storage_migration_service(&state, input).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("STORAGE_MIGRATION_FAILED", error.to_string()))?
}
