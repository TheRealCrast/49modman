use tauri::State;

use crate::{
    app_state::AppState,
    error::AppError,
    services::catalog_service::{
        get_catalog_summary as get_catalog_summary_service,
        get_package_detail as get_package_detail_service,
        search_packages as search_packages_service, sync_catalog as sync_catalog_service,
        CatalogSummaryDto, PackageCardDto, PackageDetailDto, SearchPackagesInput, SyncCatalogInput,
        SyncCatalogResult,
    },
};

#[tauri::command]
pub fn sync_catalog(
    state: State<'_, AppState>,
    input: Option<SyncCatalogInput>,
) -> Result<SyncCatalogResult, AppError> {
    sync_catalog_service(&state, input.unwrap_or(SyncCatalogInput { force: None }))
        .map_err(AppError::from)
}

#[tauri::command]
pub fn get_catalog_summary(state: State<'_, AppState>) -> Result<CatalogSummaryDto, AppError> {
    let connection = state
        .connection
        .lock()
        .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

    get_catalog_summary_service(&connection).map_err(AppError::from)
}

#[tauri::command]
pub fn search_packages(
    state: State<'_, AppState>,
    input: SearchPackagesInput,
) -> Result<Vec<PackageCardDto>, AppError> {
    let connection = state
        .connection
        .lock()
        .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

    search_packages_service(&connection, input).map_err(AppError::from)
}

#[tauri::command]
pub fn get_package_detail(
    state: State<'_, AppState>,
    package_id: String,
) -> Result<Option<PackageDetailDto>, AppError> {
    let connection = state
        .connection
        .lock()
        .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

    get_package_detail_service(&connection, &package_id).map_err(AppError::from)
}
