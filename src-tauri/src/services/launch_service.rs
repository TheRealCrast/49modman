use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

use crate::{
    app_state::{AppState, LaunchRuntimeState},
    db::{get_setting, now_rfc3339, upsert_setting},
    error::InternalError,
    services::dependency_service::invalidate_dependency_catalog_index,
    services::profile_service::{get_active_profile_id, read_profile_manifest_mods},
};

const GAME_FOLDER_NAME: &str = "Lethal Company";
const GAME_EXECUTABLE_NAME: &str = "Lethal Company.exe";
const GAME_DATA_DIR_NAME: &str = "Lethal Company_Data";
const PROFILE_RUNTIME_DIR_NAME: &str = "runtime";
const PROFILE_ACTIVE_STAGE_DIR_NAME: &str = "active-stage";
const PROFILE_ACTIVE_STAGE_TMP_PREFIX: &str = "active-stage.tmp-";
const ACTIVATION_MANIFEST_SCHEMA_VERSION: u32 = 1;
const ACTIVATION_MANIFEST_FILE_NAME: &str = "activation-manifest-v1.json";
const STEAM_APP_ID: &str = "1966720";
const BUILTIN_V49_SIGNATURE_SHA256: &[&str] =
    &["469f208de455fcb6d334b6ec3655102ae6893de374f890961ab9f317bdfb2c8c"];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SteamScanResult {
    pub steam_root_paths: Vec<String>,
    pub library_paths: Vec<String>,
    pub game_paths: Vec<String>,
    pub selected_game_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ValidateV49InstallInput {
    #[serde(default, alias = "gamePath")]
    pub game_path_override: Option<String>,
    #[serde(default)]
    pub profile_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BuildRuntimeStageInput {
    #[serde(default)]
    pub profile_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ActivateProfileInput {
    #[serde(default)]
    pub profile_id: Option<String>,
    #[serde(default, alias = "gamePath")]
    pub game_path_override: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchProfileInput {
    pub profile_id: String,
    pub launch_mode: String,
    #[serde(default)]
    pub game_path_override: Option<String>,
    #[serde(default)]
    pub proton_runtime_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchVanillaInput {
    pub launch_mode: String,
    #[serde(default)]
    pub game_path_override: Option<String>,
    #[serde(default)]
    pub proton_runtime_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V49ValidationCheck {
    pub key: String,
    pub ok: bool,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V49ValidationResult {
    pub ok: bool,
    pub code: String,
    pub message: String,
    pub resolved_game_path: Option<String>,
    pub resolved_from: Option<String>,
    pub selected_profile_id: Option<String>,
    pub checks: Vec<V49ValidationCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detected_executable_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hardlink_supported: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStageSourceMod {
    pub package_id: String,
    pub package_name: String,
    pub version_id: String,
    pub version_number: String,
    pub install_dir: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStageBuildResult {
    pub profile_id: String,
    pub stage_path: String,
    pub merged_mod_count: usize,
    pub copied_file_count: usize,
    pub overwritten_file_count: usize,
    pub source_mods: Vec<RuntimeStageSourceMod>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivationApplyResult {
    pub ok: bool,
    pub code: String,
    pub message: String,
    pub profile_id: String,
    pub game_path: String,
    pub stage_path: String,
    pub manifest_path: String,
    pub cleaned_previous_activation: bool,
    pub file_count: usize,
    pub dir_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VanillaCleanupResult {
    pub ok: bool,
    pub code: String,
    pub message: String,
    pub manifest_path: Option<String>,
    pub game_path: Option<String>,
    pub removed_file_count: usize,
    pub removed_dir_count: usize,
    pub missing_entry_count: usize,
    pub remaining_entry_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchResult {
    pub ok: bool,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_game_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_profile_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_launch_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRuntimeStatus {
    pub is_game_running: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrimResourceMemoryResult {
    pub ok: bool,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryDiagnosticsProcess {
    pub pid: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_pid: Option<u32>,
    pub name: String,
    pub role: String,
    pub rss_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pss_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swap_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryDiagnosticsTotals {
    pub rss_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pss_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swap_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryDiagnosticsSnapshot {
    pub captured_at: String,
    pub platform: String,
    pub processes: Vec<MemoryDiagnosticsProcess>,
    pub totals: MemoryDiagnosticsTotals,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtonRuntime {
    pub id: String,
    pub display_name: String,
    pub path: String,
    pub source: String,
    pub is_valid: bool,
}

#[derive(Debug, Clone)]
struct ResolvedGamePath {
    path: PathBuf,
    source: &'static str,
}

#[derive(Debug, Clone)]
struct SteamLaunchOptionsRecord {
    value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivationManifestV1 {
    schema_version: u32,
    created_at: String,
    updated_at: String,
    profile_id: String,
    game_path: String,
    platform: String,
    mode: String,
    entries: Vec<ActivationManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivationManifestEntry {
    relative_path: String,
    kind: String,
    source: String,
    operation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sha256: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct ActivationCleanupStats {
    removed_file_count: usize,
    removed_dir_count: usize,
    missing_entry_count: usize,
    remaining_file_count: usize,
    remaining_dir_count: usize,
    remaining_entry_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LaunchDiagnosticsRecord {
    timestamp: String,
    variant: String,
    launch_mode: String,
    profile_id: Option<String>,
    game_path: Option<String>,
    ok: bool,
    code: String,
    message: String,
    pid: Option<u32>,
}

#[derive(Debug, Clone)]
struct LaunchReservation {
    shared_state: Arc<Mutex<LaunchRuntimeState>>,
}

impl LaunchReservation {
    fn reserve(state: &AppState) -> Result<Self, InternalError> {
        let mut launch_state = state.launch_runtime_state.lock().map_err(|_| {
            InternalError::app(
                "LAUNCH_STATE_LOCK_FAILED",
                "Failed to lock launch runtime state.",
            )
        })?;

        if launch_state.launch_in_progress {
            return Err(InternalError::app(
                "LAUNCH_ALREADY_IN_PROGRESS",
                "A launch is already in progress. Wait for it to finish before launching again.",
            ));
        }

        if let Some(pid) = launch_state.tracked_game_pid {
            if is_tracked_game_pid_running(pid)? {
                return Err(InternalError::app(
                    "LAUNCH_ALREADY_RUNNING",
                    format!(
                        "Lethal Company is already running (pid {pid}). Close it before launching again."
                    ),
                ));
            }

            launch_state.tracked_game_pid = None;
        }

        if is_game_process_running()? {
            return Err(InternalError::app(
                "LAUNCH_ALREADY_RUNNING",
                "Lethal Company is already running. Close it before launching again.",
            ));
        }

        launch_state.launch_in_progress = true;
        Ok(Self {
            shared_state: Arc::clone(&state.launch_runtime_state),
        })
    }

    fn track_direct_launch_pid(&self, pid: u32) -> Result<(), InternalError> {
        let mut launch_state = self.shared_state.lock().map_err(|_| {
            InternalError::app(
                "LAUNCH_STATE_LOCK_FAILED",
                "Failed to lock launch runtime state.",
            )
        })?;
        launch_state.tracked_game_pid = Some(pid);
        Ok(())
    }
}

impl Drop for LaunchReservation {
    fn drop(&mut self) {
        if let Ok(mut launch_state) = self.shared_state.lock() {
            launch_state.launch_in_progress = false;
        }
    }
}

pub fn scan_steam_installations() -> Result<SteamScanResult, InternalError> {
    let steam_roots = collect_steam_root_candidates();
    let mut library_paths = BTreeSet::<PathBuf>::new();

    for root in &steam_roots {
        if !root.join("steamapps").is_dir() {
            continue;
        }

        library_paths.insert(root.clone());

        let library_vdf = root.join("steamapps").join("libraryfolders.vdf");
        for library in parse_libraryfolders(&library_vdf)? {
            library_paths.insert(library);
        }
    }

    let mut game_paths = BTreeSet::<PathBuf>::new();
    for library in &library_paths {
        let candidate = library
            .join("steamapps")
            .join("common")
            .join(GAME_FOLDER_NAME);
        if candidate.is_dir() {
            game_paths.insert(candidate);
        }
    }

    let steam_root_paths = steam_roots
        .iter()
        .map(|path| path_to_string(path))
        .collect::<Vec<_>>();
    let library_paths = library_paths
        .iter()
        .map(|path| path_to_string(path))
        .collect::<Vec<_>>();
    let game_paths = game_paths
        .iter()
        .map(|path| path_to_string(path))
        .collect::<Vec<_>>();

    Ok(SteamScanResult {
        selected_game_path: game_paths.first().cloned(),
        steam_root_paths,
        library_paths,
        game_paths,
    })
}

pub fn build_runtime_stage(
    state: &AppState,
    connection: &Connection,
    input: BuildRuntimeStageInput,
) -> Result<RuntimeStageBuildResult, InternalError> {
    let Some(profile_id) = resolve_profile_id(connection, input.profile_id.as_deref())? else {
        return Err(InternalError::app(
            "PROFILE_NOT_FOUND",
            "Cannot build runtime stage because the selected profile does not exist.",
        ));
    };

    let profile_dir = profile_root_dir(state, &profile_id);
    let runtime_dir = profile_dir.join(PROFILE_RUNTIME_DIR_NAME);
    let stage_dir = runtime_dir.join(PROFILE_ACTIVE_STAGE_DIR_NAME);

    fs::create_dir_all(&runtime_dir)?;
    cleanup_stale_stage_temp_dirs(&runtime_dir)?;

    let stage_tmp_dir = runtime_dir.join(format!(
        "{PROFILE_ACTIVE_STAGE_TMP_PREFIX}{}",
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    if stage_tmp_dir.exists() {
        fs::remove_dir_all(&stage_tmp_dir)?;
    }
    fs::create_dir_all(&stage_tmp_dir)?;

    let build_outcome = (|| {
        fs::create_dir_all(stage_tmp_dir.join("BepInEx").join("plugins"))?;
        fs::create_dir_all(stage_tmp_dir.join("BepInEx").join("config"))?;

        let mut enabled_mods = read_profile_manifest_mods(state, &profile_id)?
            .into_iter()
            .filter(|entry| entry.enabled)
            .collect::<Vec<_>>();

        enabled_mods.sort_by(|left, right| {
            left.package_name
                .to_ascii_lowercase()
                .cmp(&right.package_name.to_ascii_lowercase())
                .then_with(|| left.version_number.cmp(&right.version_number))
                .then_with(|| left.package_id.cmp(&right.package_id))
                .then_with(|| left.version_id.cmp(&right.version_id))
        });

        let mut copied_file_count = 0_usize;
        let mut overwritten_file_count = 0_usize;
        let mut source_mods = Vec::with_capacity(enabled_mods.len());

        for mod_entry in &enabled_mods {
            let install_root =
                resolve_profile_install_dir(state, &profile_id, &mod_entry.install_dir)
                    .ok_or_else(|| {
                        InternalError::app(
                            "PROFILE_MOD_INSTALL_DIR_INVALID",
                            format!(
                                "Install path is invalid for {} {}.",
                                mod_entry.package_name, mod_entry.version_number
                            ),
                        )
                    })?;

            if !install_root.exists() || !install_root.is_dir() {
                return Err(InternalError::app(
                    "PROFILE_MOD_INSTALL_DIR_MISSING",
                    format!(
                        "Missing install directory for {} {} at {}.",
                        mod_entry.package_name,
                        mod_entry.version_number,
                        path_to_string(&install_root)
                    ),
                ));
            }

            let mod_files = collect_relative_files(&install_root)?;
            for relative_path in mod_files {
                let Some(stage_relative_path) = normalize_stage_relative_path(&relative_path)
                else {
                    continue;
                };

                if should_skip_stage_path(&stage_relative_path) {
                    continue;
                }

                let source_path = install_root.join(&relative_path);
                let target_path = stage_tmp_dir.join(&stage_relative_path);

                if target_path.exists() {
                    overwritten_file_count = overwritten_file_count.saturating_add(1);
                    if target_path.is_dir() {
                        fs::remove_dir_all(&target_path)?;
                    } else {
                        fs::remove_file(&target_path)?;
                    }
                }

                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                fs::copy(source_path, target_path)?;
                copied_file_count = copied_file_count.saturating_add(1);
            }

            source_mods.push(RuntimeStageSourceMod {
                package_id: mod_entry.package_id.clone(),
                package_name: mod_entry.package_name.clone(),
                version_id: mod_entry.version_id.clone(),
                version_number: mod_entry.version_number.clone(),
                install_dir: mod_entry.install_dir.clone(),
            });
        }

        if stage_dir.exists() {
            fs::remove_dir_all(&stage_dir)?;
        }
        fs::rename(&stage_tmp_dir, &stage_dir)?;

        Ok(RuntimeStageBuildResult {
            profile_id,
            stage_path: path_to_string(&stage_dir),
            merged_mod_count: source_mods.len(),
            copied_file_count,
            overwritten_file_count,
            source_mods,
        })
    })();

    if build_outcome.is_err() {
        let _ = fs::remove_dir_all(&stage_tmp_dir);
    }

    build_outcome
}

pub fn activate_profile(
    state: &AppState,
    connection: &Connection,
    input: ActivateProfileInput,
) -> Result<ActivationApplyResult, InternalError> {
    let Some(profile_id) = resolve_profile_id(connection, input.profile_id.as_deref())? else {
        return Err(InternalError::app(
            "PROFILE_NOT_FOUND",
            "Cannot activate because the selected profile does not exist.",
        ));
    };

    let preflight = validate_v49_install(
        state,
        connection,
        ValidateV49InstallInput {
            game_path_override: input.game_path_override.clone(),
            profile_id: Some(profile_id.clone()),
        },
    )?;
    if !preflight.ok {
        return Err(InternalError::with_detail(
            "PRECHECK_FAILED",
            "Activation preflight failed.",
            format!("{}: {}", preflight.code, preflight.message),
        ));
    }

    let stage_result = build_runtime_stage(
        state,
        connection,
        BuildRuntimeStageInput {
            profile_id: Some(profile_id.clone()),
        },
    )?;
    let stage_path = PathBuf::from(stage_result.stage_path.clone());
    if !stage_path.is_dir() {
        return Err(InternalError::app(
            "RUNTIME_STAGE_MISSING",
            "Runtime stage is missing after build. Try rebuilding stage.",
        ));
    }

    let game_path_string = preflight.resolved_game_path.ok_or_else(|| {
        InternalError::app(
            "GAME_PATH_RESOLUTION_FAILED",
            "Activation could not resolve a target game path.",
        )
    })?;
    let game_path = PathBuf::from(game_path_string.clone());

    let cleanup_before_apply = deactivate_manifest_entries(state, true)?;
    if !cleanup_before_apply.ok {
        return Err(InternalError::with_detail(
            "ACTIVATION_STALE_CLEANUP_FAILED",
            "Failed to cleanup stale activation before applying a new activation.",
            cleanup_before_apply.message,
        ));
    }

    let stage_files = collect_relative_files(&stage_path)?;
    let prefer_hardlink = detect_hardlink_support(&game_path).unwrap_or(false);

    let mut file_entries = Vec::<ActivationManifestEntry>::with_capacity(stage_files.len());
    let mut created_dirs = BTreeSet::<String>::new();

    for relative_path in stage_files {
        let source_path = stage_path.join(&relative_path);
        let target_path = game_path.join(&relative_path);

        if target_path.exists() {
            return Err(InternalError::with_detail(
                "ACTIVATION_TARGET_CONFLICT",
                "Activation would overwrite an unmanaged game file.",
                path_to_string(&target_path),
            ));
        }

        if let Some(relative_parent) = relative_path.parent() {
            ensure_target_parent_dirs(&game_path, relative_parent, &mut created_dirs)?;
        }

        let operation = if prefer_hardlink {
            if fs::hard_link(&source_path, &target_path).is_ok() {
                "hardlink"
            } else {
                fs::copy(&source_path, &target_path)?;
                "copy"
            }
        } else {
            fs::copy(&source_path, &target_path)?;
            "copy"
        };

        file_entries.push(ActivationManifestEntry {
            relative_path: relative_path_to_slash_string(&relative_path),
            kind: "file".to_string(),
            source: "stage".to_string(),
            operation: operation.to_string(),
            sha256: None,
        });
    }

    let mut dir_entries = created_dirs
        .iter()
        .map(|relative_path| ActivationManifestEntry {
            relative_path: relative_path.clone(),
            kind: "dir".to_string(),
            source: "generated".to_string(),
            operation: "copy".to_string(),
            sha256: None,
        })
        .collect::<Vec<_>>();

    // Cleanup should remove deepest directories first.
    dir_entries.sort_by(|left, right| {
        right
            .relative_path
            .split('/')
            .count()
            .cmp(&left.relative_path.split('/').count())
            .then_with(|| left.relative_path.cmp(&right.relative_path))
    });

    let mut entries = file_entries.clone();
    entries.extend(dir_entries.clone());

    let now = now_rfc3339()?;
    let manifest = ActivationManifestV1 {
        schema_version: ACTIVATION_MANIFEST_SCHEMA_VERSION,
        created_at: now.clone(),
        updated_at: now,
        profile_id: profile_id.clone(),
        game_path: game_path_string.clone(),
        platform: current_platform_name().to_string(),
        mode: "modded".to_string(),
        entries,
    };

    write_activation_manifest(state, &manifest)?;

    Ok(ActivationApplyResult {
        ok: true,
        code: "ACTIVATION_APPLIED".to_string(),
        message: "Runtime stage was activated into the game directory.".to_string(),
        profile_id,
        game_path: game_path_string,
        stage_path: stage_result.stage_path,
        manifest_path: path_to_string(&activation_manifest_path(state)?),
        cleaned_previous_activation: cleanup_before_apply.removed_file_count > 0
            || cleanup_before_apply.removed_dir_count > 0
            || cleanup_before_apply.missing_entry_count > 0,
        file_count: file_entries.len(),
        dir_count: dir_entries.len(),
    })
}

pub fn deactivate_to_vanilla(state: &AppState) -> Result<VanillaCleanupResult, InternalError> {
    deactivate_manifest_entries(state, true)
}

pub fn repair_activation(state: &AppState) -> Result<VanillaCleanupResult, InternalError> {
    deactivate_manifest_entries(state, true)
}

pub fn list_proton_runtimes() -> Result<Vec<ProtonRuntime>, InternalError> {
    discover_proton_runtimes()
}

pub fn get_launch_runtime_status(state: &AppState) -> Result<LaunchRuntimeStatus, InternalError> {
    let tracked_pid = {
        let launch_state = state.launch_runtime_state.lock().map_err(|_| {
            InternalError::app(
                "LAUNCH_STATE_LOCK_FAILED",
                "Failed to lock launch runtime state.",
            )
        })?;
        launch_state.tracked_game_pid
    };

    let tracked_pid_running = if let Some(pid) = tracked_pid {
        is_tracked_game_pid_running(pid)?
    } else {
        false
    };

    if !tracked_pid_running && tracked_pid.is_some() {
        if let Ok(mut launch_state) = state.launch_runtime_state.lock() {
            launch_state.tracked_game_pid = None;
        }
    }

    let is_game_running = if tracked_pid_running {
        true
    } else {
        is_game_process_running()?
    };

    Ok(LaunchRuntimeStatus { is_game_running })
}

pub fn get_memory_diagnostics(
    _state: &AppState,
) -> Result<MemoryDiagnosticsSnapshot, InternalError> {
    #[cfg(target_os = "linux")]
    {
        return get_memory_diagnostics_linux();
    }

    #[cfg(target_os = "windows")]
    {
        return get_memory_diagnostics_windows();
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        Ok(MemoryDiagnosticsSnapshot {
            captured_at: now_rfc3339()?,
            platform: std::env::consts::OS.to_string(),
            processes: Vec::new(),
            totals: MemoryDiagnosticsTotals {
                rss_bytes: 0,
                pss_bytes: None,
                private_bytes: None,
                shared_bytes: None,
                swap_bytes: None,
            },
            notes: vec!["Memory diagnostics is not supported on this platform.".to_string()],
        })
    }
}

pub fn trim_resource_saver_memory(
    state: &AppState,
    connection: &Connection,
) -> Result<TrimResourceMemoryResult, InternalError> {
    invalidate_dependency_catalog_index(&state.dependency_index_cache)?;
    connection.execute_batch("PRAGMA optimize; PRAGMA shrink_memory;")?;
    trim_allocator_memory();

    Ok(TrimResourceMemoryResult {
        ok: true,
        code: "RESOURCE_MEMORY_TRIMMED".to_string(),
        message: "Runtime caches were trimmed for resource saver mode.".to_string(),
    })
}

pub fn set_preferred_proton_runtime(
    connection: &Connection,
    runtime_id: &str,
) -> Result<(), InternalError> {
    let runtime_id = runtime_id.trim();
    if runtime_id.is_empty() {
        return Err(InternalError::app(
            "PROTON_RUNTIME_INVALID",
            "Proton runtime id cannot be empty.",
        ));
    }

    let runtimes = discover_proton_runtimes()?;
    let selected = runtimes
        .iter()
        .find(|runtime| runtime.id == runtime_id)
        .ok_or_else(|| {
            InternalError::app(
                "PROTON_RUNTIME_NOT_FOUND",
                "Requested Proton runtime was not found in discovered runtimes.",
            )
        })?;

    if !selected.is_valid {
        return Err(InternalError::app(
            "PROTON_RUNTIME_INVALID",
            "Requested Proton runtime is not valid.",
        ));
    }

    upsert_setting(
        connection,
        "launch.preferred_proton_runtime_id",
        &serde_json::to_string(&runtime_id)?,
        &now_rfc3339()?,
    )?;

    Ok(())
}

pub fn launch_profile(
    state: &AppState,
    connection: &Connection,
    input: LaunchProfileInput,
) -> Result<LaunchResult, InternalError> {
    let diagnostics_dir = ensure_launch_diagnostics_dir(state)?;
    let launch_mode = input.launch_mode.trim().to_ascii_lowercase();
    let profile_id = input.profile_id.trim().to_string();
    let _requested_proton_runtime = input.proton_runtime_id.clone();

    if profile_id.is_empty() {
        let result = LaunchResult {
            ok: false,
            code: "PROFILE_NOT_FOUND".to_string(),
            message: "Launch failed because no profile id was provided.".to_string(),
            pid: None,
            used_game_path: None,
            used_profile_id: None,
            used_launch_mode: Some(launch_mode.clone()),
            diagnostics_path: Some(path_to_string(&diagnostics_dir)),
        };
        write_launch_diagnostics(
            &diagnostics_dir,
            &LaunchDiagnosticsRecord {
                timestamp: now_rfc3339()?,
                variant: "modded".to_string(),
                launch_mode,
                profile_id: None,
                game_path: None,
                ok: result.ok,
                code: result.code.clone(),
                message: result.message.clone(),
                pid: None,
            },
        )?;
        return Ok(result);
    }

    let launch_reservation = match LaunchReservation::reserve(state) {
        Ok(reservation) => reservation,
        Err(error) => {
            let app_error = error.to_app_error();
            let result = LaunchResult {
                ok: false,
                code: app_error.code.to_string(),
                message: app_error.message,
                pid: None,
                used_game_path: None,
                used_profile_id: Some(profile_id.clone()),
                used_launch_mode: Some(launch_mode.clone()),
                diagnostics_path: Some(path_to_string(&diagnostics_dir)),
            };
            write_launch_diagnostics(
                &diagnostics_dir,
                &LaunchDiagnosticsRecord {
                    timestamp: now_rfc3339()?,
                    variant: "modded".to_string(),
                    launch_mode,
                    profile_id: Some(profile_id),
                    game_path: None,
                    ok: result.ok,
                    code: result.code.clone(),
                    message: result.message.clone(),
                    pid: None,
                },
            )?;
            return Ok(result);
        }
    };

    let preflight = validate_v49_install(
        state,
        connection,
        ValidateV49InstallInput {
            game_path_override: input.game_path_override.clone(),
            profile_id: Some(profile_id.clone()),
        },
    )?;
    if !preflight.ok {
        let result = LaunchResult {
            ok: false,
            code: "PRECHECK_FAILED".to_string(),
            message: format!("{}: {}", preflight.code, preflight.message),
            pid: None,
            used_game_path: preflight.resolved_game_path.clone(),
            used_profile_id: Some(profile_id.clone()),
            used_launch_mode: Some(launch_mode.clone()),
            diagnostics_path: Some(path_to_string(&diagnostics_dir)),
        };
        write_launch_diagnostics(
            &diagnostics_dir,
            &LaunchDiagnosticsRecord {
                timestamp: now_rfc3339()?,
                variant: "modded".to_string(),
                launch_mode,
                profile_id: Some(profile_id),
                game_path: preflight.resolved_game_path,
                ok: result.ok,
                code: result.code.clone(),
                message: result.message.clone(),
                pid: None,
            },
        )?;
        return Ok(result);
    }

    if cfg!(target_os = "linux") && launch_mode == "steam" {
        if let Err(error) = validate_linux_steam_modded_launch_options() {
            let app_error = error.to_app_error();
            let result = LaunchResult {
                ok: false,
                code: app_error.code.to_string(),
                message: app_error.message,
                pid: None,
                used_game_path: preflight.resolved_game_path.clone(),
                used_profile_id: Some(profile_id.clone()),
                used_launch_mode: Some(launch_mode.clone()),
                diagnostics_path: Some(path_to_string(&diagnostics_dir)),
            };
            write_launch_diagnostics(
                &diagnostics_dir,
                &LaunchDiagnosticsRecord {
                    timestamp: now_rfc3339()?,
                    variant: "modded".to_string(),
                    launch_mode,
                    profile_id: Some(profile_id),
                    game_path: preflight.resolved_game_path,
                    ok: result.ok,
                    code: result.code.clone(),
                    message: result.message.clone(),
                    pid: None,
                },
            )?;
            return Ok(result);
        }
    }

    let activation_result = match activate_profile(
        state,
        connection,
        ActivateProfileInput {
            profile_id: Some(profile_id.clone()),
            game_path_override: input.game_path_override.clone(),
        },
    ) {
        Ok(result) => result,
        Err(error) => {
            let app_error = error.to_app_error();
            let result = LaunchResult {
                ok: false,
                code: "ACTIVATION_FAILED".to_string(),
                message: format!("{}: {}", app_error.code, app_error.message),
                pid: None,
                used_game_path: preflight.resolved_game_path.clone(),
                used_profile_id: Some(profile_id.clone()),
                used_launch_mode: Some(launch_mode.clone()),
                diagnostics_path: Some(path_to_string(&diagnostics_dir)),
            };
            write_launch_diagnostics(
                &diagnostics_dir,
                &LaunchDiagnosticsRecord {
                    timestamp: now_rfc3339()?,
                    variant: "modded".to_string(),
                    launch_mode,
                    profile_id: Some(profile_id),
                    game_path: preflight.resolved_game_path,
                    ok: result.ok,
                    code: result.code.clone(),
                    message: result.message.clone(),
                    pid: None,
                },
            )?;
            return Ok(result);
        }
    };

    let game_path = activation_result.game_path.clone();
    let proton_runtime = if cfg!(target_os = "linux") && launch_mode == "direct" {
        match resolve_proton_runtime_selection(connection, input.proton_runtime_id.as_deref()) {
            Ok(runtime) => {
                if input
                    .proton_runtime_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .is_some()
                {
                    upsert_setting(
                        connection,
                        "launch.preferred_proton_runtime_id",
                        &serde_json::to_string(&runtime.id)?,
                        &now_rfc3339()?,
                    )?;
                }
                Some(runtime)
            }
            Err(error) => {
                let app_error = error.to_app_error();
                let result = LaunchResult {
                    ok: false,
                    code: app_error.code.to_string(),
                    message: app_error.message,
                    pid: None,
                    used_game_path: Some(game_path.clone()),
                    used_profile_id: Some(profile_id.clone()),
                    used_launch_mode: Some(launch_mode.clone()),
                    diagnostics_path: Some(path_to_string(&diagnostics_dir)),
                };
                write_launch_diagnostics(
                    &diagnostics_dir,
                    &LaunchDiagnosticsRecord {
                        timestamp: now_rfc3339()?,
                        variant: "modded".to_string(),
                        launch_mode,
                        profile_id: Some(profile_id),
                        game_path: Some(game_path),
                        ok: result.ok,
                        code: result.code.clone(),
                        message: result.message.clone(),
                        pid: None,
                    },
                )?;
                return Ok(result);
            }
        }
    } else {
        None
    };

    let launch_result = launch_game_process(
        &launch_mode,
        &PathBuf::from(game_path.clone()),
        &diagnostics_dir,
        proton_runtime.as_ref(),
        Some(resolve_steam_compat_data_path(
            state,
            Some(activation_result.profile_id.as_str()),
        )?),
    );

    let (ok, code, message, pid) = match launch_result {
        Ok(pid) => (
            true,
            "OK".to_string(),
            "Game launch command started successfully.".to_string(),
            Some(pid),
        ),
        Err(error) => (false, "LAUNCH_FAILED".to_string(), error.to_string(), None),
    };

    if ok && launch_mode == "direct" {
        if let Some(pid) = pid {
            launch_reservation.track_direct_launch_pid(pid)?;
        }
    }

    let result = LaunchResult {
        ok,
        code: code.clone(),
        message: message.clone(),
        pid,
        used_game_path: Some(game_path.clone()),
        used_profile_id: Some(profile_id.clone()),
        used_launch_mode: Some(launch_mode.clone()),
        diagnostics_path: Some(path_to_string(&diagnostics_dir)),
    };
    write_launch_diagnostics(
        &diagnostics_dir,
        &LaunchDiagnosticsRecord {
            timestamp: now_rfc3339()?,
            variant: "modded".to_string(),
            launch_mode,
            profile_id: Some(profile_id),
            game_path: Some(game_path),
            ok,
            code,
            message,
            pid,
        },
    )?;

    Ok(result)
}

pub fn launch_vanilla(
    state: &AppState,
    connection: &Connection,
    input: LaunchVanillaInput,
) -> Result<LaunchResult, InternalError> {
    let diagnostics_dir = ensure_launch_diagnostics_dir(state)?;
    let launch_mode = input.launch_mode.trim().to_ascii_lowercase();
    let _requested_proton_runtime = input.proton_runtime_id.clone();

    let launch_reservation = match LaunchReservation::reserve(state) {
        Ok(reservation) => reservation,
        Err(error) => {
            let app_error = error.to_app_error();
            let result = LaunchResult {
                ok: false,
                code: app_error.code.to_string(),
                message: app_error.message,
                pid: None,
                used_game_path: None,
                used_profile_id: None,
                used_launch_mode: Some(launch_mode.clone()),
                diagnostics_path: Some(path_to_string(&diagnostics_dir)),
            };
            write_launch_diagnostics(
                &diagnostics_dir,
                &LaunchDiagnosticsRecord {
                    timestamp: now_rfc3339()?,
                    variant: "vanilla".to_string(),
                    launch_mode,
                    profile_id: None,
                    game_path: None,
                    ok: result.ok,
                    code: result.code.clone(),
                    message: result.message.clone(),
                    pid: None,
                },
            )?;
            return Ok(result);
        }
    };

    let cleanup_result = deactivate_to_vanilla(state)?;
    if !cleanup_result.ok {
        let result = LaunchResult {
            ok: false,
            code: "ACTIVATION_FAILED".to_string(),
            message: cleanup_result.message.clone(),
            pid: None,
            used_game_path: cleanup_result.game_path.clone(),
            used_profile_id: None,
            used_launch_mode: Some(launch_mode.clone()),
            diagnostics_path: Some(path_to_string(&diagnostics_dir)),
        };
        write_launch_diagnostics(
            &diagnostics_dir,
            &LaunchDiagnosticsRecord {
                timestamp: now_rfc3339()?,
                variant: "vanilla".to_string(),
                launch_mode,
                profile_id: None,
                game_path: cleanup_result.game_path,
                ok: result.ok,
                code: result.code.clone(),
                message: result.message.clone(),
                pid: None,
            },
        )?;
        return Ok(result);
    }

    let preflight = validate_v49_install(
        state,
        connection,
        ValidateV49InstallInput {
            game_path_override: input
                .game_path_override
                .clone()
                .or_else(|| cleanup_result.game_path.clone()),
            profile_id: None,
        },
    )?;
    if !preflight.ok {
        let result = LaunchResult {
            ok: false,
            code: "PRECHECK_FAILED".to_string(),
            message: format!("{}: {}", preflight.code, preflight.message),
            pid: None,
            used_game_path: preflight.resolved_game_path.clone(),
            used_profile_id: preflight.selected_profile_id.clone(),
            used_launch_mode: Some(launch_mode.clone()),
            diagnostics_path: Some(path_to_string(&diagnostics_dir)),
        };
        write_launch_diagnostics(
            &diagnostics_dir,
            &LaunchDiagnosticsRecord {
                timestamp: now_rfc3339()?,
                variant: "vanilla".to_string(),
                launch_mode,
                profile_id: preflight.selected_profile_id,
                game_path: preflight.resolved_game_path,
                ok: result.ok,
                code: result.code.clone(),
                message: result.message.clone(),
                pid: None,
            },
        )?;
        return Ok(result);
    }

    let game_path = preflight.resolved_game_path.ok_or_else(|| {
        InternalError::app(
            "GAME_PATH_RESOLUTION_FAILED",
            "Vanilla launch failed because no game path was resolved.",
        )
    })?;
    let proton_runtime = if cfg!(target_os = "linux") && launch_mode == "direct" {
        match resolve_proton_runtime_selection(connection, input.proton_runtime_id.as_deref()) {
            Ok(runtime) => {
                if input
                    .proton_runtime_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .is_some()
                {
                    upsert_setting(
                        connection,
                        "launch.preferred_proton_runtime_id",
                        &serde_json::to_string(&runtime.id)?,
                        &now_rfc3339()?,
                    )?;
                }
                Some(runtime)
            }
            Err(error) => {
                let app_error = error.to_app_error();
                let result = LaunchResult {
                    ok: false,
                    code: app_error.code.to_string(),
                    message: app_error.message,
                    pid: None,
                    used_game_path: Some(game_path.clone()),
                    used_profile_id: preflight.selected_profile_id.clone(),
                    used_launch_mode: Some(launch_mode.clone()),
                    diagnostics_path: Some(path_to_string(&diagnostics_dir)),
                };
                write_launch_diagnostics(
                    &diagnostics_dir,
                    &LaunchDiagnosticsRecord {
                        timestamp: now_rfc3339()?,
                        variant: "vanilla".to_string(),
                        launch_mode,
                        profile_id: preflight.selected_profile_id,
                        game_path: Some(game_path),
                        ok: result.ok,
                        code: result.code.clone(),
                        message: result.message.clone(),
                        pid: None,
                    },
                )?;
                return Ok(result);
            }
        }
    } else {
        None
    };

    let launch_result = launch_game_process(
        &launch_mode,
        &PathBuf::from(game_path.clone()),
        &diagnostics_dir,
        proton_runtime.as_ref(),
        Some(resolve_steam_compat_data_path(state, Some("vanilla"))?),
    );

    let (ok, code, message, pid) = match launch_result {
        Ok(pid) => (
            true,
            "OK".to_string(),
            "Vanilla launch command started successfully.".to_string(),
            Some(pid),
        ),
        Err(error) => (false, "LAUNCH_FAILED".to_string(), error.to_string(), None),
    };

    if ok && launch_mode == "direct" {
        if let Some(pid) = pid {
            launch_reservation.track_direct_launch_pid(pid)?;
        }
    }

    let result = LaunchResult {
        ok,
        code: code.clone(),
        message: message.clone(),
        pid,
        used_game_path: Some(game_path.clone()),
        used_profile_id: preflight.selected_profile_id.clone(),
        used_launch_mode: Some(launch_mode.clone()),
        diagnostics_path: Some(path_to_string(&diagnostics_dir)),
    };
    write_launch_diagnostics(
        &diagnostics_dir,
        &LaunchDiagnosticsRecord {
            timestamp: now_rfc3339()?,
            variant: "vanilla".to_string(),
            launch_mode,
            profile_id: preflight.selected_profile_id,
            game_path: Some(game_path),
            ok,
            code,
            message,
            pid,
        },
    )?;

    Ok(result)
}

pub fn validate_v49_install(
    state: &AppState,
    connection: &Connection,
    input: ValidateV49InstallInput,
) -> Result<V49ValidationResult, InternalError> {
    let steam_scan = scan_steam_installations()?;
    let mut checks = Vec::<V49ValidationCheck>::new();

    let selected_profile_id = match resolve_profile_id(connection, input.profile_id.as_deref())? {
        Some(profile_id) => Some(profile_id),
        None => {
            checks.push(V49ValidationCheck {
                key: "profile".to_string(),
                ok: false,
                code: "PROFILE_NOT_FOUND".to_string(),
                message: "The selected profile does not exist.".to_string(),
                detail: None,
            });

            return Ok(V49ValidationResult {
                ok: false,
                code: "PROFILE_NOT_FOUND".to_string(),
                message: "Cannot validate because the selected profile does not exist.".to_string(),
                resolved_game_path: None,
                resolved_from: None,
                selected_profile_id: None,
                checks,
                detected_executable_sha256: None,
                hardlink_supported: None,
            });
        }
    };

    let resolved = resolve_game_path(
        connection,
        input.game_path_override.as_deref(),
        selected_profile_id.as_deref(),
        &steam_scan,
    )?;

    let Some(resolved) = resolved else {
        checks.push(V49ValidationCheck {
            key: "pathResolution".to_string(),
            ok: false,
            code: "GAME_PATH_RESOLUTION_FAILED".to_string(),
            message: "Could not resolve a game install path.".to_string(),
            detail: Some(
                "Set a game path on the profile, set launch.preferred_game_path, or install through Steam so scan fallback can find it."
                    .to_string(),
            ),
        });

        return Ok(V49ValidationResult {
            ok: false,
            code: "GAME_PATH_RESOLUTION_FAILED".to_string(),
            message: "Validation failed because no usable game path could be resolved.".to_string(),
            resolved_game_path: None,
            resolved_from: None,
            selected_profile_id,
            checks,
            detected_executable_sha256: None,
            hardlink_supported: None,
        });
    };

    checks.push(V49ValidationCheck {
        key: "pathResolution".to_string(),
        ok: true,
        code: "GAME_PATH_RESOLVED".to_string(),
        message: format!(
            "Resolved game path from {}.",
            match resolved.source {
                "input_override" => "explicit override",
                "profile_game_path" => "profile game path",
                "preferred_setting" => "stored preferred game path",
                "steam_scan" => "Steam scan fallback",
                _ => resolved.source,
            }
        ),
        detail: Some(path_to_string(&resolved.path)),
    });

    let executable_path = resolved.path.join(GAME_EXECUTABLE_NAME);
    if !executable_path.is_file() {
        checks.push(V49ValidationCheck {
            key: "gameExecutable".to_string(),
            ok: false,
            code: "GAME_EXECUTABLE_MISSING".to_string(),
            message: format!("Missing {} in the game root.", GAME_EXECUTABLE_NAME),
            detail: Some(path_to_string(&executable_path)),
        });

        return Ok(V49ValidationResult {
            ok: false,
            code: "GAME_EXECUTABLE_MISSING".to_string(),
            message: "Validation failed because the game executable is missing.".to_string(),
            resolved_game_path: Some(path_to_string(&resolved.path)),
            resolved_from: Some(resolved.source.to_string()),
            selected_profile_id,
            checks,
            detected_executable_sha256: None,
            hardlink_supported: None,
        });
    }

    checks.push(V49ValidationCheck {
        key: "gameExecutable".to_string(),
        ok: true,
        code: "GAME_EXECUTABLE_FOUND".to_string(),
        message: format!("Found {}.", GAME_EXECUTABLE_NAME),
        detail: Some(path_to_string(&executable_path)),
    });

    let data_path = resolved.path.join(GAME_DATA_DIR_NAME);
    if !data_path.is_dir() {
        checks.push(V49ValidationCheck {
            key: "unityDataDir".to_string(),
            ok: false,
            code: "GAME_DATA_DIR_MISSING".to_string(),
            message: format!("Missing {} in the game root.", GAME_DATA_DIR_NAME),
            detail: Some(path_to_string(&data_path)),
        });

        return Ok(V49ValidationResult {
            ok: false,
            code: "GAME_DATA_DIR_MISSING".to_string(),
            message: "Validation failed because the Unity data folder is missing.".to_string(),
            resolved_game_path: Some(path_to_string(&resolved.path)),
            resolved_from: Some(resolved.source.to_string()),
            selected_profile_id,
            checks,
            detected_executable_sha256: None,
            hardlink_supported: None,
        });
    }

    checks.push(V49ValidationCheck {
        key: "unityDataDir".to_string(),
        ok: true,
        code: "GAME_DATA_DIR_FOUND".to_string(),
        message: format!("Found {}.", GAME_DATA_DIR_NAME),
        detail: Some(path_to_string(&data_path)),
    });

    let executable_sha256 = calculate_sha256(&executable_path)?;
    match check_v49_signature(connection, &executable_sha256)? {
        SignatureCheckOutcome::Matched => {
            checks.push(V49ValidationCheck {
                key: "v49Signature".to_string(),
                ok: true,
                code: "V49_SIGNATURE_MATCHED".to_string(),
                message: "Executable hash matches a configured supported v49 signature."
                    .to_string(),
                detail: Some(executable_sha256.clone()),
            });
        }
        SignatureCheckOutcome::Unconfigured => {
            checks.push(V49ValidationCheck {
                key: "v49Signature".to_string(),
                ok: false,
                code: "V49_SIGNATURE_UNCONFIGURED".to_string(),
                message: "No supported v49 signature hashes are configured.".to_string(),
                detail: Some(
                    "Configure launch.v49_signature_hashes in settings to include known-good hashes."
                        .to_string(),
                ),
            });

            return Ok(V49ValidationResult {
                ok: false,
                code: "V49_SIGNATURE_UNCONFIGURED".to_string(),
                message: "Validation failed because no supported v49 signatures are configured."
                    .to_string(),
                resolved_game_path: Some(path_to_string(&resolved.path)),
                resolved_from: Some(resolved.source.to_string()),
                selected_profile_id,
                checks,
                detected_executable_sha256: Some(executable_sha256),
                hardlink_supported: None,
            });
        }
        SignatureCheckOutcome::Mismatched => {
            checks.push(V49ValidationCheck {
                key: "v49Signature".to_string(),
                ok: false,
                code: "V49_SIGNATURE_MISMATCH".to_string(),
                message: "Executable hash does not match any configured v49 signature.".to_string(),
                detail: Some(executable_sha256.clone()),
            });

            return Ok(V49ValidationResult {
                ok: false,
                code: "V49_SIGNATURE_MISMATCH".to_string(),
                message: "Validation failed because the install does not match a supported v49 signature."
                    .to_string(),
                resolved_game_path: Some(path_to_string(&resolved.path)),
                resolved_from: Some(resolved.source.to_string()),
                selected_profile_id,
                checks,
                detected_executable_sha256: Some(executable_sha256),
                hardlink_supported: None,
            });
        }
    }

    if let Err(error) = check_game_path_writable(&resolved.path) {
        checks.push(V49ValidationCheck {
            key: "activationWritable".to_string(),
            ok: false,
            code: "GAME_PATH_NOT_WRITABLE".to_string(),
            message: "Game path is not writable for activation.".to_string(),
            detail: Some(error.to_string()),
        });

        return Ok(V49ValidationResult {
            ok: false,
            code: "GAME_PATH_NOT_WRITABLE".to_string(),
            message: "Validation failed because the game path is not writable.".to_string(),
            resolved_game_path: Some(path_to_string(&resolved.path)),
            resolved_from: Some(resolved.source.to_string()),
            selected_profile_id,
            checks,
            detected_executable_sha256: Some(executable_sha256),
            hardlink_supported: None,
        });
    }

    checks.push(V49ValidationCheck {
        key: "activationWritable".to_string(),
        ok: true,
        code: "GAME_PATH_WRITABLE".to_string(),
        message: "Game path is writable for activation operations.".to_string(),
        detail: None,
    });

    let hardlink_supported = detect_hardlink_support(&resolved.path).unwrap_or(false);
    if hardlink_supported {
        checks.push(V49ValidationCheck {
            key: "filesystemHardlink".to_string(),
            ok: true,
            code: "HARDLINK_SUPPORTED".to_string(),
            message: "Filesystem supports hardlinks for activation.".to_string(),
            detail: None,
        });
    } else {
        checks.push(V49ValidationCheck {
            key: "filesystemHardlink".to_string(),
            ok: true,
            code: "HARDLINK_UNAVAILABLE_COPY_FALLBACK".to_string(),
            message: "Hardlinks are unavailable; activation will use copy fallback.".to_string(),
            detail: None,
        });
    }

    if let Some(profile_id) = selected_profile_id.as_deref() {
        match validate_enabled_dependency_state(state, connection, profile_id)? {
            DependencyValidationOutcome::Valid => {
                checks.push(V49ValidationCheck {
                    key: "enabledDependencies".to_string(),
                    ok: true,
                    code: "PROFILE_DEPENDENCY_STATE_VALID".to_string(),
                    message: "Enabled installed mods have satisfied dependency state.".to_string(),
                    detail: None,
                });
            }
            DependencyValidationOutcome::Invalid { detail } => {
                checks.push(V49ValidationCheck {
                    key: "enabledDependencies".to_string(),
                    ok: false,
                    code: "PROFILE_DEPENDENCY_STATE_INVALID".to_string(),
                    message:
                        "Enabled installed mods have missing or disabled required dependencies."
                            .to_string(),
                    detail: Some(detail),
                });

                return Ok(V49ValidationResult {
                    ok: false,
                    code: "PROFILE_DEPENDENCY_STATE_INVALID".to_string(),
                    message:
                        "Validation failed because enabled installed mods have dependency issues."
                            .to_string(),
                    resolved_game_path: Some(path_to_string(&resolved.path)),
                    resolved_from: Some(resolved.source.to_string()),
                    selected_profile_id,
                    checks,
                    detected_executable_sha256: Some(executable_sha256),
                    hardlink_supported: Some(hardlink_supported),
                });
            }
        }
    }

    upsert_setting(
        connection,
        "launch.preferred_game_path",
        &serde_json::to_string(&path_to_string(&resolved.path))?,
        &now_rfc3339()?,
    )?;

    Ok(V49ValidationResult {
        ok: true,
        code: "OK".to_string(),
        message: "Validation passed: game install and profile dependency state are ready."
            .to_string(),
        resolved_game_path: Some(path_to_string(&resolved.path)),
        resolved_from: Some(resolved.source.to_string()),
        selected_profile_id,
        checks,
        detected_executable_sha256: Some(executable_sha256),
        hardlink_supported: Some(hardlink_supported),
    })
}

fn deactivate_manifest_entries(
    state: &AppState,
    remove_manifest_on_success: bool,
) -> Result<VanillaCleanupResult, InternalError> {
    let manifest_path = activation_manifest_path(state)?;
    let Some(manifest) = read_activation_manifest(state)? else {
        return Ok(VanillaCleanupResult {
            ok: true,
            code: "NO_ACTIVE_MANIFEST".to_string(),
            message: "No activation manifest was found; game is already in vanilla state."
                .to_string(),
            manifest_path: Some(path_to_string(&manifest_path)),
            game_path: None,
            removed_file_count: 0,
            removed_dir_count: 0,
            missing_entry_count: 0,
            remaining_entry_count: 0,
        });
    };

    let game_root = PathBuf::from(manifest.game_path.clone());
    let cleanup_stats = cleanup_manifest_targets(&game_root, &manifest.entries)?;

    let ok = cleanup_stats.remaining_file_count == 0;

    if ok && remove_manifest_on_success {
        if manifest_path.is_file() {
            fs::remove_file(&manifest_path)?;
        }
    }

    let retained_dirs = cleanup_stats.remaining_dir_count;
    Ok(VanillaCleanupResult {
        ok,
        code: if ok {
            if retained_dirs > 0 {
                "VANILLA_CLEANUP_COMPLETE_WITH_RETAINED_DIRS".to_string()
            } else {
                "VANILLA_CLEANUP_COMPLETE".to_string()
            }
        } else {
            "VANILLA_CLEANUP_INCOMPLETE".to_string()
        },
        message: if ok {
            if retained_dirs > 0 {
                format!(
                    "Managed activation files were removed. Retained {retained_dirs} non-empty managed directories."
                )
            } else {
                "Managed activation files were removed and cleanup verification passed.".to_string()
            }
        } else {
            format!(
                "Cleanup finished with {} remaining managed file(s); run repair again.",
                cleanup_stats.remaining_file_count
            )
        },
        manifest_path: Some(path_to_string(&manifest_path)),
        game_path: Some(manifest.game_path),
        removed_file_count: cleanup_stats.removed_file_count,
        removed_dir_count: cleanup_stats.removed_dir_count,
        missing_entry_count: cleanup_stats.missing_entry_count,
        remaining_entry_count: cleanup_stats.remaining_entry_count,
    })
}

enum SignatureCheckOutcome {
    Matched,
    Unconfigured,
    Mismatched,
}

enum DependencyValidationOutcome {
    Valid,
    Invalid { detail: String },
}

fn resolve_profile_id(
    connection: &Connection,
    requested_profile_id: Option<&str>,
) -> Result<Option<String>, InternalError> {
    let requested_profile_id = requested_profile_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());

    match requested_profile_id {
        Some(profile_id) => {
            let exists = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM profiles WHERE id = ?1)",
                params![profile_id],
                |row| row.get::<_, i64>(0),
            )? != 0;

            if exists {
                Ok(Some(profile_id))
            } else {
                Ok(None)
            }
        }
        None => Ok(Some(get_active_profile_id(connection)?)),
    }
}

fn resolve_game_path(
    connection: &Connection,
    override_path: Option<&str>,
    profile_id: Option<&str>,
    steam_scan: &SteamScanResult,
) -> Result<Option<ResolvedGamePath>, InternalError> {
    if let Some(path) = normalize_candidate_path(override_path) {
        return Ok(Some(ResolvedGamePath {
            path,
            source: "input_override",
        }));
    }

    if let Some(profile_id) = profile_id {
        let profile_game_path = connection
            .query_row(
                "SELECT game_path FROM profiles WHERE id = ?1",
                params![profile_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;

        if let Some(path) = normalize_candidate_path(profile_game_path.as_deref()) {
            return Ok(Some(ResolvedGamePath {
                path,
                source: "profile_game_path",
            }));
        }
    }

    if let Some(path) = read_preferred_game_path(connection)? {
        return Ok(Some(ResolvedGamePath {
            path,
            source: "preferred_setting",
        }));
    }

    if let Some(path) = steam_scan
        .selected_game_path
        .as_deref()
        .and_then(|value| normalize_candidate_path(Some(value)))
    {
        return Ok(Some(ResolvedGamePath {
            path,
            source: "steam_scan",
        }));
    }

    Ok(None)
}

fn read_preferred_game_path(connection: &Connection) -> Result<Option<PathBuf>, InternalError> {
    let setting = get_setting(connection, "launch.preferred_game_path")?;
    let Some(setting) = setting else {
        return Ok(None);
    };

    match serde_json::from_str::<String>(&setting) {
        Ok(value) => Ok(normalize_candidate_path(Some(value.as_str()))),
        Err(_) => Ok(None),
    }
}

fn normalize_candidate_path(value: Option<&str>) -> Option<PathBuf> {
    let trimmed = value.map(str::trim).filter(|entry| !entry.is_empty())?;

    if let Some(stripped) = trimmed.strip_prefix('~') {
        let home = std::env::var_os("HOME")?;
        let mut path = PathBuf::from(home);
        let stripped = stripped.trim_start_matches('/').trim_start_matches('\\');
        if !stripped.is_empty() {
            path = path.join(stripped);
        }
        return Some(path);
    }

    Some(PathBuf::from(trimmed))
}

fn check_v49_signature(
    connection: &Connection,
    computed_sha256: &str,
) -> Result<SignatureCheckOutcome, InternalError> {
    let mut allowed_hashes = BUILTIN_V49_SIGNATURE_SHA256
        .iter()
        .filter_map(|value| normalize_sha256(value))
        .collect::<HashSet<_>>();

    if let Some(setting) = get_setting(connection, "launch.v49_signature_hash")? {
        if let Ok(value) = serde_json::from_str::<String>(&setting) {
            if let Some(value) = normalize_sha256(&value) {
                allowed_hashes.insert(value);
            }
        }
    }

    if let Some(setting) = get_setting(connection, "launch.v49_signature_hashes")? {
        if let Ok(values) = serde_json::from_str::<Vec<String>>(&setting) {
            for value in values {
                if let Some(value) = normalize_sha256(&value) {
                    allowed_hashes.insert(value);
                }
            }
        }
    }

    if allowed_hashes.is_empty() {
        return Ok(SignatureCheckOutcome::Unconfigured);
    }

    if allowed_hashes.contains(&computed_sha256.to_ascii_lowercase()) {
        Ok(SignatureCheckOutcome::Matched)
    } else {
        Ok(SignatureCheckOutcome::Mismatched)
    }
}

fn normalize_sha256(value: &str) -> Option<String> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.len() != 64 {
        return None;
    }

    if normalized.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Some(normalized)
    } else {
        None
    }
}

fn check_game_path_writable(game_root: &Path) -> Result<(), std::io::Error> {
    let probe_name = format!(
        ".49modman-write-probe-{}",
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    );
    let probe_path = game_root.join(probe_name);

    fs::write(&probe_path, b"probe")?;
    fs::remove_file(probe_path)
}

fn detect_hardlink_support(game_root: &Path) -> Option<bool> {
    let timestamp = OffsetDateTime::now_utc().unix_timestamp_nanos();
    let source = game_root.join(format!(".49modman-hardlink-source-{timestamp}"));
    let target = game_root.join(format!(".49modman-hardlink-target-{timestamp}"));

    if fs::write(&source, b"probe").is_err() {
        return None;
    }

    let supported = fs::hard_link(&source, &target).is_ok();

    let _ = fs::remove_file(&target);
    let _ = fs::remove_file(&source);

    Some(supported)
}

fn validate_enabled_dependency_state(
    state: &AppState,
    connection: &Connection,
    profile_id: &str,
) -> Result<DependencyValidationOutcome, InternalError> {
    let installed_mods = read_profile_manifest_mods(state, profile_id)?;
    if installed_mods.is_empty() {
        return Ok(DependencyValidationOutcome::Valid);
    }

    let enabled_mods = installed_mods
        .iter()
        .filter(|entry| entry.enabled)
        .collect::<Vec<_>>();

    if enabled_mods.is_empty() {
        return Ok(DependencyValidationOutcome::Valid);
    }

    let installed_enabled_by_raw = installed_mods
        .iter()
        .map(|entry| {
            (
                dependency_raw_key(&entry.package_name, &entry.version_number),
                entry.enabled,
            )
        })
        .collect::<HashMap<_, _>>();

    let mut dependency_statement =
        connection.prepare("SELECT dependencies_json FROM package_versions WHERE id = ?1")?;
    let mut issues = Vec::<String>::new();
    let mut dedupe = HashSet::<String>::new();

    for entry in enabled_mods {
        let dependencies_json = dependency_statement
            .query_row(params![entry.version_id], |row| row.get::<_, String>(0))
            .optional()?;

        let Some(dependencies_json) = dependencies_json else {
            let issue = format!(
                "{} {} dependency metadata is missing from the local catalog.",
                entry.package_name, entry.version_number
            );
            if dedupe.insert(issue.clone()) {
                issues.push(issue);
            }
            continue;
        };

        for dependency_raw in parse_dependency_entries(&dependencies_json) {
            let dependency_raw = dependency_raw.trim();
            if dependency_raw.is_empty() {
                continue;
            }

            match installed_enabled_by_raw.get(dependency_raw) {
                Some(true) => {}
                Some(false) => {
                    let issue = format!(
                        "{} {} requires {} but it is installed and disabled.",
                        entry.package_name, entry.version_number, dependency_raw
                    );
                    if dedupe.insert(issue.clone()) {
                        issues.push(issue);
                    }
                }
                None => {
                    let issue = format!(
                        "{} {} requires {} but it is not installed in the profile.",
                        entry.package_name, entry.version_number, dependency_raw
                    );
                    if dedupe.insert(issue.clone()) {
                        issues.push(issue);
                    }
                }
            }
        }
    }

    if issues.is_empty() {
        return Ok(DependencyValidationOutcome::Valid);
    }

    let max_issue_lines = 8;
    let overflow_count = issues.len().saturating_sub(max_issue_lines);
    let mut detail_lines = issues.into_iter().take(max_issue_lines).collect::<Vec<_>>();
    if overflow_count > 0 {
        detail_lines.push(format!("...and {overflow_count} more dependency issue(s)."));
    }

    Ok(DependencyValidationOutcome::Invalid {
        detail: detail_lines.join("\n"),
    })
}

fn cleanup_stale_stage_temp_dirs(runtime_dir: &Path) -> Result<(), InternalError> {
    if !runtime_dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(runtime_dir)? {
        let entry = entry?;
        let entry_name = entry.file_name();
        let entry_name = entry_name.to_string_lossy();
        if !entry_name.starts_with(PROFILE_ACTIVE_STAGE_TMP_PREFIX) {
            continue;
        }

        let entry_path = entry.path();
        if entry_path.is_dir() {
            fs::remove_dir_all(entry_path)?;
        } else if entry_path.exists() {
            fs::remove_file(entry_path)?;
        }
    }

    Ok(())
}

fn cleanup_manifest_targets(
    game_root: &Path,
    entries: &[ActivationManifestEntry],
) -> Result<ActivationCleanupStats, InternalError> {
    let mut stats = ActivationCleanupStats::default();
    let mut file_paths = Vec::<PathBuf>::new();
    let mut dir_paths = Vec::<PathBuf>::new();

    for entry in entries {
        let Some(path) = resolve_manifest_relative_path(game_root, &entry.relative_path) else {
            return Err(InternalError::with_detail(
                "ACTIVATION_MANIFEST_INVALID",
                "Activation manifest contains an unsafe relative path.",
                entry.relative_path.clone(),
            ));
        };

        if entry.kind == "dir" {
            dir_paths.push(path);
        } else {
            file_paths.push(path);
        }
    }

    for path in &file_paths {
        if !path.exists() {
            stats.missing_entry_count = stats.missing_entry_count.saturating_add(1);
            continue;
        }

        if path.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
        stats.removed_file_count = stats.removed_file_count.saturating_add(1);
    }

    dir_paths.sort_by(|left, right| {
        right
            .components()
            .count()
            .cmp(&left.components().count())
            .then_with(|| left.to_string_lossy().cmp(&right.to_string_lossy()))
    });

    for path in &dir_paths {
        if !path.exists() {
            stats.missing_entry_count = stats.missing_entry_count.saturating_add(1);
            continue;
        }

        if path.is_dir() {
            match fs::remove_dir(path) {
                Ok(()) => {
                    stats.removed_dir_count = stats.removed_dir_count.saturating_add(1);
                }
                Err(error) if error.kind() == std::io::ErrorKind::DirectoryNotEmpty => {}
                Err(error) => return Err(InternalError::from(error)),
            }
        } else {
            fs::remove_file(path)?;
            stats.removed_dir_count = stats.removed_dir_count.saturating_add(1);
        }
    }

    for path in &file_paths {
        if path.exists() {
            stats.remaining_file_count = stats.remaining_file_count.saturating_add(1);
        }
    }

    for path in &dir_paths {
        if path.exists() {
            stats.remaining_dir_count = stats.remaining_dir_count.saturating_add(1);
        }
    }
    stats.remaining_entry_count = stats
        .remaining_file_count
        .saturating_add(stats.remaining_dir_count);

    Ok(stats)
}

fn ensure_target_parent_dirs(
    game_root: &Path,
    relative_parent: &Path,
    created_dirs: &mut BTreeSet<String>,
) -> Result<(), InternalError> {
    let mut current_absolute = game_root.to_path_buf();
    let mut relative_accumulator = PathBuf::new();

    for segment in relative_parent.components() {
        let segment = segment.as_os_str();
        current_absolute = current_absolute.join(segment);
        relative_accumulator = relative_accumulator.join(segment);

        if !current_absolute.exists() {
            created_dirs.insert(relative_path_to_slash_string(&relative_accumulator));
        }
    }

    fs::create_dir_all(game_root.join(relative_parent))?;
    Ok(())
}

fn activation_manifest_path(state: &AppState) -> Result<PathBuf, InternalError> {
    let app_data = state.profiles_dir.parent().ok_or_else(|| {
        InternalError::app(
            "RESOURCE_LOAD_FAILED",
            "Failed to resolve app data root from profiles directory.",
        )
    })?;

    Ok(app_data.join("state").join(ACTIVATION_MANIFEST_FILE_NAME))
}

fn read_activation_manifest(
    state: &AppState,
) -> Result<Option<ActivationManifestV1>, InternalError> {
    let manifest_path = activation_manifest_path(state)?;
    if !manifest_path.is_file() {
        return Ok(None);
    }

    let bytes = fs::read(manifest_path)?;
    let manifest = serde_json::from_slice::<ActivationManifestV1>(&bytes)?;

    if manifest.schema_version != ACTIVATION_MANIFEST_SCHEMA_VERSION {
        return Err(InternalError::with_detail(
            "ACTIVATION_MANIFEST_UNSUPPORTED",
            "Activation manifest schema version is not supported.",
            manifest.schema_version.to_string(),
        ));
    }

    Ok(Some(manifest))
}

fn write_activation_manifest(
    state: &AppState,
    manifest: &ActivationManifestV1,
) -> Result<(), InternalError> {
    let manifest_path = activation_manifest_path(state)?;
    let parent = manifest_path.parent().ok_or_else(|| {
        InternalError::app(
            "RESOURCE_LOAD_FAILED",
            "Failed to resolve activation manifest parent directory.",
        )
    })?;
    fs::create_dir_all(parent)?;

    let temp_path = parent.join(format!(
        "{ACTIVATION_MANIFEST_FILE_NAME}.tmp-{}",
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    let json = serde_json::to_vec_pretty(manifest)?;
    fs::write(&temp_path, &json)?;
    fs::rename(temp_path, manifest_path)?;
    Ok(())
}

fn resolve_manifest_relative_path(root: &Path, relative: &str) -> Option<PathBuf> {
    let mut path = root.to_path_buf();
    for segment in relative.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return None;
        }
        path = path.join(segment);
    }
    Some(path)
}

fn relative_path_to_slash_string(relative: &Path) -> String {
    relative
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/")
}

fn current_platform_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    }
}

fn ensure_launch_diagnostics_dir(state: &AppState) -> Result<PathBuf, InternalError> {
    let app_data = state.profiles_dir.parent().ok_or_else(|| {
        InternalError::app(
            "RESOURCE_LOAD_FAILED",
            "Failed to resolve app data root for launch diagnostics.",
        )
    })?;

    let launch_logs_root = app_data.join("logs").join("launch");
    fs::create_dir_all(&launch_logs_root)?;

    let run_dir = launch_logs_root.join(format!(
        "run-{}",
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    fs::create_dir_all(&run_dir)?;
    Ok(run_dir)
}

fn write_launch_diagnostics(
    diagnostics_dir: &Path,
    record: &LaunchDiagnosticsRecord,
) -> Result<(), InternalError> {
    let json = serde_json::to_vec_pretty(record)?;
    let path = diagnostics_dir.join("launch.json");
    fs::write(path, json)?;
    Ok(())
}

#[derive(Debug, Clone)]
struct ProcessIdentity {
    pid: u32,
    parent_pid: Option<u32>,
    name: String,
    vm_rss_bytes: Option<u64>,
}

#[derive(Debug, Clone)]
struct ProcessMemoryStats {
    rss_bytes: u64,
    pss_bytes: Option<u64>,
    private_bytes: Option<u64>,
    shared_bytes: Option<u64>,
    swap_bytes: Option<u64>,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone)]
struct WindowsProcessIdentity {
    pid: u32,
    parent_pid: Option<u32>,
    name: String,
    working_set_bytes: u64,
}

#[cfg(target_os = "linux")]
fn get_memory_diagnostics_linux() -> Result<MemoryDiagnosticsSnapshot, InternalError> {
    let root_pid = std::process::id();
    let mut rows = Vec::<MemoryDiagnosticsProcess>::new();
    let mut notes = Vec::<String>::new();
    let mut skipped = 0usize;
    let mut memory_read_errors = 0usize;

    let (processes, status_read_errors) = collect_linux_process_tree(root_pid)?;

    for process in processes {
        match read_linux_process_memory_stats(process.pid, process.vm_rss_bytes) {
            Ok(Some(memory)) => rows.push(MemoryDiagnosticsProcess {
                pid: process.pid,
                parent_pid: process.parent_pid,
                name: process.name.clone(),
                role: classify_memory_process_role(root_pid, process.pid, &process.name),
                rss_bytes: memory.rss_bytes,
                pss_bytes: memory.pss_bytes,
                private_bytes: memory.private_bytes,
                shared_bytes: memory.shared_bytes,
                swap_bytes: memory.swap_bytes,
            }),
            Ok(None) => {
                skipped = skipped.saturating_add(1);
            }
            Err(_) => {
                skipped = skipped.saturating_add(1);
                memory_read_errors = memory_read_errors.saturating_add(1);
            }
        }
    }

    rows.sort_by(|left, right| right.rss_bytes.cmp(&left.rss_bytes));
    let totals = summarize_memory_totals(&rows);

    if rows.is_empty() {
        notes.push("No matching app processes were readable at snapshot time.".to_string());
    }
    if skipped > 0 {
        notes.push(format!(
            "Skipped {skipped} process{} while collecting memory metrics.",
            if skipped == 1 { "" } else { "es" }
        ));
    }
    if status_read_errors > 0 {
        notes.push(format!(
            "Encountered {status_read_errors} process status read error{}.",
            if status_read_errors == 1 { "" } else { "s" }
        ));
    }
    if memory_read_errors > 0 {
        notes.push(format!(
            "Encountered {memory_read_errors} process memory read error{}.",
            if memory_read_errors == 1 { "" } else { "s" }
        ));
    }
    if rows.iter().all(|row| row.pss_bytes.is_none()) {
        notes.push("PSS metrics were unavailable for this snapshot.".to_string());
    }
    if rows
        .iter()
        .all(|row| row.private_bytes.is_none() || row.shared_bytes.is_none())
    {
        notes.push("Private/shared split was unavailable for one or more processes.".to_string());
    }

    Ok(MemoryDiagnosticsSnapshot {
        captured_at: now_rfc3339()?,
        platform: "linux".to_string(),
        processes: rows,
        totals,
        notes,
    })
}

#[cfg(target_os = "linux")]
fn collect_linux_process_tree(
    root_pid: u32,
) -> Result<(Vec<ProcessIdentity>, usize), InternalError> {
    let entries = fs::read_dir("/proc")?;
    let mut all = HashMap::<u32, ProcessIdentity>::new();
    let mut status_read_errors = 0usize;

    for entry_result in entries {
        let entry = match entry_result {
            Ok(entry) => entry,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => continue,
            Err(error) => return Err(error.into()),
        };

        let Some(pid_text) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let Ok(pid) = pid_text.parse::<u32>() else {
            continue;
        };

        match read_linux_process_status(pid) {
            Ok(Some(status)) => {
                all.insert(pid, status);
            }
            Ok(None) => {}
            Err(_) => {
                status_read_errors = status_read_errors.saturating_add(1);
            }
        }
    }

    all.entry(root_pid).or_insert_with(|| ProcessIdentity {
        pid: root_pid,
        parent_pid: None,
        name: current_process_name(),
        vm_rss_bytes: None,
    });

    let mut children = HashMap::<u32, Vec<u32>>::new();
    for process in all.values() {
        if let Some(parent_pid) = process.parent_pid {
            children.entry(parent_pid).or_default().push(process.pid);
        }
    }

    let mut ordered = Vec::<ProcessIdentity>::new();
    let mut visited = HashSet::<u32>::new();
    let mut queue = VecDeque::<u32>::from([root_pid]);

    while let Some(pid) = queue.pop_front() {
        if !visited.insert(pid) {
            continue;
        }

        let Some(process) = all.get(&pid) else {
            continue;
        };
        ordered.push(process.clone());

        if let Some(child_ids) = children.get(&pid) {
            for child_pid in child_ids {
                queue.push_back(*child_pid);
            }
        }
    }

    Ok((ordered, status_read_errors))
}

#[cfg(target_os = "linux")]
fn read_linux_process_status(pid: u32) -> Result<Option<ProcessIdentity>, InternalError> {
    let status_path = PathBuf::from("/proc").join(pid.to_string()).join("status");
    let content = match fs::read_to_string(status_path) {
        Ok(content) => content,
        Err(error)
            if error.kind() == std::io::ErrorKind::NotFound
                || error.kind() == std::io::ErrorKind::PermissionDenied =>
        {
            return Ok(None);
        }
        Err(error) => return Err(error.into()),
    };

    let mut name = None::<String>;
    let mut parent_pid = None::<u32>;
    let mut vm_rss_bytes = None::<u64>;

    for line in content.lines() {
        if let Some(value) = line.strip_prefix("Name:") {
            name = Some(value.trim().to_string());
            continue;
        }
        if let Some(value) = line.strip_prefix("PPid:") {
            parent_pid = parse_status_u32(value);
            continue;
        }
        if let Some(value) = line.strip_prefix("VmRSS:") {
            vm_rss_bytes = parse_status_kib_bytes(value);
        }
    }

    let Some(name) = name else {
        return Ok(None);
    };

    Ok(Some(ProcessIdentity {
        pid,
        parent_pid,
        name,
        vm_rss_bytes,
    }))
}

#[cfg(target_os = "linux")]
fn read_linux_process_memory_stats(
    pid: u32,
    fallback_rss_bytes: Option<u64>,
) -> Result<Option<ProcessMemoryStats>, InternalError> {
    let smaps_rollup = PathBuf::from("/proc")
        .join(pid.to_string())
        .join("smaps_rollup");
    let content = match fs::read_to_string(&smaps_rollup) {
        Ok(content) => Some(content),
        Err(error)
            if error.kind() == std::io::ErrorKind::NotFound
                || error.kind() == std::io::ErrorKind::PermissionDenied =>
        {
            None
        }
        Err(error) => return Err(error.into()),
    };

    if let Some(content) = content {
        let mut rss_bytes = None::<u64>;
        let mut pss_bytes = None::<u64>;
        let mut private_clean_bytes = None::<u64>;
        let mut private_dirty_bytes = None::<u64>;
        let mut shared_clean_bytes = None::<u64>;
        let mut shared_dirty_bytes = None::<u64>;
        let mut swap_bytes = None::<u64>;

        for line in content.lines() {
            rss_bytes = rss_bytes.or_else(|| parse_kib_field(line, "Rss:"));
            pss_bytes = pss_bytes.or_else(|| parse_kib_field(line, "Pss:"));
            private_clean_bytes =
                private_clean_bytes.or_else(|| parse_kib_field(line, "Private_Clean:"));
            private_dirty_bytes =
                private_dirty_bytes.or_else(|| parse_kib_field(line, "Private_Dirty:"));
            shared_clean_bytes =
                shared_clean_bytes.or_else(|| parse_kib_field(line, "Shared_Clean:"));
            shared_dirty_bytes =
                shared_dirty_bytes.or_else(|| parse_kib_field(line, "Shared_Dirty:"));
            swap_bytes = swap_bytes.or_else(|| parse_kib_field(line, "Swap:"));
        }

        let private_bytes = match (private_clean_bytes, private_dirty_bytes) {
            (None, None) => None,
            (clean, dirty) => Some(clean.unwrap_or(0).saturating_add(dirty.unwrap_or(0))),
        };
        let shared_bytes = match (shared_clean_bytes, shared_dirty_bytes) {
            (None, None) => None,
            (clean, dirty) => Some(clean.unwrap_or(0).saturating_add(dirty.unwrap_or(0))),
        };
        let rss_bytes = rss_bytes.or(fallback_rss_bytes);

        if let Some(rss_bytes) = rss_bytes {
            return Ok(Some(ProcessMemoryStats {
                rss_bytes,
                pss_bytes,
                private_bytes,
                shared_bytes,
                swap_bytes,
            }));
        }
    }

    if let Some(rss_bytes) = fallback_rss_bytes {
        return Ok(Some(ProcessMemoryStats {
            rss_bytes,
            pss_bytes: None,
            private_bytes: None,
            shared_bytes: None,
            swap_bytes: None,
        }));
    }

    Ok(None)
}

#[cfg(target_os = "linux")]
fn parse_status_u32(value: &str) -> Option<u32> {
    value.split_whitespace().next()?.parse::<u32>().ok()
}

#[cfg(target_os = "linux")]
fn parse_status_kib_bytes(value: &str) -> Option<u64> {
    let kib = value.split_whitespace().next()?.parse::<u64>().ok()?;
    Some(kib.saturating_mul(1024))
}

#[cfg(target_os = "linux")]
fn parse_kib_field(line: &str, key: &str) -> Option<u64> {
    let value = line.strip_prefix(key)?.trim().split_whitespace().next()?;
    let kib = value.parse::<u64>().ok()?;
    Some(kib.saturating_mul(1024))
}

#[cfg(target_os = "windows")]
fn get_memory_diagnostics_windows() -> Result<MemoryDiagnosticsSnapshot, InternalError> {
    let root_pid = std::process::id();
    let mut notes = Vec::<String>::new();

    let windows_processes = match collect_windows_process_tree(root_pid) {
        Ok(processes) => processes,
        Err(error) => {
            notes.push(format!(
                "Falling back to limited Windows snapshot: {}",
                error
            ));
            Vec::new()
        }
    };

    let mut rows = windows_processes
        .into_iter()
        .map(|process| MemoryDiagnosticsProcess {
            pid: process.pid,
            parent_pid: process.parent_pid,
            name: process.name.clone(),
            role: classify_memory_process_role(root_pid, process.pid, &process.name),
            rss_bytes: process.working_set_bytes,
            pss_bytes: None,
            private_bytes: None,
            shared_bytes: None,
            swap_bytes: None,
        })
        .collect::<Vec<_>>();

    if rows.is_empty() {
        rows.push(MemoryDiagnosticsProcess {
            pid: root_pid,
            parent_pid: None,
            name: current_process_name(),
            role: "appMain".to_string(),
            rss_bytes: 0,
            pss_bytes: None,
            private_bytes: None,
            shared_bytes: None,
            swap_bytes: None,
        });
        notes.push(
            "No process tree data was available; showing a placeholder for the app process."
                .to_string(),
        );
    }

    notes.push(
        "Windows currently reports RSS/Working Set only; PSS/private/shared/swap are unavailable."
            .to_string(),
    );

    rows.sort_by(|left, right| right.rss_bytes.cmp(&left.rss_bytes));
    let totals = summarize_memory_totals(&rows);

    Ok(MemoryDiagnosticsSnapshot {
        captured_at: now_rfc3339()?,
        platform: "windows".to_string(),
        processes: rows,
        totals,
        notes,
    })
}

#[cfg(target_os = "windows")]
fn collect_windows_process_tree(
    root_pid: u32,
) -> Result<Vec<WindowsProcessIdentity>, InternalError> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-CimInstance Win32_Process | Select-Object ProcessId,ParentProcessId,Name,WorkingSetSize | ConvertTo-Json -Compress",
        ])
        .output()
        .map_err(|error| {
            InternalError::with_detail(
                "MEMORY_DIAGNOSTICS_FAILED",
                "Could not run PowerShell process query for memory diagnostics.",
                error.to_string(),
            )
        })?;

    if !output.status.success() {
        return Err(InternalError::with_detail(
            "MEMORY_DIAGNOSTICS_FAILED",
            "PowerShell process query failed for memory diagnostics.",
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return Ok(Vec::new());
    }

    let json = serde_json::from_str::<serde_json::Value>(&stdout)?;
    let mut all = HashMap::<u32, WindowsProcessIdentity>::new();
    for process in parse_windows_process_rows(&json) {
        all.insert(process.pid, process);
    }

    if all.is_empty() {
        return Ok(Vec::new());
    }

    let mut children = HashMap::<u32, Vec<u32>>::new();
    for process in all.values() {
        if let Some(parent_pid) = process.parent_pid {
            children.entry(parent_pid).or_default().push(process.pid);
        }
    }

    let mut ordered = Vec::<WindowsProcessIdentity>::new();
    let mut visited = HashSet::<u32>::new();
    let mut queue = VecDeque::<u32>::from([root_pid]);

    while let Some(pid) = queue.pop_front() {
        if !visited.insert(pid) {
            continue;
        }

        let Some(process) = all.get(&pid) else {
            continue;
        };
        ordered.push(process.clone());

        if let Some(child_ids) = children.get(&pid) {
            for child_pid in child_ids {
                queue.push_back(*child_pid);
            }
        }
    }

    Ok(ordered)
}

#[cfg(target_os = "windows")]
fn parse_windows_process_rows(json: &serde_json::Value) -> Vec<WindowsProcessIdentity> {
    match json {
        serde_json::Value::Array(items) => items
            .iter()
            .filter_map(parse_windows_process_row)
            .collect::<Vec<_>>(),
        serde_json::Value::Object(_) => parse_windows_process_row(json).into_iter().collect(),
        _ => Vec::new(),
    }
}

#[cfg(target_os = "windows")]
fn parse_windows_process_row(value: &serde_json::Value) -> Option<WindowsProcessIdentity> {
    let pid = parse_json_u32(value.get("ProcessId")?)?;
    let parent_pid = value.get("ParentProcessId").and_then(parse_json_u32);
    let name = value
        .get("Name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let working_set_bytes = value
        .get("WorkingSetSize")
        .and_then(parse_json_u64)
        .unwrap_or(0);

    Some(WindowsProcessIdentity {
        pid,
        parent_pid,
        name,
        working_set_bytes,
    })
}

#[cfg(target_os = "windows")]
fn parse_json_u32(value: &serde_json::Value) -> Option<u32> {
    if let Some(number) = value.as_u64() {
        return u32::try_from(number).ok();
    }

    value
        .as_str()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .and_then(|number| u32::try_from(number).ok())
}

#[cfg(target_os = "windows")]
fn parse_json_u64(value: &serde_json::Value) -> Option<u64> {
    if let Some(number) = value.as_u64() {
        return Some(number);
    }

    value
        .as_str()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
}

fn current_process_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "49modman".to_string())
}

fn classify_memory_process_role(root_pid: u32, pid: u32, name: &str) -> String {
    if pid == root_pid {
        return "appMain".to_string();
    }

    let lower = name.to_ascii_lowercase();
    if lower.contains("webkit") {
        return "webview".to_string();
    }
    if lower.contains("network") {
        return "network".to_string();
    }

    "appChild".to_string()
}

fn summarize_memory_totals(rows: &[MemoryDiagnosticsProcess]) -> MemoryDiagnosticsTotals {
    MemoryDiagnosticsTotals {
        rss_bytes: rows
            .iter()
            .fold(0_u64, |sum, row| sum.saturating_add(row.rss_bytes)),
        pss_bytes: sum_optional_bytes(rows.iter().map(|row| row.pss_bytes)),
        private_bytes: sum_optional_bytes(rows.iter().map(|row| row.private_bytes)),
        shared_bytes: sum_optional_bytes(rows.iter().map(|row| row.shared_bytes)),
        swap_bytes: sum_optional_bytes(rows.iter().map(|row| row.swap_bytes)),
    }
}

fn sum_optional_bytes(values: impl Iterator<Item = Option<u64>>) -> Option<u64> {
    let mut total = 0_u64;
    let mut had_any = false;

    for value in values.flatten() {
        had_any = true;
        total = total.saturating_add(value);
    }

    had_any.then_some(total)
}

fn trim_allocator_memory() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    unsafe {
        // Best-effort hint to glibc allocator to release free heap pages back to the OS.
        let _ = malloc_trim(0);
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
unsafe extern "C" {
    fn malloc_trim(pad: usize) -> i32;
}

fn is_tracked_game_pid_running(pid: u32) -> Result<bool, InternalError> {
    #[cfg(target_os = "windows")]
    {
        return is_tracked_game_pid_running_windows(pid);
    }

    #[cfg(target_os = "linux")]
    {
        return is_tracked_game_pid_running_linux(pid);
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = pid;
        Ok(false)
    }
}

fn is_game_process_running() -> Result<bool, InternalError> {
    #[cfg(target_os = "windows")]
    {
        return is_game_process_running_windows();
    }

    #[cfg(target_os = "linux")]
    {
        return is_game_process_running_linux();
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        Ok(false)
    }
}

#[cfg(target_os = "windows")]
fn is_tracked_game_pid_running_windows(pid: u32) -> Result<bool, InternalError> {
    let output = match Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
    {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };

    if !output.status.success() {
        return Ok(false);
    }

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let fields = line
            .split(',')
            .map(|value| value.trim().trim_matches('"'))
            .collect::<Vec<_>>();

        if fields.len() < 2 {
            continue;
        }

        if fields[0].eq_ignore_ascii_case(GAME_EXECUTABLE_NAME)
            && fields[1].parse::<u32>().ok() == Some(pid)
        {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(target_os = "linux")]
fn is_tracked_game_pid_running_linux(pid: u32) -> Result<bool, InternalError> {
    let proc_dir = PathBuf::from("/proc").join(pid.to_string());
    if !proc_dir.exists() {
        return Ok(false);
    }

    let cmdline_matches = read_process_cmdline_contains_game_executable(&proc_dir.join("cmdline"))?;
    if cmdline_matches {
        return Ok(true);
    }

    read_process_comm_matches_game_executable(&proc_dir.join("comm"))
}

#[cfg(target_os = "windows")]
fn is_game_process_running_windows() -> Result<bool, InternalError> {
    let output = match Command::new("tasklist")
        .args([
            "/FI",
            &format!("IMAGENAME eq {GAME_EXECUTABLE_NAME}"),
            "/FO",
            "CSV",
            "/NH",
        ])
        .output()
    {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };

    if !output.status.success() {
        return Ok(false);
    }

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let image_name = line
            .split(',')
            .next()
            .map(|value| value.trim().trim_matches('"'))
            .unwrap_or_default();
        if image_name.eq_ignore_ascii_case(GAME_EXECUTABLE_NAME) {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(target_os = "linux")]
fn is_game_process_running_linux() -> Result<bool, InternalError> {
    let entries = match fs::read_dir("/proc") {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };

    for entry_result in entries {
        let entry = match entry_result {
            Ok(entry) => entry,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => continue,
            Err(error) => return Err(error.into()),
        };

        let file_name = entry.file_name();
        let Some(pid_text) = file_name.to_str() else {
            continue;
        };
        if pid_text.parse::<u32>().is_err() {
            continue;
        }

        let process_dir = entry.path();
        if read_process_cmdline_contains_game_executable(&process_dir.join("cmdline"))? {
            return Ok(true);
        }
        if read_process_comm_matches_game_executable(&process_dir.join("comm"))? {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(target_os = "linux")]
fn read_process_cmdline_contains_game_executable(path: &Path) -> Result<bool, InternalError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error)
            if error.kind() == std::io::ErrorKind::NotFound
                || error.kind() == std::io::ErrorKind::PermissionDenied =>
        {
            return Ok(false);
        }
        Err(error) => return Err(error.into()),
    };

    if bytes.is_empty() {
        return Ok(false);
    }

    for arg in bytes.split(|byte| *byte == 0) {
        if arg.is_empty() {
            continue;
        }

        let token = String::from_utf8_lossy(arg);
        if is_game_executable_arg(token.as_ref()) {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(target_os = "linux")]
fn read_process_comm_matches_game_executable(path: &Path) -> Result<bool, InternalError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error)
            if error.kind() == std::io::ErrorKind::NotFound
                || error.kind() == std::io::ErrorKind::PermissionDenied =>
        {
            return Ok(false);
        }
        Err(error) => return Err(error.into()),
    };

    if bytes.is_empty() {
        return Ok(false);
    }

    let name = String::from_utf8_lossy(&bytes).trim().to_ascii_lowercase();
    Ok(name == "lethal company.exe" || name == "lethal company.")
}

fn is_game_executable_arg(value: &str) -> bool {
    let normalized = value
        .trim()
        .trim_matches('"')
        .replace('\\', "/")
        .to_ascii_lowercase();

    normalized == "lethal company.exe"
        || normalized == "./lethal company.exe"
        || normalized.ends_with("/lethal company.exe")
}

fn launch_game_process(
    launch_mode: &str,
    game_root: &Path,
    diagnostics_dir: &Path,
    proton_runtime: Option<&ProtonRuntime>,
    steam_compat_data_path: Option<PathBuf>,
) -> Result<u32, InternalError> {
    if launch_mode != "direct" && launch_mode != "steam" {
        return Err(InternalError::app(
            "LAUNCH_MODE_INVALID",
            format!("Unsupported launch mode: {launch_mode}"),
        ));
    }

    if launch_mode == "direct" && cfg!(target_os = "windows") {
        launch_direct_windows(game_root, diagnostics_dir)
    } else if launch_mode == "direct" && cfg!(target_os = "linux") {
        let runtime = proton_runtime.ok_or_else(|| {
            InternalError::app(
                "PROTON_RUNTIME_REQUIRED",
                "Linux direct launch requires a valid Proton runtime selection.",
            )
        })?;

        launch_direct_linux(
            game_root,
            diagnostics_dir,
            runtime,
            steam_compat_data_path.as_deref(),
        )
    } else if launch_mode == "direct" {
        Err(InternalError::app(
            "LAUNCH_MODE_UNSUPPORTED_PLATFORM",
            "Direct launch is currently supported on Windows and Linux only.",
        ))
    } else {
        launch_steam(diagnostics_dir)
    }
}

fn launch_direct_windows(game_root: &Path, diagnostics_dir: &Path) -> Result<u32, InternalError> {
    let executable = game_root.join(GAME_EXECUTABLE_NAME);
    if !executable.is_file() {
        return Err(InternalError::with_detail(
            "LAUNCH_FAILED",
            "Direct launch executable is missing.",
            path_to_string(&executable),
        ));
    }

    let stdout = fs::File::create(diagnostics_dir.join("stdout.log"))?;
    let stderr = fs::File::create(diagnostics_dir.join("stderr.log"))?;

    let child = Command::new(&executable)
        .current_dir(game_root)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .map_err(|error| {
            InternalError::with_detail(
                "LAUNCH_FAILED",
                "Failed to start direct game process.",
                error.to_string(),
            )
        })?;

    Ok(child.id())
}

fn launch_direct_linux(
    game_root: &Path,
    diagnostics_dir: &Path,
    runtime: &ProtonRuntime,
    steam_compat_data_path: Option<&Path>,
) -> Result<u32, InternalError> {
    let executable = game_root.join(GAME_EXECUTABLE_NAME);
    if !executable.is_file() {
        return Err(InternalError::with_detail(
            "LAUNCH_FAILED",
            "Direct launch executable is missing.",
            path_to_string(&executable),
        ));
    }

    let proton_path = PathBuf::from(runtime.path.clone());
    if !proton_path.is_file() {
        return Err(InternalError::with_detail(
            "PROTON_RUNTIME_INVALID",
            "Selected Proton runtime executable does not exist.",
            runtime.path.clone(),
        ));
    }

    let stdout = fs::File::create(diagnostics_dir.join("stdout.log"))?;
    let stderr = fs::File::create(diagnostics_dir.join("stderr.log"))?;
    let mut command = Command::new(&proton_path);
    command
        .arg("run")
        .arg(&executable)
        .current_dir(game_root)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    // BepInEx on Proton requires native winhttp override for Doorstop injection.
    command.env("WINEDLLOVERRIDES", "winhttp=n,b");

    let steam_client_path = resolve_steam_client_install_path(&proton_path).ok_or_else(|| {
        InternalError::app(
            "PROTON_CLIENT_PATH_REQUIRED",
            "Linux direct launch requires a resolvable Steam client install path.",
        )
    })?;
    command.env(
        "STEAM_COMPAT_CLIENT_INSTALL_PATH",
        path_to_string(&steam_client_path),
    );

    if let Some(compat_path) = steam_compat_data_path {
        fs::create_dir_all(compat_path)?;
        command.env("STEAM_COMPAT_DATA_PATH", compat_path);
    }

    let child = command.spawn().map_err(|error| {
        InternalError::with_detail(
            "LAUNCH_FAILED",
            "Failed to start Linux direct launch via Proton.",
            error.to_string(),
        )
    })?;

    Ok(child.id())
}

fn launch_steam(diagnostics_dir: &Path) -> Result<u32, InternalError> {
    fs::write(diagnostics_dir.join("stdout.log"), b"")?;
    fs::write(diagnostics_dir.join("stderr.log"), b"")?;

    let mut last_error = None;
    let candidates = if cfg!(target_os = "windows") {
        vec!["steam", "steam.exe"]
    } else {
        vec!["steam", "steam.sh"]
    };

    for executable in candidates {
        let mut command = Command::new(executable);
        command.arg("-applaunch").arg(STEAM_APP_ID);
        if cfg!(target_os = "linux") {
            // Best-effort for cases where Steam is started by this process.
            command.env("WINEDLLOVERRIDES", "winhttp=n,b");
        }

        match command.spawn() {
            Ok(child) => return Ok(child.id()),
            Err(error) => {
                last_error = Some(error.to_string());
            }
        }
    }

    Err(InternalError::with_detail(
        "LAUNCH_FAILED",
        "Failed to start Steam launch process.",
        last_error.unwrap_or_else(|| "Steam executable not found.".to_string()),
    ))
}

fn validate_linux_steam_modded_launch_options() -> Result<(), InternalError> {
    let records = discover_steam_launch_options_records(STEAM_APP_ID)?;
    let expected = r#"WINEDLLOVERRIDES="winhttp=n,b" %command%"#;

    if records.is_empty() {
        return Err(InternalError::app(
            "STEAM_LAUNCH_OPTIONS_INVALID",
            format_steam_launch_options_user_message(expected, None),
        ));
    }

    for record in &records {
        let normalized = record.value.trim().to_ascii_lowercase();
        let has_command_placeholder = normalized.contains("%command%");
        let has_winhttp_override = normalized.contains("winhttp=n,b");
        if has_command_placeholder && has_winhttp_override {
            return Ok(());
        }
    }

    let current_values = records
        .iter()
        .map(|record| {
            let value = record.value.trim();
            if value.is_empty() {
                "<empty>".to_string()
            } else {
                value.to_string()
            }
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(3)
        .collect::<Vec<_>>();

    Err(InternalError::app(
        "STEAM_LAUNCH_OPTIONS_INVALID",
        format_steam_launch_options_user_message(expected, Some(current_values)),
    ))
}

fn format_steam_launch_options_user_message(
    expected: &str,
    current_values: Option<Vec<String>>,
) -> String {
    let mut message = String::from(
        "Steam launch options are preventing modded Steam launch.\n\
Open Steam -> Lethal Company -> Properties -> Launch Options, then set:\n\
",
    );
    message.push_str(expected);

    if let Some(values) = current_values {
        if !values.is_empty() {
            message.push_str("\n\nCurrent Launch Options value(s):");
            for value in values {
                message.push_str("\n- ");
                message.push_str(&value);
            }
        }
    }

    message.push_str("\n\nAfter saving, launch modded again.");
    message
}

fn discover_steam_launch_options_records(
    app_id: &str,
) -> Result<Vec<SteamLaunchOptionsRecord>, InternalError> {
    let mut records = Vec::<SteamLaunchOptionsRecord>::new();
    for config_path in collect_steam_localconfig_paths() {
        let content = fs::read_to_string(&config_path)?;
        if let Some(value) = parse_steam_launch_options_from_vdf_content(&content, app_id) {
            records.push(SteamLaunchOptionsRecord { value });
        }
    }
    Ok(records)
}

fn collect_steam_localconfig_paths() -> Vec<PathBuf> {
    let mut paths = BTreeSet::<PathBuf>::new();

    for steam_root in collect_steam_root_candidates() {
        let userdata_dir = steam_root.join("userdata");
        if !userdata_dir.is_dir() {
            continue;
        }

        let Ok(user_entries) = fs::read_dir(&userdata_dir) else {
            continue;
        };

        for user_entry in user_entries.flatten() {
            let user_path = user_entry.path();
            if !user_path.is_dir() {
                continue;
            }

            let Some(user_dir_name) = user_path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !user_dir_name.chars().all(|ch| ch.is_ascii_digit()) {
                continue;
            }

            let config_path = user_path.join("config").join("localconfig.vdf");
            if config_path.is_file() {
                paths.insert(config_path);
            }
        }
    }

    paths.into_iter().collect()
}

fn parse_steam_launch_options_from_vdf_content(content: &str, app_id: &str) -> Option<String> {
    let mut object_stack = Vec::<String>::new();
    let mut pending_key: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        let tokens = extract_quoted_tokens(trimmed);
        if tokens.len() >= 2 {
            if stack_ends_with_case_insensitive(&object_stack, &["apps", app_id])
                && tokens[0].eq_ignore_ascii_case("LaunchOptions")
            {
                return Some(tokens[1].clone());
            }
            pending_key = None;
        } else if tokens.len() == 1 {
            pending_key = Some(tokens[0].clone());
        }

        let open_count = trimmed.chars().filter(|ch| *ch == '{').count();
        for _ in 0..open_count {
            if let Some(key) = pending_key.take() {
                object_stack.push(key);
            } else {
                object_stack.push(String::new());
            }
        }

        let close_count = trimmed.chars().filter(|ch| *ch == '}').count();
        for _ in 0..close_count {
            let _ = object_stack.pop();
        }
        if close_count > 0 {
            pending_key = None;
        }
    }

    None
}

fn stack_ends_with_case_insensitive(stack: &[String], expected_suffix: &[&str]) -> bool {
    if stack.len() < expected_suffix.len() {
        return false;
    }

    let start = stack.len() - expected_suffix.len();
    stack[start..]
        .iter()
        .zip(expected_suffix.iter())
        .all(|(actual, expected)| actual.eq_ignore_ascii_case(expected))
}

fn profile_root_dir(state: &AppState, profile_id: &str) -> PathBuf {
    state.profiles_dir.join(profile_id)
}

fn resolve_profile_install_dir(
    state: &AppState,
    profile_id: &str,
    install_dir: &str,
) -> Option<PathBuf> {
    let mut path = profile_root_dir(state, profile_id);

    for segment in install_dir.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return None;
        }
        path = path.join(segment);
    }

    Some(path)
}

fn collect_relative_files(root: &Path) -> Result<Vec<PathBuf>, InternalError> {
    let mut files = Vec::<PathBuf>::new();
    collect_relative_files_recursive(root, root, &mut files)?;
    files.sort_by(|left, right| {
        left.to_string_lossy()
            .to_ascii_lowercase()
            .cmp(&right.to_string_lossy().to_ascii_lowercase())
            .then_with(|| left.to_string_lossy().cmp(&right.to_string_lossy()))
    });
    Ok(files)
}

fn collect_relative_files_recursive(
    root: &Path,
    current: &Path,
    output: &mut Vec<PathBuf>,
) -> Result<(), InternalError> {
    let mut children = fs::read_dir(current)?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|entry| entry.path())
        .collect::<Vec<_>>();

    children.sort_by(|left, right| {
        left.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_ascii_lowercase()
            .cmp(
                &right
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_ascii_lowercase(),
            )
            .then_with(|| left.to_string_lossy().cmp(&right.to_string_lossy()))
    });

    for child in children {
        if child.is_dir() {
            collect_relative_files_recursive(root, &child, output)?;
        } else if child.is_file() {
            let relative = child
                .strip_prefix(root)
                .map_err(|_| {
                    InternalError::app(
                        "RUNTIME_STAGE_BUILD_FAILED",
                        format!(
                            "Failed to resolve file path relative to install root: {}",
                            path_to_string(&child)
                        ),
                    )
                })?
                .to_path_buf();

            output.push(relative);
        }
    }

    Ok(())
}

fn should_skip_stage_path(relative_path: &Path) -> bool {
    if relative_path.components().count() != 1 {
        return false;
    }

    let file_name = relative_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    matches!(
        file_name.as_str(),
        "icon.png"
            | "manifest.json"
            | "readme.md"
            | "readme.txt"
            | "changelog.md"
            | "changelog.txt"
    )
}

fn normalize_stage_relative_path(relative_path: &Path) -> Option<PathBuf> {
    let mut components = relative_path
        .components()
        .map(|component| {
            component
                .as_os_str()
                .to_str()
                .map(|value| value.to_string())
        })
        .collect::<Option<Vec<_>>>()?;

    if components.is_empty() {
        return None;
    }

    components = strip_mod_package_wrapper_components(components);
    if components.is_empty() {
        return None;
    }

    if should_place_under_plugins_by_extension(&components)
        && !is_runtime_anchor_component(&components[0].to_ascii_lowercase())
    {
        let mut remapped = Vec::<String>::with_capacity(components.len() + 2);
        remapped.push("BepInEx".to_string());
        remapped.push("plugins".to_string());
        remapped.extend(components);
        components = remapped;
    }

    if let Some(first_component) = components.first() {
        let first = first_component.to_ascii_lowercase();
        if matches!(first.as_str(), "plugins" | "patchers" | "config" | "core") {
            let mut remapped = Vec::<String>::with_capacity(components.len() + 1);
            remapped.push("BepInEx".to_string());
            remapped.extend(components);
            components = remapped;
        }
    }

    Some(components_to_relative_path(&components))
}

fn strip_mod_package_wrapper_components(mut components: Vec<String>) -> Vec<String> {
    loop {
        if components.len() < 2 {
            return components;
        }

        let first = components[0].to_ascii_lowercase();
        let second = components[1].to_ascii_lowercase();
        let first_is_anchor = is_runtime_anchor_component(&first);
        let second_is_anchor = is_runtime_anchor_component(&second);
        let first_is_known_wrapper = is_known_package_wrapper_component(&first);

        // A Thunderstore package may wrap payload files in a package-named root
        // directory (for example `BepInExPack/`). Remove wrapper levels so staged
        // output matches game-root layout expectations.
        if first_is_known_wrapper && second_is_anchor {
            components.remove(0);
            continue;
        }

        if !first_is_anchor && second_is_anchor {
            components.remove(0);
            continue;
        }

        return components;
    }
}

fn is_known_package_wrapper_component(component: &str) -> bool {
    component == "bepinexpack"
        || component.starts_with("bepinexpack-")
        || component.starts_with("bepinex-bepinexpack")
}

fn is_runtime_anchor_component(component: &str) -> bool {
    matches!(
        component,
        "bepinex"
            | "plugins"
            | "patchers"
            | "config"
            | "core"
            | "winhttp.dll"
            | "doorstop_config.ini"
            | "run_bepinex.sh"
            | "start_game_bepinex.sh"
            | "libdoorstop_x64.so"
            | "libdoorstop_x86.so"
            | "doorstop_libs"
            | "manifest.json"
            | "icon.png"
            | "readme.md"
            | "readme.txt"
            | "changelog.md"
            | "changelog.txt"
    )
}

fn should_place_under_plugins_by_extension(components: &[String]) -> bool {
    let Some(file_name) = components.last() else {
        return false;
    };

    file_name.to_ascii_lowercase().ends_with(".lethalbundle")
}

fn components_to_relative_path(components: &[String]) -> PathBuf {
    let mut path = PathBuf::new();
    for component in components {
        path.push(component);
    }
    path
}

fn parse_dependency_entries(value: &str) -> Vec<String> {
    match serde_json::from_str::<Vec<String>>(value) {
        Ok(entries) => entries,
        Err(_) => {
            let trimmed = value.trim();
            if trimmed.is_empty() || trimmed == "[]" || trimmed == "null" {
                Vec::new()
            } else {
                vec![trimmed.to_string()]
            }
        }
    }
}

fn dependency_raw_key(package_name: &str, version_number: &str) -> String {
    format!("{package_name}-{version_number}")
}

fn calculate_sha256(path: &Path) -> Result<String, InternalError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn resolve_proton_runtime_selection(
    connection: &Connection,
    requested_runtime_id: Option<&str>,
) -> Result<ProtonRuntime, InternalError> {
    let runtimes = discover_proton_runtimes()?;
    let valid_runtimes = runtimes
        .iter()
        .filter(|runtime| runtime.is_valid)
        .cloned()
        .collect::<Vec<_>>();

    if let Some(requested_runtime_id) = requested_runtime_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let selected = valid_runtimes
            .iter()
            .find(|runtime| runtime.id == requested_runtime_id)
            .cloned()
            .ok_or_else(|| {
                InternalError::app(
                    "PROTON_RUNTIME_INVALID",
                    "Requested Proton runtime is missing or invalid for Linux direct launch.",
                )
            })?;

        return Ok(selected);
    }

    let preferred_runtime_id = get_setting(connection, "launch.preferred_proton_runtime_id")?
        .and_then(|value| serde_json::from_str::<String>(&value).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if let Some(preferred_runtime_id) = preferred_runtime_id {
        if let Some(selected) = valid_runtimes
            .iter()
            .find(|runtime| runtime.id == preferred_runtime_id)
            .cloned()
        {
            return Ok(selected);
        }
    }

    valid_runtimes.into_iter().next().ok_or_else(|| {
        InternalError::app(
            "PROTON_RUNTIME_REQUIRED",
            "Linux direct launch requires at least one valid Proton runtime.",
        )
    })
}

fn discover_proton_runtimes() -> Result<Vec<ProtonRuntime>, InternalError> {
    let steam_roots = collect_steam_root_candidates();
    let mut library_paths = BTreeSet::<PathBuf>::new();

    for root in &steam_roots {
        if !root.join("steamapps").is_dir() {
            continue;
        }

        library_paths.insert(root.clone());

        let library_vdf = root.join("steamapps").join("libraryfolders.vdf");
        for library in parse_libraryfolders(&library_vdf)? {
            library_paths.insert(library);
        }
    }

    let mut runtimes = BTreeMap::<String, ProtonRuntime>::new();

    for library in &library_paths {
        let common_dir = library.join("steamapps").join("common");
        if !common_dir.is_dir() {
            continue;
        }

        for entry in fs::read_dir(&common_dir)? {
            let entry = entry?;
            let runtime_dir = entry.path();
            if !runtime_dir.is_dir() {
                continue;
            }

            let display_name = entry.file_name().to_string_lossy().to_string();
            if !looks_like_proton_runtime_name(&display_name) {
                continue;
            }

            let proton_bin = runtime_dir.join("proton");
            let id = path_to_string(&proton_bin);
            runtimes.insert(
                id.clone(),
                ProtonRuntime {
                    id,
                    display_name,
                    path: path_to_string(&proton_bin),
                    source: "steam".to_string(),
                    is_valid: proton_bin.is_file(),
                },
            );
        }
    }

    for steam_root in &steam_roots {
        let compatibility_dir = steam_root.join("compatibilitytools.d");
        if !compatibility_dir.is_dir() {
            continue;
        }

        for entry in fs::read_dir(&compatibility_dir)? {
            let entry = entry?;
            let runtime_dir = entry.path();
            if !runtime_dir.is_dir() {
                continue;
            }

            let display_name = entry.file_name().to_string_lossy().to_string();
            let proton_bin = runtime_dir.join("proton");
            let id = path_to_string(&proton_bin);
            runtimes.insert(
                id.clone(),
                ProtonRuntime {
                    id,
                    display_name,
                    path: path_to_string(&proton_bin),
                    source: "custom".to_string(),
                    is_valid: proton_bin.is_file(),
                },
            );
        }
    }

    let mut values = runtimes.into_values().collect::<Vec<_>>();
    values.sort_by(|left, right| {
        left.display_name
            .to_ascii_lowercase()
            .cmp(&right.display_name.to_ascii_lowercase())
            .then_with(|| left.path.cmp(&right.path))
    });

    Ok(values)
}

fn looks_like_proton_runtime_name(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.starts_with("proton")
        || name.contains("ge-proton")
        || name.contains("proton-ge")
        || name.contains("luxtorpeda")
}

fn resolve_steam_compat_data_path(
    state: &AppState,
    profile_hint: Option<&str>,
) -> Result<PathBuf, InternalError> {
    let app_data = state.profiles_dir.parent().ok_or_else(|| {
        InternalError::app(
            "RESOURCE_LOAD_FAILED",
            "Failed to resolve app data root for Proton compat data path.",
        )
    })?;

    let profile_hint = profile_hint
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or("default");
    let compat_path = app_data
        .join("state")
        .join("proton-compat")
        .join(profile_hint);
    fs::create_dir_all(&compat_path)?;
    Ok(compat_path)
}

fn resolve_steam_client_install_path(proton_path: &Path) -> Option<PathBuf> {
    for ancestor in proton_path.ancestors() {
        if ancestor.join("steamapps").is_dir() {
            return Some(ancestor.to_path_buf());
        }
    }

    collect_steam_root_candidates()
        .into_iter()
        .find(|candidate| candidate.join("steamapps").is_dir())
}

fn collect_steam_root_candidates() -> Vec<PathBuf> {
    let mut candidates = BTreeSet::<PathBuf>::new();

    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        candidates.insert(home.join(".steam").join("steam"));
        candidates.insert(home.join(".local").join("share").join("Steam"));
        candidates.insert(
            home.join(".var")
                .join("app")
                .join("com.valvesoftware.Steam")
                .join(".steam")
                .join("steam"),
        );
        candidates.insert(
            home.join("Library")
                .join("Application Support")
                .join("Steam"),
        );
    }

    if let Some(program_files_x86) = std::env::var_os("PROGRAMFILES(X86)") {
        candidates.insert(PathBuf::from(program_files_x86).join("Steam"));
    }

    if let Some(program_files) = std::env::var_os("PROGRAMFILES") {
        candidates.insert(PathBuf::from(program_files).join("Steam"));
    }

    candidates.into_iter().collect()
}

fn parse_libraryfolders(path: &Path) -> Result<Vec<PathBuf>, InternalError> {
    if !path.is_file() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(path)?;
    let mut libraries = BTreeSet::<PathBuf>::new();

    for line in content.lines() {
        let tokens = extract_quoted_tokens(line);
        if tokens.len() < 2 {
            continue;
        }

        let first = tokens[0].trim();
        let second = tokens[1].trim();

        if first.eq_ignore_ascii_case("path") {
            if let Some(path) = normalize_vdf_path(second) {
                libraries.insert(path);
            }
            continue;
        }

        if first.chars().all(|ch| ch.is_ascii_digit()) && looks_like_path(second) {
            if let Some(path) = normalize_vdf_path(second) {
                libraries.insert(path);
            }
        }
    }

    Ok(libraries.into_iter().collect())
}

fn extract_quoted_tokens(line: &str) -> Vec<String> {
    let mut tokens = Vec::<String>::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escaped = false;

    for ch in line.chars() {
        if in_quotes {
            if escaped {
                current.push(ch);
                escaped = false;
                continue;
            }

            if ch == '\\' {
                escaped = true;
                continue;
            }

            if ch == '"' {
                tokens.push(current.clone());
                current.clear();
                in_quotes = false;
                continue;
            }

            current.push(ch);
        } else if ch == '"' {
            in_quotes = true;
        }
    }

    if in_quotes && escaped {
        current.push('\\');
    }

    tokens
}

fn looks_like_path(value: &str) -> bool {
    value.contains('/') || value.contains('\\') || value.contains(':')
}

fn normalize_vdf_path(value: &str) -> Option<PathBuf> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let unescaped = trimmed
        .replace("\\\\", "\\")
        .replace("\\/", "/")
        .replace("//", "/");

    normalize_candidate_path(Some(unescaped.as_str()))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use time::OffsetDateTime;

    use super::{
        collect_relative_files, extract_quoted_tokens, is_game_executable_arg, normalize_sha256,
        normalize_stage_relative_path, parse_dependency_entries,
        parse_steam_launch_options_from_vdf_content, should_skip_stage_path,
    };

    #[test]
    fn parse_dependency_entries_handles_json_and_fallback() {
        assert_eq!(
            parse_dependency_entries("[\"A-B-1.0.0\", \"C-D-2.0.0\"]"),
            vec!["A-B-1.0.0", "C-D-2.0.0"]
        );
        assert_eq!(parse_dependency_entries("[]"), Vec::<String>::new());
        assert_eq!(
            parse_dependency_entries("A-B-1.0.0"),
            vec!["A-B-1.0.0".to_string()]
        );
    }

    #[test]
    fn normalize_sha256_accepts_only_64_hex_chars() {
        let valid = "2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881";
        assert_eq!(normalize_sha256(valid), Some(valid.to_string()));
        assert_eq!(normalize_sha256("xyz"), None);
        assert_eq!(normalize_sha256("abc"), None);
    }

    #[test]
    fn extract_quoted_tokens_reads_vdf_lines() {
        let tokens = extract_quoted_tokens("\t\"path\"\t\"D:\\\\SteamLibrary\"");
        assert_eq!(tokens, vec!["path", "D:\\SteamLibrary"]);
    }

    #[test]
    fn extract_quoted_tokens_handles_escaped_quotes() {
        let tokens = extract_quoted_tokens(
            "\"LaunchOptions\"\t\"WINEDLLOVERRIDES=\\\"winhttp=n,b\\\" %command%\"",
        );
        assert_eq!(
            tokens,
            vec![
                "LaunchOptions".to_string(),
                "WINEDLLOVERRIDES=\"winhttp=n,b\" %command%".to_string()
            ]
        );
    }

    #[test]
    fn collect_relative_files_is_stable_and_sorted() {
        let root = temp_test_dir("collect-relative-files");
        fs::create_dir_all(root.join("B").join("sub")).unwrap();
        fs::create_dir_all(root.join("a")).unwrap();
        fs::write(root.join("B").join("sub").join("z.txt"), b"z").unwrap();
        fs::write(root.join("a").join("a.txt"), b"a").unwrap();
        fs::write(root.join("B").join("b.txt"), b"b").unwrap();

        let relative_files = collect_relative_files(&root).unwrap();
        let actual = relative_files
            .iter()
            .map(|entry| entry.to_string_lossy().replace('\\', "/"))
            .collect::<Vec<_>>();

        assert_eq!(actual, vec!["a/a.txt", "B/b.txt", "B/sub/z.txt"]);
    }

    #[test]
    fn skip_rules_only_apply_to_root_metadata() {
        assert!(should_skip_stage_path(&PathBuf::from("manifest.json")));
        assert!(should_skip_stage_path(&PathBuf::from("icon.png")));
        assert!(should_skip_stage_path(&PathBuf::from("README.md")));
        assert!(!should_skip_stage_path(&PathBuf::from(
            "BepInEx/plugins/README.md"
        )));
        assert!(!should_skip_stage_path(&PathBuf::from(
            "BepInEx/plugins/test.dll"
        )));
    }

    #[test]
    fn normalize_stage_relative_path_strips_wrapper_and_maps_plugins_dirs() {
        assert_eq!(
            normalize_stage_relative_path(&PathBuf::from("BepInExPack/winhttp.dll"))
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/"),
            "winhttp.dll"
        );

        assert_eq!(
            normalize_stage_relative_path(&PathBuf::from("BepInExPack/BepInEx/core/test.dll"))
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/"),
            "BepInEx/core/test.dll"
        );

        assert_eq!(
            normalize_stage_relative_path(&PathBuf::from("BepInExPack/plugins/test.dll"))
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/"),
            "BepInEx/plugins/test.dll"
        );

        assert_eq!(
            normalize_stage_relative_path(&PathBuf::from("ExampleMod/patchers/hook.dll"))
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/"),
            "BepInEx/patchers/hook.dll"
        );

        assert_eq!(
            normalize_stage_relative_path(&PathBuf::from("Maritopia/maritopia.lethalbundle"))
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/"),
            "BepInEx/plugins/Maritopia/maritopia.lethalbundle"
        );
    }

    #[test]
    fn parse_steam_launch_options_from_vdf_content_reads_app_launch_options() {
        let content = r#"
"UserLocalConfigStore"
{
    "Software"
    {
        "Valve"
        {
            "Steam"
            {
                "apps"
                {
                    "1966720"
                    {
                        "LaunchOptions"		"./Lethal Company.exe"
                    }
                }
            }
        }
    }
}
"#;

        assert_eq!(
            parse_steam_launch_options_from_vdf_content(content, "1966720")
                .as_deref()
                .unwrap_or_default(),
            "./Lethal Company.exe"
        );
        assert!(parse_steam_launch_options_from_vdf_content(content, "9999999").is_none());
    }

    #[test]
    fn game_executable_arg_matcher_requires_executable_path() {
        assert!(is_game_executable_arg("./Lethal Company.exe"));
        assert!(is_game_executable_arg(
            "Z:\\steamapps\\common\\Lethal Company\\Lethal Company.exe"
        ));
        assert!(!is_game_executable_arg(
            "/mnt/recovery/SteamLibrary/steamapps/common/Lethal Company"
        ));
        assert!(!is_game_executable_arg("dolphin"));
    }

    fn temp_test_dir(prefix: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "49modman-{}-{}",
            prefix,
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));

        if path.exists() {
            let _ = fs::remove_dir_all(&path);
        }
        fs::create_dir_all(&path).unwrap();
        path
    }
}
