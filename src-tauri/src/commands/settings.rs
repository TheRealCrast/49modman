use tauri::{async_runtime, State};

use crate::{
    app_state::AppState,
    error::AppError,
    services::settings_service::{
        get_warning_prefs as get_warning_prefs_service,
        set_warning_preference as set_warning_preference_service, WarningPrefsDto,
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
