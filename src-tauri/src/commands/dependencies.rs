use tauri::{async_runtime, State};

use crate::{
    app_state::AppState,
    error::AppError,
    services::dependency_service::{
        get_version_dependencies as get_version_dependencies_service, GetVersionDependenciesInput,
        VersionDependenciesDto,
    },
};

#[tauri::command]
pub async fn get_version_dependencies(
    state: State<'_, AppState>,
    input: GetVersionDependenciesInput,
) -> Result<VersionDependenciesDto, AppError> {
    let connection = state.connection.clone();

    async_runtime::spawn_blocking(move || {
        let connection = connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        get_version_dependencies_service(&connection, input).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("DB_INIT_FAILED", error.to_string()))?
}
