use tauri::{async_runtime, State};

use crate::{
    app_state::AppState,
    error::AppError,
    services::{
        cache_service::clear_cache_files,
        dependency_service::invalidate_dependency_catalog_index,
        profile_service::{
            clear_profiles_storage as clear_profiles_storage_service,
            create_profile as create_profile_service, delete_profile as delete_profile_service,
            delete_profile_storage as delete_profile_storage_service,
            ensure_all_profile_storage as ensure_all_profile_storage_service,
            ensure_profile_storage as ensure_profile_storage_service,
            get_active_profile as get_active_profile_service,
            get_profile_detail as get_profile_detail_service,
            get_profile_storage_size_bytes as get_profile_storage_size_bytes_service,
            get_profiles_storage_summary as get_profiles_storage_summary_service,
            list_profiles as list_profiles_service,
            open_active_profile_folder as open_active_profile_folder_service,
            open_profiles_folder as open_profiles_folder_service,
            reset_all_data as reset_all_data_service,
            set_active_profile as set_active_profile_service,
            update_profile as update_profile_service, CreateProfileInput, DeleteProfileResult,
            ProfileDetailDto, ProfileSummaryDto, ProfilesStorageSummaryDto, UpdateProfileInput,
        },
    },
};

#[tauri::command]
pub async fn list_profiles(state: State<'_, AppState>) -> Result<Vec<ProfileSummaryDto>, AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        let connection = state
            .connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        let mut profiles = list_profiles_service(&connection).map_err(AppError::from)?;

        for profile in &mut profiles {
            ensure_profile_storage_service(&state, &connection, &profile.id)
                .map_err(AppError::from)?;
            profile.profile_size_bytes =
                get_profile_storage_size_bytes_service(&state, &profile.id)
                    .map_err(AppError::from)?;
        }

        Ok(profiles)
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
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        let connection = state
            .connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        let profile = create_profile_service(&connection, input).map_err(AppError::from)?;
        ensure_profile_storage_service(&state, &connection, &profile.id).map_err(AppError::from)?;
        Ok(profile)
    })
    .await
    .map_err(|error| AppError::new("DB_INIT_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn update_profile(
    state: State<'_, AppState>,
    input: UpdateProfileInput,
) -> Result<ProfileDetailDto, AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        let connection = state
            .connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        let profile = update_profile_service(&connection, input).map_err(AppError::from)?;
        ensure_profile_storage_service(&state, &connection, &profile.id).map_err(AppError::from)?;
        Ok(profile)
    })
    .await
    .map_err(|error| AppError::new("DB_INIT_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn delete_profile(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<DeleteProfileResult, AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        let connection = state
            .connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        let deleted = delete_profile_service(&connection, &profile_id).map_err(AppError::from)?;
        delete_profile_storage_service(&state, &deleted.deleted_id).map_err(AppError::from)?;
        Ok(deleted)
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
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        clear_cache_files(&state).map_err(AppError::from)?;
        clear_profiles_storage_service(&state).map_err(AppError::from)?;

        let connection = state
            .connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        reset_all_data_service(&connection).map_err(AppError::from)?;
        ensure_all_profile_storage_service(&state, &connection).map_err(AppError::from)?;
        invalidate_dependency_catalog_index(&state.dependency_index_cache)
            .map_err(AppError::from)?;
        Ok(())
    })
    .await
    .map_err(|error| AppError::new("DB_INIT_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn open_profiles_folder(state: State<'_, AppState>) -> Result<(), AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        open_profiles_folder_service(&state).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("OPEN_PROFILES_FOLDER_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn open_active_profile_folder(state: State<'_, AppState>) -> Result<(), AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        let connection = state
            .connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        open_active_profile_folder_service(&state, &connection).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("OPEN_ACTIVE_PROFILE_FOLDER_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn get_profiles_storage_summary(
    state: State<'_, AppState>,
) -> Result<ProfilesStorageSummaryDto, AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        let connection = state
            .connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        get_profiles_storage_summary_service(&state, &connection).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("GET_PROFILES_STORAGE_SUMMARY_FAILED", error.to_string()))?
}
