use tauri::State;

use crate::{
    app_state::AppState,
    error::AppError,
    services::reference_service::{
        list_reference_rows as list_reference_rows_service,
        set_reference_state as set_reference_state_service, ReferenceRowDto,
        SetReferenceStateInput,
    },
};

#[tauri::command]
pub fn list_reference_rows(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<ReferenceRowDto>, AppError> {
    let connection = state
        .connection
        .lock()
        .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

    list_reference_rows_service(&connection, &query).map_err(AppError::from)
}

#[tauri::command]
pub fn set_reference_state(
    state: State<'_, AppState>,
    input: SetReferenceStateInput,
) -> Result<ReferenceRowDto, AppError> {
    let connection = state
        .connection
        .lock()
        .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

    set_reference_state_service(&connection, input).map_err(AppError::from)
}
