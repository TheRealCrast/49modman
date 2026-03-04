use tauri::{async_runtime, State};

use crate::{app_state::AppState, error::AppError};

#[tauri::command]
pub async fn open_external_url(_state: State<'_, AppState>, url: String) -> Result<(), AppError> {
    async_runtime::spawn_blocking(move || webbrowser::open(&url).map(|_| ()))
        .await
        .map_err(|error| AppError::new("OPEN_URL_FAILED", error.to_string()))?
        .map_err(|error| AppError::new("OPEN_URL_FAILED", error.to_string()))
}
