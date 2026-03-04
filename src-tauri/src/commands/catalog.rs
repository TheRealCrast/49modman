use tauri::{async_runtime, State};

use crate::{
    app_state::AppState,
    error::AppError,
    services::catalog_service::{
        get_catalog_summary as get_catalog_summary_service,
        get_package_detail as get_package_detail_service,
        search_packages as search_packages_service, sync_catalog as sync_catalog_service,
        CatalogSummaryDto, PackageDetailDto, SearchPackagesInput, SearchPackagesResult,
        SyncCatalogInput, SyncCatalogResult,
    },
};

#[tauri::command]
pub async fn sync_catalog(
    state: State<'_, AppState>,
    input: Option<SyncCatalogInput>,
) -> Result<SyncCatalogResult, AppError> {
    let state = state.inner().clone();
    let input = input.unwrap_or(SyncCatalogInput { force: None });

    async_runtime::spawn_blocking(move || {
        sync_catalog_service(&state, input).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("CATALOG_SYNC_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn get_catalog_summary(
    state: State<'_, AppState>,
) -> Result<CatalogSummaryDto, AppError> {
    let connection = state.connection.clone();

    async_runtime::spawn_blocking(move || {
        let connection = connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        get_catalog_summary_service(&connection).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("DB_INIT_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn search_packages(
    state: State<'_, AppState>,
    input: SearchPackagesInput,
) -> Result<SearchPackagesResult, AppError> {
    let connection = state.connection.clone();

    async_runtime::spawn_blocking(move || {
        let connection = connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        search_packages_service(&connection, input).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("DB_INIT_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn get_package_detail(
    state: State<'_, AppState>,
    package_id: String,
) -> Result<Option<PackageDetailDto>, AppError> {
    let connection = state.connection.clone();

    async_runtime::spawn_blocking(move || {
        let connection = connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        get_package_detail_service(&connection, &package_id).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("DB_INIT_FAILED", error.to_string()))?
}
