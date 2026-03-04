use tauri::{async_runtime, State};

use crate::{
    app_state::AppState,
    error::AppError,
    services::dependency_service::{
        get_version_dependencies as get_version_dependencies_service, GetVersionDependenciesInput,
        VersionDependenciesDto, warm_dependency_catalog_index,
    },
};

#[tauri::command]
pub async fn get_version_dependencies(
    state: State<'_, AppState>,
    input: GetVersionDependenciesInput,
) -> Result<VersionDependenciesDto, AppError> {
    let connection = state.connection.clone();
    let dependency_index_cache = state.dependency_index_cache.clone();

    async_runtime::spawn_blocking(move || {
        let connection = connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        get_version_dependencies_service(&connection, &dependency_index_cache, input)
            .map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("DB_INIT_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn warm_dependency_index(state: State<'_, AppState>) -> Result<(), AppError> {
    let connection = state.connection.clone();
    let dependency_index_cache = state.dependency_index_cache.clone();

    async_runtime::spawn_blocking(move || {
        let connection = connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        warm_dependency_catalog_index(&connection, &dependency_index_cache).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("DB_INIT_FAILED", error.to_string()))?
}
