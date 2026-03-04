use tauri::{async_runtime, State};

use crate::{
    app_state::AppState,
    error::AppError,
    services::download_service::{
        get_task as get_task_service, list_active_downloads as list_active_downloads_service,
        DownloadJobDto, InstallTaskDto,
    },
};

#[tauri::command]
pub async fn list_active_downloads(
    state: State<'_, AppState>,
) -> Result<Vec<DownloadJobDto>, AppError> {
    let connection = state.connection.clone();

    async_runtime::spawn_blocking(move || {
        let connection = connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;
        list_active_downloads_service(&connection).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("DB_INIT_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn get_task(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<Option<InstallTaskDto>, AppError> {
    let connection = state.connection.clone();

    async_runtime::spawn_blocking(move || {
        let connection = connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;
        get_task_service(&connection, &task_id).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("DB_INIT_FAILED", error.to_string()))?
}
