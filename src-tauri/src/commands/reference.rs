use tauri::{async_runtime, State};

use crate::{
    app_state::AppState,
    error::AppError,
    services::reference_service::{
        list_reference_rows as list_reference_rows_service,
        set_reference_state as set_reference_state_service, ListReferenceRowsInput,
        ListReferenceRowsResult, ReferenceRowDto, SetReferenceStateInput,
    },
};

#[tauri::command]
pub async fn list_reference_rows(
    state: State<'_, AppState>,
    input: ListReferenceRowsInput,
) -> Result<ListReferenceRowsResult, AppError> {
    let connection = state.connection.clone();

    async_runtime::spawn_blocking(move || {
        let connection = connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        list_reference_rows_service(&connection, input).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("DB_INIT_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn set_reference_state(
    state: State<'_, AppState>,
    input: SetReferenceStateInput,
) -> Result<ReferenceRowDto, AppError> {
    let connection = state.connection.clone();

    async_runtime::spawn_blocking(move || {
        let connection = connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        set_reference_state_service(&connection, input).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("DB_INIT_FAILED", error.to_string()))?
}
