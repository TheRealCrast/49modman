use tauri::{async_runtime, State};

use crate::{
    app_state::AppState,
    error::AppError,
    services::profile_service::{
        create_profile as create_profile_service, delete_profile as delete_profile_service,
        get_active_profile as get_active_profile_service,
        get_profile_detail as get_profile_detail_service, list_profiles as list_profiles_service,
        reset_all_data as reset_all_data_service, set_active_profile as set_active_profile_service,
        update_profile as update_profile_service, CreateProfileInput, DeleteProfileResult,
        ProfileDetailDto, ProfileSummaryDto, UpdateProfileInput,
    },
};

#[tauri::command]
pub async fn list_profiles(state: State<'_, AppState>) -> Result<Vec<ProfileSummaryDto>, AppError> {
    let connection = state.connection.clone();

    async_runtime::spawn_blocking(move || {
        let connection = connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        list_profiles_service(&connection).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("DB_INIT_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn get_active_profile(
    state: State<'_, AppState>,
) -> Result<Option<ProfileDetailDto>, AppError> {
    let connection = state.connection.clone();

    async_runtime::spawn_blocking(move || {
        let connection = connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        get_active_profile_service(&connection).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("DB_INIT_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn set_active_profile(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<Option<ProfileDetailDto>, AppError> {
    let connection = state.connection.clone();

    async_runtime::spawn_blocking(move || {
        let connection = connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        set_active_profile_service(&connection, &profile_id).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("DB_INIT_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn create_profile(
    state: State<'_, AppState>,
    input: CreateProfileInput,
) -> Result<ProfileDetailDto, AppError> {
    let connection = state.connection.clone();

    async_runtime::spawn_blocking(move || {
        let connection = connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        create_profile_service(&connection, input).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("DB_INIT_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn update_profile(
    state: State<'_, AppState>,
    input: UpdateProfileInput,
) -> Result<ProfileDetailDto, AppError> {
    let connection = state.connection.clone();

    async_runtime::spawn_blocking(move || {
        let connection = connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        update_profile_service(&connection, input).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("DB_INIT_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn delete_profile(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<DeleteProfileResult, AppError> {
    let connection = state.connection.clone();

    async_runtime::spawn_blocking(move || {
        let connection = connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        delete_profile_service(&connection, &profile_id).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("DB_INIT_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn get_profile_detail(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<Option<ProfileDetailDto>, AppError> {
    let connection = state.connection.clone();

    async_runtime::spawn_blocking(move || {
        let connection = connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        get_profile_detail_service(&connection, &profile_id).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("DB_INIT_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn reset_all_data(state: State<'_, AppState>) -> Result<(), AppError> {
    let connection = state.connection.clone();

    async_runtime::spawn_blocking(move || {
        let connection = connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        reset_all_data_service(&connection).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("DB_INIT_FAILED", error.to_string()))?
}
