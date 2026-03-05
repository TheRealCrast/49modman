use tauri::{async_runtime, State};

use crate::{
    app_state::AppState,
    error::AppError,
    services::{
        cache_service::{
            clear_cache as clear_cache_service,
            clear_cache_unreferenced as clear_cache_unreferenced_service,
            get_cache_summary as get_cache_summary_service,
            open_cache_folder as open_cache_folder_service,
            preview_clear_cache_unreferenced as preview_clear_cache_unreferenced_service,
            CachePrunePreviewDto, CacheSummaryDto,
        },
        download_service::{
            queue_install_to_cache as queue_install_to_cache_service, QueueInstallToCacheInput,
            QueueInstallToCacheResult,
        },
    },
};

#[tauri::command]
pub async fn queue_install_to_cache(
    state: State<'_, AppState>,
    input: QueueInstallToCacheInput,
) -> Result<QueueInstallToCacheResult, AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        queue_install_to_cache_service(&state, input).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("DOWNLOAD_TASK_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn get_cache_summary(state: State<'_, AppState>) -> Result<CacheSummaryDto, AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        let connection = state
            .connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;
        get_cache_summary_service(&state, &connection).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("DB_INIT_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn open_cache_folder(state: State<'_, AppState>) -> Result<(), AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || open_cache_folder_service(&state).map_err(AppError::from))
        .await
        .map_err(|error| AppError::new("OPEN_CACHE_FOLDER_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn clear_cache(state: State<'_, AppState>) -> Result<CacheSummaryDto, AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        let connection = state
            .connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;
        clear_cache_service(&state, &connection).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("CACHE_CLEAR_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn preview_clear_cache_unreferenced(
    state: State<'_, AppState>,
) -> Result<CachePrunePreviewDto, AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        let connection = state
            .connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;
        preview_clear_cache_unreferenced_service(&state, &connection).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("CACHE_CLEAR_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn clear_cache_unreferenced(
    state: State<'_, AppState>,
) -> Result<CacheSummaryDto, AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        let connection = state
            .connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;
        clear_cache_unreferenced_service(&state, &connection).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("CACHE_CLEAR_FAILED", error.to_string()))?
}
