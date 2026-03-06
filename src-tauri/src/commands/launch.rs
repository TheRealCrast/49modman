use tauri::{async_runtime, State};

use crate::{
    app_state::AppState,
    error::AppError,
    services::launch_service::{
        activate_profile as activate_profile_service,
        build_runtime_stage as build_runtime_stage_service,
        deactivate_to_vanilla as deactivate_to_vanilla_service,
        get_launch_runtime_status as get_launch_runtime_status_service,
        get_memory_diagnostics as get_memory_diagnostics_service,
        launch_profile as launch_profile_service, launch_vanilla as launch_vanilla_service,
        list_proton_runtimes as list_proton_runtimes_service,
        repair_activation as repair_activation_service,
        scan_steam_installations as scan_steam_installations_service,
        set_preferred_proton_runtime as set_preferred_proton_runtime_service,
        trim_resource_saver_memory as trim_resource_saver_memory_service,
        validate_v49_install as validate_v49_install_service, ActivateProfileInput,
        ActivationApplyResult, BuildRuntimeStageInput, LaunchProfileInput, LaunchResult,
        LaunchRuntimeStatus, LaunchVanillaInput, MemoryDiagnosticsSnapshot, ProtonRuntime,
        RuntimeStageBuildResult, SteamScanResult, TrimResourceMemoryResult, V49ValidationResult,
        ValidateV49InstallInput, VanillaCleanupResult,
    },
};

#[tauri::command]
pub async fn scan_steam_installations(
    _state: State<'_, AppState>,
) -> Result<SteamScanResult, AppError> {
    async_runtime::spawn_blocking(move || {
        scan_steam_installations_service().map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("STEAM_SCAN_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn validate_v49_install(
    state: State<'_, AppState>,
    input: Option<ValidateV49InstallInput>,
) -> Result<V49ValidationResult, AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        let connection = state
            .connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        validate_v49_install_service(&state, &connection, input.unwrap_or_default())
            .map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("VALIDATE_V49_INSTALL_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn build_runtime_stage(
    state: State<'_, AppState>,
    input: Option<BuildRuntimeStageInput>,
) -> Result<RuntimeStageBuildResult, AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        let connection = state
            .connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        build_runtime_stage_service(&state, &connection, input.unwrap_or_default())
            .map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("RUNTIME_STAGE_BUILD_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn activate_profile(
    state: State<'_, AppState>,
    input: Option<ActivateProfileInput>,
) -> Result<ActivationApplyResult, AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        let connection = state
            .connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        activate_profile_service(&state, &connection, input.unwrap_or_default())
            .map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("ACTIVATION_APPLY_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn deactivate_to_vanilla(
    state: State<'_, AppState>,
) -> Result<VanillaCleanupResult, AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        deactivate_to_vanilla_service(&state).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("VANILLA_CLEANUP_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn repair_activation(
    state: State<'_, AppState>,
) -> Result<VanillaCleanupResult, AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || repair_activation_service(&state).map_err(AppError::from))
        .await
        .map_err(|error| AppError::new("REPAIR_ACTIVATION_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn get_launch_runtime_status(
    state: State<'_, AppState>,
) -> Result<LaunchRuntimeStatus, AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        get_launch_runtime_status_service(&state).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("GET_LAUNCH_RUNTIME_STATUS_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn get_memory_diagnostics(
    state: State<'_, AppState>,
) -> Result<MemoryDiagnosticsSnapshot, AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        get_memory_diagnostics_service(&state).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("GET_MEMORY_DIAGNOSTICS_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn trim_resource_saver_memory(
    state: State<'_, AppState>,
) -> Result<TrimResourceMemoryResult, AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        let connection = state
            .connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        trim_resource_saver_memory_service(&state, &connection).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("TRIM_RESOURCE_MEMORY_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn launch_profile(
    state: State<'_, AppState>,
    input: LaunchProfileInput,
) -> Result<LaunchResult, AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        let connection = state
            .connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        launch_profile_service(&state, &connection, input).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("LAUNCH_PROFILE_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn launch_vanilla(
    state: State<'_, AppState>,
    input: LaunchVanillaInput,
) -> Result<LaunchResult, AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        let connection = state
            .connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        launch_vanilla_service(&state, &connection, input).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("LAUNCH_VANILLA_FAILED", error.to_string()))?
}

#[tauri::command]
pub async fn list_proton_runtimes(
    _state: State<'_, AppState>,
) -> Result<Vec<ProtonRuntime>, AppError> {
    async_runtime::spawn_blocking(move || list_proton_runtimes_service().map_err(AppError::from))
        .await
        .map_err(|error| AppError::new("LIST_PROTON_RUNTIMES_FAILED", error.to_string()))?
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPreferredProtonRuntimeInput {
    pub runtime_id: String,
}

#[tauri::command]
pub async fn set_preferred_proton_runtime(
    state: State<'_, AppState>,
    input: SetPreferredProtonRuntimeInput,
) -> Result<(), AppError> {
    let state = state.inner().clone();

    async_runtime::spawn_blocking(move || {
        let connection = state
            .connection
            .lock()
            .map_err(|_| AppError::new("DB_INIT_FAILED", "Failed to lock the SQLite connection"))?;

        set_preferred_proton_runtime_service(&connection, &input.runtime_id).map_err(AppError::from)
    })
    .await
    .map_err(|error| AppError::new("SET_PREFERRED_PROTON_RUNTIME_FAILED", error.to_string()))?
}
