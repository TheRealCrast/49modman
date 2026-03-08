use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use base64::Engine;
use rfd::FileDialog;
use rusqlite::{params, Connection, OptionalExtension};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

use crate::{
    app_state::AppState,
    db::{get_setting, now_rfc3339, reset_user_data, upsert_setting},
    error::InternalError,
    services::cache_service::{cached_archive_path, upsert_local_cached_archive},
};

const PROFILE_MANIFEST_SCHEMA_VERSION: u32 = 1;
const PROFILE_PACK_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSummaryDto {
    pub id: String,
    pub name: String,
    pub notes: String,
    pub game_path: String,
    pub last_played: Option<String>,
    pub launch_mode_default: String,
    pub installed_count: usize,
    pub enabled_count: usize,
    pub is_builtin_default: bool,
    pub profile_size_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDetailDto {
    pub id: String,
    pub name: String,
    pub notes: String,
    pub game_path: String,
    pub last_played: Option<String>,
    pub launch_mode_default: String,
    pub is_builtin_default: bool,
    pub installed_mods: Vec<ProfileInstalledModDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInstalledModDto {
    pub package_id: String,
    pub package_name: String,
    pub version_id: String,
    pub version_number: String,
    pub enabled: bool,
    pub source_kind: String,
    pub install_dir: String,
    pub installed_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_data_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProfileInput {
    pub name: String,
    pub notes: Option<String>,
    pub game_path: Option<String>,
    pub launch_mode_default: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileInput {
    pub profile_id: String,
    pub name: String,
    pub notes: Option<String>,
    pub game_path: Option<String>,
    pub launch_mode_default: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetInstalledModEnabledInput {
    pub profile_id: String,
    pub package_id: String,
    pub version_id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UninstallInstalledModInput {
    pub profile_id: String,
    pub package_id: String,
    pub version_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetUninstallDependantsInput {
    pub profile_id: String,
    pub package_id: String,
    pub version_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UninstallDependantDto {
    pub package_id: String,
    pub package_name: String,
    pub version_id: String,
    pub version_number: String,
    pub min_depth: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteProfileResult {
    pub deleted_id: String,
    pub next_active_profile_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesStorageSummaryDto {
    pub profile_count: usize,
    pub profiles_total_bytes: i64,
    pub active_profile_bytes: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProfilePackResult {
    pub cancelled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mod_count: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProfilePackInput {
    pub profile_id: String,
    #[serde(default)]
    pub embed_unavailable_payloads: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewExportProfilePackUnavailableModDto {
    pub package_id: String,
    pub package_name: String,
    pub version_id: String,
    pub version_number: String,
    pub unavailable_reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewExportProfilePackResult {
    pub profile_id: String,
    pub profile_name: String,
    pub mod_count: usize,
    #[serde(default)]
    pub unavailable_mods: Vec<PreviewExportProfilePackUnavailableModDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProfilePackResult {
    pub cancelled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    pub payload_mode: String,
    pub embedded_mod_count: usize,
    pub referenced_mod_count: usize,
    pub has_legacy_runtime_plugins_payload: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<ProfileDetailDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProfilePackPreviewModDto {
    pub package_id: String,
    pub package_name: String,
    pub version_id: String,
    pub version_number: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProfilePackPreviewResult {
    pub cancelled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_name: Option<String>,
    pub payload_mode: String,
    pub embedded_mod_count: usize,
    pub referenced_mod_count: usize,
    pub has_legacy_runtime_plugins_payload: bool,
    #[serde(default)]
    pub mods: Vec<ImportProfilePackPreviewModDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProfileModZipPreviewResult {
    pub cancelled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imported_mod: Option<ImportProfileModZipModDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProfileModZipInput {
    pub profile_id: Option<String>,
    pub source_path: Option<String>,
    #[serde(default)]
    pub add_to_cache: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProfileModZipModDto {
    pub package_id: String,
    pub package_name: String,
    pub version_id: String,
    pub version_number: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProfileModZipResult {
    pub cancelled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    pub added_to_cache: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imported_mod: Option<ImportProfileModZipModDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<ProfileDetailDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileManifest {
    pub schema_version: u32,
    pub updated_at: String,
    pub profile: ProfileManifestProfile,
    #[serde(default)]
    pub mods: Vec<ProfileManifestModEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileManifestProfile {
    pub id: String,
    pub name: String,
    pub notes: String,
    pub game_path: String,
    pub launch_mode_default: String,
    pub is_builtin_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileManifestModEntry {
    pub package_id: String,
    pub package_name: String,
    pub version_id: String,
    pub version_number: String,
    pub enabled: bool,
    pub source_kind: String,
    pub install_dir: String,
    pub installed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfilePackManifest {
    pub schema_version: u32,
    pub kind: String,
    pub exported_at: String,
    pub source_profile_id: String,
    pub source_profile_name: String,
    pub mod_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfilePackProfileDocument {
    pub name: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub game_path: String,
    #[serde(default = "default_launch_mode")]
    pub launch_mode_default: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfilePackModsLockDocument {
    pub schema_version: u32,
    #[serde(default)]
    pub mods: Vec<ProfilePackModLockEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfilePackModLockEntry {
    pub package_id: String,
    pub package_name: String,
    pub version_id: String,
    pub version_number: String,
    #[serde(default = "default_enabled_true")]
    pub enabled: bool,
    #[serde(default = "default_source_kind")]
    pub source_kind: String,
    pub install_dir: String,
    pub installed_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalZipManifest {
    pub name: Option<String>,
    pub version_number: Option<String>,
    pub full_name: Option<String>,
    pub author_name: Option<String>,
}

#[derive(Debug, Clone)]
struct ResolvedImportedModIdentity {
    package_id: String,
    package_name: String,
    version_id: String,
    version_number: String,
}

pub fn list_profiles(connection: &Connection) -> Result<Vec<ProfileSummaryDto>, InternalError> {
    let mut statement = connection.prepare(
        "SELECT id, name, notes, game_path, last_played_at, launch_mode_default, is_builtin_default
         FROM profiles
         ORDER BY is_builtin_default DESC, CASE WHEN is_builtin_default = 1 THEN 0 ELSE 1 END, updated_at DESC, name COLLATE NOCASE ASC",
    )?;

    let rows = statement.query_map([], map_profile_summary_row)?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(InternalError::from)
}

pub fn get_active_profile(
    connection: &Connection,
) -> Result<Option<ProfileDetailDto>, InternalError> {
    let profile_id = get_active_profile_id(connection)?;
    get_profile_detail(connection, &profile_id)
}

pub fn set_active_profile(
    connection: &Connection,
    profile_id: &str,
) -> Result<Option<ProfileDetailDto>, InternalError> {
    if !profile_exists(connection, profile_id)? {
        return Err(InternalError::app(
            "PROFILE_NOT_FOUND",
            format!("Profile {profile_id} does not exist."),
        ));
    }

    let updated_at = now_rfc3339()?;
    upsert_setting(
        connection,
        "profiles.active_id",
        &serde_json::to_string(&profile_id)?,
        &updated_at,
    )?;

    get_profile_detail(connection, profile_id)
}

pub fn create_profile(
    connection: &Connection,
    input: CreateProfileInput,
) -> Result<ProfileDetailDto, InternalError> {
    let name = input.name.trim();

    if name.is_empty() {
        return Err(InternalError::app(
            "PROFILE_NAME_INVALID",
            "Profile name cannot be empty.",
        ));
    }

    let duplicate_exists = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM profiles WHERE name = ?1 COLLATE NOCASE)",
        params![name],
        |row| row.get::<_, i64>(0),
    )? != 0;

    if duplicate_exists {
        return Err(InternalError::app(
            "PROFILE_NAME_CONFLICT",
            "A profile with that name already exists.",
        ));
    }

    let launch_mode_default = match input.launch_mode_default.as_deref() {
        Some("steam") | None => "steam".to_string(),
        Some("direct") => "direct".to_string(),
        Some(other) => {
            return Err(InternalError::app(
                "PROFILE_LAUNCH_MODE_INVALID",
                format!("Unsupported launch mode: {other}"),
            ))
        }
    };

    let profile_id = format!(
        "profile-{}",
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    );
    let created_at = now_rfc3339()?;
    let notes = input.notes.unwrap_or_default();
    let game_path = input.game_path.unwrap_or_default();

    connection.execute(
        "INSERT INTO profiles (
            id,
            name,
            notes,
            game_path,
            launch_mode_default,
            created_at,
            updated_at,
            last_played_at,
            is_builtin_default
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, 0)",
        params![
            profile_id,
            name,
            notes,
            game_path,
            launch_mode_default,
            created_at,
            now_rfc3339()?
        ],
    )?;

    upsert_setting(
        connection,
        "profiles.active_id",
        &serde_json::to_string(&profile_id)?,
        &now_rfc3339()?,
    )?;

    get_profile_detail(connection, &profile_id)?.ok_or_else(|| {
        InternalError::app(
            "PROFILE_NOT_FOUND",
            "The created profile could not be loaded back from the database.",
        )
    })
}

pub fn update_profile(
    connection: &Connection,
    input: UpdateProfileInput,
) -> Result<ProfileDetailDto, InternalError> {
    let name = input.name.trim();

    if name.is_empty() {
        return Err(InternalError::app(
            "PROFILE_NAME_INVALID",
            "Profile name cannot be empty.",
        ));
    }

    let existing = connection
        .query_row(
            "SELECT is_builtin_default FROM profiles WHERE id = ?1",
            params![input.profile_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;

    let Some(is_builtin_default) = existing.map(|value| value != 0) else {
        return Err(InternalError::app(
            "PROFILE_NOT_FOUND",
            "That profile does not exist.",
        ));
    };

    if is_builtin_default && name != "Default" {
        return Err(InternalError::app(
            "DEFAULT_PROFILE_PROTECTED",
            "The built-in Default profile name cannot be changed.",
        ));
    }

    let duplicate_exists = connection.query_row(
        "SELECT EXISTS(
                SELECT 1
                FROM profiles
                WHERE name = ?1 COLLATE NOCASE
                  AND id != ?2
            )",
        params![name, input.profile_id],
        |row| row.get::<_, i64>(0),
    )? != 0;

    if duplicate_exists {
        return Err(InternalError::app(
            "PROFILE_NAME_CONFLICT",
            "A profile with that name already exists.",
        ));
    }

    let launch_mode_default = match input.launch_mode_default.as_deref() {
        Some("steam") | None => "steam".to_string(),
        Some("direct") => "direct".to_string(),
        Some(other) => {
            return Err(InternalError::app(
                "PROFILE_LAUNCH_MODE_INVALID",
                format!("Unsupported launch mode: {other}"),
            ))
        }
    };

    connection.execute(
        "UPDATE profiles
         SET name = ?1,
             notes = ?2,
             game_path = ?3,
             launch_mode_default = ?4,
             updated_at = ?5
         WHERE id = ?6",
        params![
            name,
            input.notes.unwrap_or_default(),
            input.game_path.unwrap_or_default(),
            launch_mode_default,
            now_rfc3339()?,
            input.profile_id
        ],
    )?;

    get_profile_detail(connection, &input.profile_id)?.ok_or_else(|| {
        InternalError::app(
            "PROFILE_NOT_FOUND",
            "The updated profile could not be loaded back from the database.",
        )
    })
}

pub fn delete_profile(
    connection: &Connection,
    profile_id: &str,
) -> Result<DeleteProfileResult, InternalError> {
    let profile = connection
        .query_row(
            "SELECT id, is_builtin_default FROM profiles WHERE id = ?1",
            params![profile_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? != 0)),
        )
        .optional()?;

    let Some((profile_id, is_builtin_default)) = profile else {
        return Err(InternalError::app(
            "PROFILE_NOT_FOUND",
            "That profile does not exist.",
        ));
    };

    if is_builtin_default {
        return Err(InternalError::app(
            "DEFAULT_PROFILE_PROTECTED",
            "The built-in Default profile cannot be deleted.",
        ));
    }

    connection.execute("DELETE FROM profiles WHERE id = ?1", params![profile_id])?;

    let next_active_profile_id = match get_active_profile_id(connection)? {
        active_id if active_id == profile_id => {
            upsert_setting(
                connection,
                "profiles.active_id",
                &serde_json::to_string("default")?,
                &now_rfc3339()?,
            )?;
            Some("default".to_string())
        }
        active_id => Some(active_id),
    };

    Ok(DeleteProfileResult {
        deleted_id: profile_id,
        next_active_profile_id,
    })
}

pub fn get_profile_detail(
    connection: &Connection,
    profile_id: &str,
) -> Result<Option<ProfileDetailDto>, InternalError> {
    connection
        .query_row(
            "SELECT id, name, notes, game_path, last_played_at, launch_mode_default, is_builtin_default
             FROM profiles
             WHERE id = ?1",
            params![profile_id],
            map_profile_detail_row,
        )
        .optional()
        .map_err(InternalError::from)
}

pub fn reset_all_data(connection: &Connection) -> Result<(), InternalError> {
    reset_user_data(connection)
}

pub fn ensure_all_profile_storage(
    state: &AppState,
    connection: &Connection,
) -> Result<(), InternalError> {
    fs::create_dir_all(&state.profiles_dir)?;

    let mut statement = connection.prepare("SELECT id FROM profiles ORDER BY id ASC")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;

    for row in rows {
        ensure_profile_storage(state, connection, &row?)?;
    }

    Ok(())
}

pub fn ensure_profile_storage(
    state: &AppState,
    connection: &Connection,
    profile_id: &str,
) -> Result<(), InternalError> {
    let profile = get_profile_detail(connection, profile_id)?.ok_or_else(|| {
        InternalError::app(
            "PROFILE_NOT_FOUND",
            format!("Profile {profile_id} does not exist."),
        )
    })?;

    let profile_dir = profile_dir(state, profile_id);
    fs::create_dir_all(profile_dir.join("mods"))?;
    fs::create_dir_all(profile_dir.join("runtime"))?;
    fs::create_dir_all(profile_dir.join("runtime").join("BepInEx").join("plugins"))?;
    fs::create_dir_all(profile_dir.join("runtime").join("BepInEx").join("config"))?;

    let existing_mods = read_profile_manifest_mods(state, profile_id)?;
    write_profile_manifest(state, &profile, existing_mods)
}

pub fn delete_profile_storage(state: &AppState, profile_id: &str) -> Result<(), InternalError> {
    let profile_dir = profile_dir(state, profile_id);

    if profile_dir.exists() {
        fs::remove_dir_all(profile_dir)?;
    }

    Ok(())
}

pub fn clear_profiles_storage(state: &AppState) -> Result<(), InternalError> {
    clear_directory_contents(&state.profiles_dir)
}

pub fn open_profiles_folder(state: &AppState) -> Result<(), InternalError> {
    fs::create_dir_all(&state.profiles_dir)?;
    open_folder_path(
        &state.profiles_dir,
        "OPEN_PROFILES_FOLDER_FAILED",
        "Failed to open the profiles folder in the system file explorer.",
    )
}

pub fn open_active_profile_folder(
    state: &AppState,
    connection: &Connection,
) -> Result<(), InternalError> {
    let active_profile_id = get_active_profile_id(connection)?;

    ensure_profile_storage(state, connection, &active_profile_id)?;
    open_folder_path(
        &profile_dir(state, &active_profile_id),
        "OPEN_ACTIVE_PROFILE_FOLDER_FAILED",
        "Failed to open the active profile folder in the system file explorer.",
    )
}

pub fn get_profiles_storage_summary(
    state: &AppState,
    connection: &Connection,
) -> Result<ProfilesStorageSummaryDto, InternalError> {
    fs::create_dir_all(&state.profiles_dir)?;

    let profile_count = connection.query_row("SELECT COUNT(*) FROM profiles", [], |row| {
        row.get::<_, i64>(0)
    })?;

    let active_profile_id = get_active_profile_id(connection)?;
    ensure_profile_storage(state, connection, &active_profile_id)?;

    let profiles_total_bytes = directory_size_bytes(&state.profiles_dir)?;
    let active_profile_bytes = directory_size_bytes(&profile_dir(state, &active_profile_id))?;

    Ok(ProfilesStorageSummaryDto {
        profile_count: profile_count.max(0) as usize,
        profiles_total_bytes: profiles_total_bytes.min(i64::MAX as u64) as i64,
        active_profile_bytes: active_profile_bytes.min(i64::MAX as u64) as i64,
    })
}

pub fn get_profile_storage_size_bytes(
    state: &AppState,
    profile_id: &str,
) -> Result<i64, InternalError> {
    let bytes = directory_size_bytes(&profile_dir(state, profile_id))?;
    Ok(bytes.min(i64::MAX as u64) as i64)
}

fn collect_unavailable_profile_pack_mods(
    connection: &Connection,
    installed_mods: &[ProfileManifestModEntry],
) -> Result<Vec<PreviewExportProfilePackUnavailableModDto>, InternalError> {
    let mut availability_statement = connection.prepare(
        "SELECT download_url
         FROM package_versions
         WHERE package_id = ?1
           AND id = ?2
         LIMIT 1",
    )?;

    let mut unavailable_mods = Vec::new();
    for entry in installed_mods {
        let download_url = availability_statement
            .query_row(params![entry.package_id, entry.version_id], |row| {
                row.get::<_, String>(0)
            })
            .optional()?;

        let unavailable_reason = match download_url {
            None => Some("missingVersion"),
            Some(url) if url.trim().is_empty() => Some("missingDownloadUrl"),
            _ => None,
        };

        if let Some(unavailable_reason) = unavailable_reason {
            unavailable_mods.push(PreviewExportProfilePackUnavailableModDto {
                package_id: entry.package_id.clone(),
                package_name: entry.package_name.clone(),
                version_id: entry.version_id.clone(),
                version_number: entry.version_number.clone(),
                unavailable_reason: unavailable_reason.to_string(),
            });
        }
    }

    unavailable_mods.sort_by(|left, right| {
        left.package_name
            .to_lowercase()
            .cmp(&right.package_name.to_lowercase())
            .then_with(|| left.version_number.cmp(&right.version_number))
    });

    let missing_version_count = unavailable_mods
        .iter()
        .filter(|entry| entry.unavailable_reason == "missingVersion")
        .count();
    let missing_download_url_count = unavailable_mods
        .iter()
        .filter(|entry| entry.unavailable_reason == "missingDownloadUrl")
        .count();
    eprintln!(
        "[profile-pack] unavailable detection checked_mods={} unavailable_mods={} missing_version={} missing_download_url={}",
        installed_mods.len(),
        unavailable_mods.len(),
        missing_version_count,
        missing_download_url_count
    );

    Ok(unavailable_mods)
}

pub fn preview_export_profile_pack(
    state: &AppState,
    connection: &Connection,
    profile_id: &str,
) -> Result<PreviewExportProfilePackResult, InternalError> {
    let profile = get_profile_detail(connection, profile_id)?.ok_or_else(|| {
        InternalError::app(
            "PROFILE_NOT_FOUND",
            format!("Profile {profile_id} does not exist."),
        )
    })?;

    ensure_profile_storage(state, connection, profile_id)?;
    let installed_mods = read_profile_manifest_mods(state, profile_id)?;
    let unavailable_mods = collect_unavailable_profile_pack_mods(connection, &installed_mods)?;
    eprintln!(
        "[profile-pack] export preview profile_id={} installed_mods={} unavailable_mods={}",
        profile.id,
        installed_mods.len(),
        unavailable_mods.len()
    );

    Ok(PreviewExportProfilePackResult {
        profile_id: profile.id,
        profile_name: profile.name,
        mod_count: installed_mods.len(),
        unavailable_mods,
    })
}

pub fn export_profile_pack(
    state: &AppState,
    connection: &Connection,
    profile_id: &str,
    embed_unavailable_payloads: bool,
) -> Result<ExportProfilePackResult, InternalError> {
    let profile = get_profile_detail(connection, profile_id)?.ok_or_else(|| {
        InternalError::app(
            "PROFILE_NOT_FOUND",
            format!("Profile {profile_id} does not exist."),
        )
    })?;

    ensure_profile_storage(state, connection, profile_id)?;
    let installed_mods = read_profile_manifest_mods(state, profile_id)?;
    let unavailable_mod_keys = if embed_unavailable_payloads {
        collect_unavailable_profile_pack_mods(connection, &installed_mods)?
            .into_iter()
            .map(|entry| installed_node_key(&entry.package_id, &entry.version_id))
            .collect::<HashSet<_>>()
    } else {
        HashSet::new()
    };
    let unavailable_mod_count = unavailable_mod_keys.len();

    let default_file_name = format!("{}.49pack", sanitize_path_segment(&profile.name));
    let Some(mut output_path) = FileDialog::new()
        .set_title("Export profile as .49pack")
        .add_filter("49modman profile pack", &["49pack"])
        .set_file_name(&default_file_name)
        .save_file()
    else {
        eprintln!(
            "[profile-pack] export cancelled profile_id={} embed_unavailable_payloads={} unavailable_mods={}",
            profile.id,
            embed_unavailable_payloads,
            unavailable_mod_count
        );
        return Ok(ExportProfilePackResult {
            cancelled: true,
            path: None,
            profile_id: None,
            profile_name: None,
            mod_count: None,
        });
    };
    eprintln!(
        "[profile-pack] export started profile_id={} installed_mods={} embed_unavailable_payloads={} unavailable_mods={}",
        profile.id,
        installed_mods.len(),
        embed_unavailable_payloads,
        unavailable_mod_count
    );

    if output_path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| !value.eq_ignore_ascii_case("49pack"))
        .unwrap_or(true)
    {
        output_path.set_extension("49pack");
    }

    let options = default_zip_options();
    let output_file = fs::File::create(&output_path)?;
    let mut writer = ZipWriter::new(output_file);

    let manifest = ProfilePackManifest {
        schema_version: PROFILE_PACK_SCHEMA_VERSION,
        kind: "49pack".to_string(),
        exported_at: now_rfc3339()?,
        source_profile_id: profile.id.clone(),
        source_profile_name: profile.name.clone(),
        mod_count: installed_mods.len(),
    };
    write_zip_json_entry(&mut writer, "manifest.json", &manifest, options)?;

    let profile_document = ProfilePackProfileDocument {
        name: profile.name.clone(),
        notes: profile.notes.clone(),
        game_path: profile.game_path.clone(),
        launch_mode_default: profile.launch_mode_default.clone(),
    };
    write_zip_json_entry(&mut writer, "profile.json", &profile_document, options)?;

    let mods_lock = ProfilePackModsLockDocument {
        schema_version: PROFILE_MANIFEST_SCHEMA_VERSION,
        mods: installed_mods
            .iter()
            .map(|entry| ProfilePackModLockEntry {
                package_id: entry.package_id.clone(),
                package_name: entry.package_name.clone(),
                version_id: entry.version_id.clone(),
                version_number: entry.version_number.clone(),
                enabled: entry.enabled,
                source_kind: entry.source_kind.clone(),
                install_dir: entry.install_dir.clone(),
                installed_at: Some(entry.installed_at.clone()),
            })
            .collect(),
    };
    write_zip_json_entry(&mut writer, "mods.lock.json", &mods_lock, options)?;

    if !profile.notes.trim().is_empty() {
        let notes = format!("{}\n", profile.notes.trim_end());
        write_zip_text_entry(&mut writer, "notes.txt", &notes, options)?;
    }

    let mut embedded_mod_payload_count = 0_usize;
    let mut missing_embedded_payload_count = 0_usize;
    if embed_unavailable_payloads {
        for entry in &installed_mods {
            if !unavailable_mod_keys
                .contains(&installed_node_key(&entry.package_id, &entry.version_id))
            {
                continue;
            }

            let Some(source_path) = profile_install_dir_path(state, profile_id, &entry.install_dir)
            else {
                missing_embedded_payload_count += 1;
                continue;
            };
            if !source_path.exists() {
                missing_embedded_payload_count += 1;
                continue;
            }

            copy_path_into_zip(
                &mut writer,
                &source_path,
                Path::new(&entry.install_dir),
                options,
            )?;
            embedded_mod_payload_count += 1;
        }
    }

    let runtime_config_dir = profile_dir(state, profile_id).join("runtime/BepInEx/config");
    if runtime_config_dir.exists() {
        copy_directory_children_into_zip(
            &mut writer,
            &runtime_config_dir,
            Path::new("config/BepInEx/config"),
            options,
        )?;
    }

    writer.finish()?;
    let export_payload_mode = if embedded_mod_payload_count > 0 {
        "hybrid"
    } else {
        "compact"
    };
    eprintln!(
        "[profile-pack] export completed profile_id={} payload_mode={} embedded_mod_payloads={} missing_embedded_payloads={} output_path={}",
        profile.id,
        export_payload_mode,
        embedded_mod_payload_count,
        missing_embedded_payload_count,
        output_path.display()
    );

    Ok(ExportProfilePackResult {
        cancelled: false,
        path: Some(output_path.display().to_string()),
        profile_id: Some(profile.id),
        profile_name: Some(profile.name),
        mod_count: Some(installed_mods.len()),
    })
}

pub fn preview_import_profile_pack() -> Result<ImportProfilePackPreviewResult, InternalError> {
    let Some(source_path) = FileDialog::new()
        .set_title("Import .49pack profile")
        .add_filter("49modman profile pack", &["49pack"])
        .pick_file()
    else {
        return Ok(ImportProfilePackPreviewResult {
            cancelled: true,
            source_path: None,
            profile_name: None,
            payload_mode: "compact".to_string(),
            embedded_mod_count: 0,
            referenced_mod_count: 0,
            has_legacy_runtime_plugins_payload: false,
            mods: Vec::new(),
        });
    };

    let read_result = read_pack_documents_from_path(&source_path)?;
    let mut mods = read_result
        .mods_lock_document
        .mods
        .into_iter()
        .filter(|entry| !entry.package_id.trim().is_empty() && !entry.version_id.trim().is_empty())
        .map(|entry| ImportProfilePackPreviewModDto {
            package_id: entry.package_id,
            package_name: entry.package_name,
            version_id: entry.version_id,
            version_number: entry.version_number,
        })
        .collect::<Vec<_>>();
    mods.sort_by(|left, right| {
        left.package_name
            .to_lowercase()
            .cmp(&right.package_name.to_lowercase())
            .then_with(|| left.version_number.cmp(&right.version_number))
    });

    Ok(ImportProfilePackPreviewResult {
        cancelled: false,
        source_path: Some(source_path.display().to_string()),
        profile_name: Some(read_result.profile_document.name),
        payload_mode: read_result.payload_summary.payload_mode,
        embedded_mod_count: read_result.payload_summary.embedded_mod_count,
        referenced_mod_count: read_result.payload_summary.referenced_mod_count,
        has_legacy_runtime_plugins_payload: read_result
            .payload_summary
            .has_legacy_runtime_plugins_payload,
        mods,
    })
}

pub fn preview_import_profile_mod_zip(
    connection: &Connection,
) -> Result<ImportProfileModZipPreviewResult, InternalError> {
    let Some(source_path) = FileDialog::new()
        .set_title("Import mod .zip")
        .add_filter("ZIP archive", &["zip"])
        .pick_file()
    else {
        return Ok(ImportProfileModZipPreviewResult {
            cancelled: true,
            source_path: None,
            imported_mod: None,
        });
    };

    if !source_path.is_file() {
        return Err(InternalError::app(
            "MOD_ARCHIVE_NOT_FOUND",
            "The selected mod archive no longer exists.",
        ));
    }

    let resolved_identity = resolve_imported_mod_identity(connection, &source_path)?;

    Ok(ImportProfileModZipPreviewResult {
        cancelled: false,
        source_path: Some(source_path.display().to_string()),
        imported_mod: Some(ImportProfileModZipModDto {
            package_id: resolved_identity.package_id,
            package_name: resolved_identity.package_name,
            version_id: resolved_identity.version_id,
            version_number: resolved_identity.version_number,
        }),
    })
}

pub fn import_profile_pack(
    state: &AppState,
    connection: &Connection,
    source_path: &str,
) -> Result<ImportProfilePackResult, InternalError> {
    let source_path = Path::new(source_path);
    if !source_path.is_file() {
        return Err(InternalError::app(
            "PROFILE_PACK_NOT_FOUND",
            "The selected .49pack file no longer exists.",
        ));
    }

    let read_result = read_pack_documents_from_path(source_path)?;
    let archive_file = fs::File::open(source_path)?;
    let mut archive = ZipArchive::new(archive_file)?;

    let desired_name = read_result.profile_document.name.trim();
    if desired_name.is_empty() {
        return Err(InternalError::app(
            "PROFILE_IMPORT_INVALID",
            "That .49pack profile name is empty.",
        ));
    }

    let import_profile_name = resolve_import_profile_name(connection, desired_name)?;
    let launch_mode_default = match read_result.profile_document.launch_mode_default.as_str() {
        "direct" => Some("direct".to_string()),
        _ => Some("steam".to_string()),
    };

    let mut profile = create_profile(
        connection,
        CreateProfileInput {
            name: import_profile_name,
            notes: Some(read_result.profile_document.notes),
            game_path: Some(read_result.profile_document.game_path),
            launch_mode_default,
        },
    )?;

    ensure_profile_storage(state, connection, &profile.id)?;

    let profile_root = profile_dir(state, &profile.id);
    extract_zip_tree_into_directory(&mut archive, &["mods"], &profile_root.join("mods"))?;
    extract_zip_tree_into_directory(
        &mut archive,
        &["config", "BepInEx", "config"],
        &profile_root.join("runtime/BepInEx/config"),
    )?;
    extract_zip_tree_into_directory(
        &mut archive,
        &["config", "BepInEx", "plugins"],
        &profile_root.join("runtime/BepInEx/plugins"),
    )?;

    let fallback_installed_at = now_rfc3339()?;
    let imported_mods = read_result
        .mods_lock_document
        .mods
        .into_iter()
        .filter(|entry| !entry.package_id.trim().is_empty() && !entry.version_id.trim().is_empty())
        .map(|entry| ProfileManifestModEntry {
            package_id: entry.package_id,
            package_name: entry.package_name,
            version_id: entry.version_id,
            version_number: entry.version_number,
            enabled: entry.enabled,
            source_kind: entry.source_kind,
            install_dir: entry.install_dir,
            installed_at: entry
                .installed_at
                .unwrap_or_else(|| fallback_installed_at.clone()),
        })
        .collect::<Vec<_>>();

    write_profile_manifest(state, &profile, imported_mods)?;
    profile.installed_mods = read_profile_installed_mods(state, &profile.id)?;

    Ok(ImportProfilePackResult {
        cancelled: false,
        source_path: Some(source_path.display().to_string()),
        payload_mode: read_result.payload_summary.payload_mode,
        embedded_mod_count: read_result.payload_summary.embedded_mod_count,
        referenced_mod_count: read_result.payload_summary.referenced_mod_count,
        has_legacy_runtime_plugins_payload: read_result
            .payload_summary
            .has_legacy_runtime_plugins_payload,
        profile: Some(profile),
    })
}

pub fn import_profile_mod_zip(
    state: &AppState,
    connection: &Connection,
    input: ImportProfileModZipInput,
) -> Result<ImportProfileModZipResult, InternalError> {
    let target_profile_id = match input.profile_id.as_deref() {
        Some(profile_id) => profile_id.to_string(),
        None => get_active_profile_id(connection)?,
    };
    let mut profile = get_profile_detail(connection, &target_profile_id)?.ok_or_else(|| {
        InternalError::app(
            "PROFILE_NOT_FOUND",
            format!("Profile {target_profile_id} does not exist."),
        )
    })?;
    ensure_profile_storage(state, connection, &profile.id)?;

    let source_path = if let Some(source_path) = input
        .source_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        PathBuf::from(source_path)
    } else {
        let Some(selected_path) = FileDialog::new()
            .set_title("Import mod .zip")
            .add_filter("ZIP archive", &["zip"])
            .pick_file()
        else {
            return Ok(ImportProfileModZipResult {
                cancelled: true,
                source_path: None,
                added_to_cache: false,
                imported_mod: None,
                profile: None,
            });
        };
        selected_path
    };

    if !source_path.is_file() {
        return Err(InternalError::app(
            "MOD_ARCHIVE_NOT_FOUND",
            "The selected mod archive no longer exists.",
        ));
    }

    let resolved_identity = resolve_imported_mod_identity(connection, &source_path)?;
    let profile_root = profile_dir(state, &profile.id);
    let mods_dir = profile_root.join("mods");
    let folder_name = format!(
        "{}-{}",
        sanitize_path_segment(&resolved_identity.package_name),
        sanitize_path_segment(&resolved_identity.version_number)
    );
    let install_dir = mods_dir.join(&folder_name);
    fs::create_dir_all(&mods_dir)?;

    if install_dir.exists() {
        fs::remove_dir_all(&install_dir)?;
    }
    fs::create_dir_all(&install_dir)?;
    extract_zip_archive(&source_path, &install_dir)?;

    let mut installed_mods = read_profile_manifest_mods(state, &profile.id)?;
    let install_dir_relative = format!("mods/{folder_name}");
    let installed_at = now_rfc3339()?;

    if let Some(entry) = installed_mods.iter_mut().find(|entry| {
        entry.package_id == resolved_identity.package_id
            && entry.version_id == resolved_identity.version_id
    }) {
        entry.package_name = resolved_identity.package_name.clone();
        entry.version_number = resolved_identity.version_number.clone();
        entry.enabled = true;
        entry.source_kind = "local_zip".to_string();
        entry.install_dir = install_dir_relative.clone();
        entry.installed_at = installed_at.clone();
    } else {
        installed_mods.push(ProfileManifestModEntry {
            package_id: resolved_identity.package_id.clone(),
            package_name: resolved_identity.package_name.clone(),
            version_id: resolved_identity.version_id.clone(),
            version_number: resolved_identity.version_number.clone(),
            enabled: true,
            source_kind: "local_zip".to_string(),
            install_dir: install_dir_relative,
            installed_at,
        });
    }

    write_profile_manifest(state, &profile, installed_mods)?;

    let added_to_cache = if input.add_to_cache.unwrap_or(false) {
        add_local_mod_archive_to_cache(state, connection, &source_path)?;
        true
    } else {
        false
    };

    profile.installed_mods = read_profile_installed_mods(state, &profile.id)?;

    Ok(ImportProfileModZipResult {
        cancelled: false,
        source_path: Some(source_path.display().to_string()),
        added_to_cache,
        imported_mod: Some(ImportProfileModZipModDto {
            package_id: resolved_identity.package_id,
            package_name: resolved_identity.package_name,
            version_id: resolved_identity.version_id,
            version_number: resolved_identity.version_number,
        }),
        profile: Some(profile),
    })
}

pub fn read_profile_manifest_mods(
    state: &AppState,
    profile_id: &str,
) -> Result<Vec<ProfileManifestModEntry>, InternalError> {
    let manifest_path = profile_manifest_path(state, profile_id);
    if !manifest_path.is_file() {
        return Ok(Vec::new());
    }

    let manifest_bytes = fs::read(manifest_path)?;
    let mut manifest = serde_json::from_slice::<ProfileManifest>(&manifest_bytes)?;
    let original_mod_count = manifest.mods.len();

    manifest.mods.retain(|entry| {
        profile_install_dir_path(state, profile_id, &entry.install_dir)
            .map(|install_path| install_path.exists())
            .unwrap_or(false)
    });

    if manifest.mods.len() != original_mod_count {
        manifest.updated_at = now_rfc3339()?;
        write_profile_manifest_document(state, profile_id, &manifest)?;
    }

    Ok(manifest.mods)
}

pub fn read_profile_installed_mods(
    state: &AppState,
    profile_id: &str,
) -> Result<Vec<ProfileInstalledModDto>, InternalError> {
    let entries = read_profile_manifest_mods(state, profile_id)?;
    let mut mods = Vec::with_capacity(entries.len());

    for entry in entries {
        let icon_data_url = resolve_mod_icon_data_url(state, profile_id, &entry.install_dir)?;
        mods.push(ProfileInstalledModDto {
            package_id: entry.package_id,
            package_name: entry.package_name,
            version_id: entry.version_id,
            version_number: entry.version_number,
            enabled: entry.enabled,
            source_kind: entry.source_kind,
            install_dir: entry.install_dir,
            installed_at: entry.installed_at,
            icon_data_url,
        });
    }

    Ok(mods)
}

pub fn set_profile_mod_enabled(
    state: &AppState,
    profile: &ProfileDetailDto,
    package_id: &str,
    version_id: &str,
    enabled: bool,
) -> Result<ProfileDetailDto, InternalError> {
    let mut installed_mods = read_profile_manifest_mods(state, &profile.id)?;
    let Some(entry) = installed_mods
        .iter_mut()
        .find(|entry| entry.package_id == package_id && entry.version_id == version_id)
    else {
        return Err(InternalError::app(
            "PROFILE_MOD_NOT_FOUND",
            "That mod version is not installed in this profile.",
        ));
    };

    entry.enabled = enabled;
    write_profile_manifest(state, profile, installed_mods)?;

    let mut updated_profile = profile.clone();
    updated_profile.installed_mods = read_profile_installed_mods(state, &profile.id)?;
    Ok(updated_profile)
}

pub fn uninstall_profile_mod(
    state: &AppState,
    profile: &ProfileDetailDto,
    package_id: &str,
    version_id: &str,
) -> Result<ProfileDetailDto, InternalError> {
    let mut installed_mods = read_profile_manifest_mods(state, &profile.id)?;
    let Some(entry_index) = installed_mods
        .iter()
        .position(|entry| entry.package_id == package_id && entry.version_id == version_id)
    else {
        return Err(InternalError::app(
            "PROFILE_MOD_NOT_FOUND",
            "That mod version is not installed in this profile.",
        ));
    };

    let removed_entry = installed_mods.remove(entry_index);
    if let Some(install_path) =
        profile_install_dir_path(state, &profile.id, &removed_entry.install_dir)
    {
        if install_path.is_dir() {
            fs::remove_dir_all(install_path)?;
        } else if install_path.is_file() {
            fs::remove_file(install_path)?;
        }
    }

    write_profile_manifest(state, profile, installed_mods)?;

    let mut updated_profile = profile.clone();
    updated_profile.installed_mods = read_profile_installed_mods(state, &profile.id)?;
    Ok(updated_profile)
}

pub fn get_uninstall_dependants(
    state: &AppState,
    connection: &Connection,
    input: GetUninstallDependantsInput,
) -> Result<Vec<UninstallDependantDto>, InternalError> {
    let Some(_profile) = get_profile_detail(connection, &input.profile_id)? else {
        return Err(InternalError::app(
            "PROFILE_NOT_FOUND",
            format!("Profile {} does not exist.", input.profile_id),
        ));
    };

    let version_ids = input
        .version_ids
        .into_iter()
        .filter(|entry| !entry.trim().is_empty())
        .collect::<HashSet<_>>();
    if version_ids.is_empty() {
        return Ok(Vec::new());
    }

    let installed_mods = read_profile_manifest_mods(state, &input.profile_id)?;
    if installed_mods.is_empty() {
        return Ok(Vec::new());
    }

    #[derive(Debug, Clone)]
    struct InstalledNode {
        package_id: String,
        package_name: String,
        version_id: String,
        version_number: String,
        dependency_keys: Vec<String>,
    }

    let mut nodes_by_key = HashMap::<String, InstalledNode>::new();
    let mut node_key_by_raw_dependency = HashMap::<String, String>::new();

    for entry in &installed_mods {
        let key = installed_node_key(&entry.package_id, &entry.version_id);
        nodes_by_key.insert(
            key.clone(),
            InstalledNode {
                package_id: entry.package_id.clone(),
                package_name: entry.package_name.clone(),
                version_id: entry.version_id.clone(),
                version_number: entry.version_number.clone(),
                dependency_keys: Vec::new(),
            },
        );
        node_key_by_raw_dependency.insert(
            dependency_raw_key(&entry.package_name, &entry.version_number),
            key,
        );
    }

    let removed_version_keys = installed_mods
        .iter()
        .filter(|entry| {
            entry.package_id == input.package_id && version_ids.contains(&entry.version_id)
        })
        .map(|entry| installed_node_key(&entry.package_id, &entry.version_id))
        .collect::<HashSet<_>>();
    if removed_version_keys.is_empty() {
        return Ok(Vec::new());
    }

    let mut dependency_statement =
        connection.prepare("SELECT dependencies_json FROM package_versions WHERE id = ?1")?;

    for node in nodes_by_key.values_mut() {
        let dependencies_json = dependency_statement
            .query_row(params![node.version_id.clone()], |row| {
                row.get::<_, String>(0)
            })
            .optional()?;
        let Some(dependencies_json) = dependencies_json else {
            continue;
        };

        for dependency_raw in parse_dependency_entries(&dependencies_json) {
            let dependency_raw = dependency_raw.trim();
            if dependency_raw.is_empty() {
                continue;
            }

            if let Some(dependency_key) = node_key_by_raw_dependency.get(dependency_raw) {
                node.dependency_keys.push(dependency_key.clone());
            }
        }
    }

    fn min_depth_to_removed(
        key: &str,
        nodes_by_key: &HashMap<String, InstalledNode>,
        removed_version_keys: &HashSet<String>,
        memo: &mut HashMap<String, Option<usize>>,
        visiting: &mut HashSet<String>,
    ) -> Option<usize> {
        if let Some(cached) = memo.get(key) {
            return *cached;
        }

        if !visiting.insert(key.to_string()) {
            return None;
        }

        let mut best: Option<usize> = None;
        if let Some(node) = nodes_by_key.get(key) {
            for dependency_key in &node.dependency_keys {
                let candidate = if removed_version_keys.contains(dependency_key) {
                    Some(1)
                } else {
                    min_depth_to_removed(
                        dependency_key,
                        nodes_by_key,
                        removed_version_keys,
                        memo,
                        visiting,
                    )
                    .map(|depth| depth.saturating_add(1))
                };

                if let Some(depth) = candidate {
                    best = match best {
                        Some(current) => Some(current.min(depth)),
                        None => Some(depth),
                    };
                }
            }
        }

        visiting.remove(key);
        memo.insert(key.to_string(), best);
        best
    }

    let mut memo = HashMap::<String, Option<usize>>::new();
    let mut dependants = Vec::new();

    for (key, node) in &nodes_by_key {
        if removed_version_keys.contains(key) {
            continue;
        }

        let mut visiting = HashSet::new();
        let Some(min_depth) = min_depth_to_removed(
            key,
            &nodes_by_key,
            &removed_version_keys,
            &mut memo,
            &mut visiting,
        ) else {
            continue;
        };

        dependants.push(UninstallDependantDto {
            package_id: node.package_id.clone(),
            package_name: node.package_name.clone(),
            version_id: node.version_id.clone(),
            version_number: node.version_number.clone(),
            min_depth,
        });
    }

    dependants.sort_by(|left, right| {
        left.min_depth
            .cmp(&right.min_depth)
            .then_with(|| {
                left.package_name
                    .to_lowercase()
                    .cmp(&right.package_name.to_lowercase())
            })
            .then_with(|| left.version_number.cmp(&right.version_number))
    });

    Ok(dependants)
}

pub fn install_cached_archive_into_profile(
    state: &AppState,
    profile: &ProfileDetailDto,
    archive_path: &Path,
    package_id: &str,
    package_name: &str,
    version_id: &str,
    version_number: &str,
) -> Result<(), InternalError> {
    let profile_id = profile.id.as_str();

    if !archive_path.is_file() {
        return Err(InternalError::app(
            "CACHE_ARCHIVE_MISSING",
            "The cached archive is missing from disk. Try installing again.",
        ));
    }

    let folder_name = format!(
        "{}-{}",
        sanitize_path_segment(package_name),
        sanitize_path_segment(version_number)
    );
    let profile_root = profile_dir(state, profile_id);
    let mods_dir = profile_root.join("mods");
    let install_dir = mods_dir.join(&folder_name);

    fs::create_dir_all(&mods_dir)?;

    if install_dir.exists() {
        fs::remove_dir_all(&install_dir)?;
    }
    fs::create_dir_all(&install_dir)?;
    extract_zip_archive(archive_path, &install_dir)?;

    let mut installed_mods = read_profile_manifest_mods(state, profile_id)?;
    let install_dir_relative = format!("mods/{folder_name}");
    let installed_at = now_rfc3339()?;

    if let Some(entry) = installed_mods
        .iter_mut()
        .find(|entry| entry.package_id == package_id && entry.version_id == version_id)
    {
        entry.package_name = package_name.to_string();
        entry.version_number = version_number.to_string();
        entry.enabled = true;
        entry.source_kind = "thunderstore".to_string();
        entry.install_dir = install_dir_relative.clone();
        entry.installed_at = installed_at.clone();
    } else {
        installed_mods.push(ProfileManifestModEntry {
            package_id: package_id.to_string(),
            package_name: package_name.to_string(),
            version_id: version_id.to_string(),
            version_number: version_number.to_string(),
            enabled: true,
            source_kind: "thunderstore".to_string(),
            install_dir: install_dir_relative,
            installed_at,
        })
    }

    write_profile_manifest(state, profile, installed_mods)
}

fn write_profile_manifest(
    state: &AppState,
    profile: &ProfileDetailDto,
    installed_mods: Vec<ProfileManifestModEntry>,
) -> Result<(), InternalError> {
    let manifest = ProfileManifest {
        schema_version: PROFILE_MANIFEST_SCHEMA_VERSION,
        updated_at: now_rfc3339()?,
        profile: ProfileManifestProfile {
            id: profile.id.clone(),
            name: profile.name.clone(),
            notes: profile.notes.clone(),
            game_path: profile.game_path.clone(),
            launch_mode_default: profile.launch_mode_default.clone(),
            is_builtin_default: profile.is_builtin_default,
        },
        mods: installed_mods,
    };

    write_profile_manifest_document(state, &profile.id, &manifest)
}

fn write_profile_manifest_document(
    state: &AppState,
    profile_id: &str,
    manifest: &ProfileManifest,
) -> Result<(), InternalError> {
    let manifest_path = profile_manifest_path(state, profile_id);
    let profile_dir = manifest_path.parent().ok_or_else(|| {
        InternalError::app(
            "RESOURCE_LOAD_FAILED",
            "Failed to resolve profile manifest parent directory.",
        )
    })?;

    fs::create_dir_all(profile_dir.join("mods"))?;

    let temp_manifest_path = profile_dir.join(format!(
        "manifest.json.tmp-{}",
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    let manifest_json = serde_json::to_vec_pretty(manifest)?;

    let mut temp_file = fs::File::create(&temp_manifest_path)?;
    temp_file.write_all(&manifest_json)?;
    temp_file.write_all(b"\n")?;
    temp_file.sync_all()?;

    fs::rename(temp_manifest_path, manifest_path)?;
    Ok(())
}

fn extract_zip_archive(archive_path: &Path, destination: &Path) -> Result<(), InternalError> {
    let archive_file = fs::File::open(archive_path)?;
    let mut archive = ZipArchive::new(archive_file)?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let Some(entry_path) = entry.enclosed_name().map(|value| value.to_path_buf()) else {
            return Err(InternalError::app(
                "ARCHIVE_INVALID",
                "The downloaded archive contains invalid paths.",
            ));
        };

        let output_path = destination.join(entry_path);
        if entry.is_dir() {
            fs::create_dir_all(&output_path)?;
            continue;
        }

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut output_file = fs::File::create(&output_path)?;
        io::copy(&mut entry, &mut output_file)?;
    }

    Ok(())
}

fn profile_dir(state: &AppState, profile_id: &str) -> PathBuf {
    state.profiles_dir.join(profile_id)
}

fn profile_manifest_path(state: &AppState, profile_id: &str) -> PathBuf {
    profile_dir(state, profile_id).join("manifest.json")
}

fn resolve_mod_icon_data_url(
    state: &AppState,
    profile_id: &str,
    install_dir: &str,
) -> Result<Option<String>, InternalError> {
    let install_path = profile_install_dir_path(state, profile_id, install_dir);
    let Some(install_path) = install_path else {
        return Ok(None);
    };

    let icon_path = install_path.join("icon.png");
    if !icon_path.is_file() {
        return Ok(None);
    }

    let bytes = fs::read(icon_path)?;
    if bytes.is_empty() {
        return Ok(None);
    }

    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(Some(format!("data:image/png;base64,{encoded}")))
}

fn profile_install_dir_path(
    state: &AppState,
    profile_id: &str,
    install_dir: &str,
) -> Option<PathBuf> {
    let mut path = profile_dir(state, profile_id);

    for segment in install_dir.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return None;
        }
        path = path.join(segment);
    }

    Some(path)
}

fn resolve_imported_mod_identity(
    connection: &Connection,
    archive_path: &Path,
) -> Result<ResolvedImportedModIdentity, InternalError> {
    let file_stem = archive_path
        .file_stem()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("imported-mod")
        .to_string();
    let (file_name_hint, file_version_hint) = split_file_stem_name_and_version(&file_stem);

    let manifest = read_local_zip_manifest(archive_path)?;
    let mut package_name_candidates = Vec::new();
    let mut version_number = file_version_hint
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "local".to_string());

    if let Some(manifest) = manifest {
        if let Some(value) = manifest.full_name {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                package_name_candidates.push(trimmed.to_string());
            }
        }
        if let Some(name) = manifest.name {
            let trimmed = name.trim();
            if !trimmed.is_empty() {
                if let Some(author_name) = manifest.author_name {
                    let author_name = author_name.trim();
                    if !author_name.is_empty() {
                        package_name_candidates.push(format!("{author_name}-{trimmed}"));
                    }
                }
                package_name_candidates.push(trimmed.to_string());
            }
        }
        if let Some(manifest_version) = manifest.version_number {
            let trimmed = manifest_version.trim();
            if !trimmed.is_empty() {
                version_number = trimmed.to_string();
            }
        }
    }

    if !file_name_hint.trim().is_empty() {
        package_name_candidates.push(file_name_hint);
    }
    package_name_candidates.push(file_stem);

    let package_name_candidates = dedupe_non_empty_strings(package_name_candidates);

    for package_name in &package_name_candidates {
        if let Some((package_id, resolved_package_name, version_id)) =
            find_catalog_version_identity(connection, package_name, &version_number)?
        {
            return Ok(ResolvedImportedModIdentity {
                package_id,
                package_name: resolved_package_name,
                version_id,
                version_number: version_number.clone(),
            });
        }
    }

    let package_name = package_name_candidates
        .into_iter()
        .next()
        .unwrap_or_else(|| "Imported mod".to_string());
    let package_id = normalize_loose_identifier(&package_name);
    let version_id = format!(
        "local-{}-{}",
        package_id,
        normalize_loose_identifier(&version_number)
    );

    Ok(ResolvedImportedModIdentity {
        package_id,
        package_name,
        version_id,
        version_number,
    })
}

fn read_local_zip_manifest(archive_path: &Path) -> Result<Option<LocalZipManifest>, InternalError> {
    let archive_file = fs::File::open(archive_path)?;
    let mut archive = ZipArchive::new(archive_file)?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let Some(entry_path) = entry.enclosed_name() else {
            continue;
        };

        if entry_path
            .file_name()
            .and_then(|value| value.to_str())
            .map(|value| value.eq_ignore_ascii_case("manifest.json"))
            .unwrap_or(false)
        {
            let mut json = String::new();
            entry.read_to_string(&mut json)?;
            if json.trim().is_empty() {
                return Ok(None);
            }

            if let Ok(manifest) = serde_json::from_str::<LocalZipManifest>(&json) {
                return Ok(Some(manifest));
            }
        }
    }

    Ok(None)
}

fn split_file_stem_name_and_version(value: &str) -> (String, Option<String>) {
    let trimmed = value.trim();
    let Some((left, right)) = trimmed.rsplit_once('-') else {
        return (trimmed.to_string(), None);
    };
    let version = right.trim();
    if version.is_empty() {
        return (trimmed.to_string(), None);
    }

    let has_digit = version.chars().any(|character| character.is_ascii_digit());
    if !has_digit {
        return (trimmed.to_string(), None);
    }

    let name = left.trim();
    if name.is_empty() {
        return (trimmed.to_string(), None);
    }

    (name.to_string(), Some(version.to_string()))
}

fn dedupe_non_empty_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }

        let key = trimmed.to_lowercase();
        if seen.insert(key) {
            deduped.push(trimmed.to_string());
        }
    }
    deduped
}

fn find_catalog_version_identity(
    connection: &Connection,
    package_name: &str,
    version_number: &str,
) -> Result<Option<(String, String, String)>, InternalError> {
    connection
        .query_row(
            "SELECT p.id, p.full_name, pv.id
             FROM packages p
             INNER JOIN package_versions pv ON pv.package_id = p.id
             WHERE lower(p.full_name) = lower(?1)
               AND pv.version_number = ?2
             LIMIT 1",
            params![package_name, version_number],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(InternalError::from)
}

fn normalize_loose_identifier(value: &str) -> String {
    let normalized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();

    let collapsed = normalized
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    if collapsed.is_empty() {
        "mod".to_string()
    } else {
        collapsed
    }
}

fn add_local_mod_archive_to_cache(
    state: &AppState,
    connection: &Connection,
    source_path: &Path,
) -> Result<(), InternalError> {
    let archive_name = source_path
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("imported-mod.zip");
    let archive_name = sanitize_path_segment(archive_name);
    let (sha256, file_size) = compute_sha256_and_size(source_path)?;
    let cache_key = format!("local-{sha256}");
    let relative_path = format!("local/{}-{archive_name}", &sha256[..12.min(sha256.len())]);
    let archive_path = cached_archive_path(state, &relative_path);

    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if archive_path != source_path {
        fs::copy(source_path, &archive_path)?;
    }

    upsert_local_cached_archive(
        connection,
        &cache_key,
        &sha256,
        &archive_name,
        &relative_path,
        file_size,
    )
}

fn compute_sha256_and_size(path: &Path) -> Result<(String, i64), InternalError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut bytes = [0_u8; 8192];
    let mut total_bytes = 0_i64;

    loop {
        let read = file.read(&mut bytes)?;
        if read == 0 {
            break;
        }
        hasher.update(&bytes[..read]);
        total_bytes = total_bytes.saturating_add(read as i64);
    }

    Ok((format!("{:x}", hasher.finalize()), total_bytes))
}

fn sanitize_path_segment(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric()
                || character == '-'
                || character == '_'
                || character == '.'
            {
                character
            } else {
                '_'
            }
        })
        .collect();

    if sanitized.is_empty() {
        "mod".to_string()
    } else {
        sanitized
    }
}

fn dependency_raw_key(package_name: &str, version_number: &str) -> String {
    format!("{package_name}-{version_number}")
}

fn installed_node_key(package_id: &str, version_id: &str) -> String {
    format!("{package_id}:{version_id}")
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

fn resolve_import_profile_name(
    connection: &Connection,
    desired_name: &str,
) -> Result<String, InternalError> {
    let base_name = desired_name.trim();
    if base_name.is_empty() {
        return Ok("Imported profile".to_string());
    }

    let exists = profile_name_exists(connection, base_name)?;
    if !exists {
        return Ok(base_name.to_string());
    }

    for suffix in 2..=999 {
        let candidate = format!("{base_name} ({suffix})");
        if !profile_name_exists(connection, &candidate)? {
            return Ok(candidate);
        }
    }

    Ok(format!(
        "{}-{}",
        base_name,
        OffsetDateTime::now_utc().unix_timestamp()
    ))
}

fn profile_name_exists(connection: &Connection, name: &str) -> Result<bool, InternalError> {
    Ok(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM profiles WHERE name = ?1 COLLATE NOCASE)",
        params![name],
        |row| row.get::<_, i64>(0),
    )? != 0)
}

fn default_enabled_true() -> bool {
    true
}

fn default_source_kind() -> String {
    "thunderstore".to_string()
}

fn default_launch_mode() -> String {
    "steam".to_string()
}

fn default_zip_options() -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644)
}

fn write_zip_json_entry<T: Serialize>(
    writer: &mut ZipWriter<fs::File>,
    entry_path: &str,
    value: &T,
    options: SimpleFileOptions,
) -> Result<(), InternalError> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    write_zip_binary_entry(writer, entry_path, &bytes, options)
}

fn write_zip_text_entry(
    writer: &mut ZipWriter<fs::File>,
    entry_path: &str,
    value: &str,
    options: SimpleFileOptions,
) -> Result<(), InternalError> {
    write_zip_binary_entry(writer, entry_path, value.as_bytes(), options)
}

fn write_zip_binary_entry(
    writer: &mut ZipWriter<fs::File>,
    entry_path: &str,
    bytes: &[u8],
    options: SimpleFileOptions,
) -> Result<(), InternalError> {
    writer.start_file(normalize_archive_path_str(entry_path), options)?;
    writer.write_all(bytes)?;
    Ok(())
}

fn write_zip_directory_entry(
    writer: &mut ZipWriter<fs::File>,
    entry_path: &str,
    options: SimpleFileOptions,
) -> Result<(), InternalError> {
    let mut normalized = normalize_archive_path_str(entry_path);
    if !normalized.ends_with('/') {
        normalized.push('/');
    }
    writer.add_directory(normalized, options)?;
    Ok(())
}

fn normalize_archive_path_str(path: &str) -> String {
    path.replace('\\', "/")
}

fn normalize_archive_path(path: &Path) -> String {
    let mut segments = Vec::<String>::new();
    for component in path.components() {
        if let std::path::Component::Normal(value) = component {
            let segment = value.to_string_lossy();
            if !segment.is_empty() {
                segments.push(segment.to_string());
            }
        }
    }
    segments.join("/")
}

fn copy_path_into_zip(
    writer: &mut ZipWriter<fs::File>,
    source_path: &Path,
    archive_path: &Path,
    options: SimpleFileOptions,
) -> Result<(), InternalError> {
    if source_path.is_dir() {
        write_zip_directory_entry(writer, &normalize_archive_path(archive_path), options)?;

        for entry in fs::read_dir(source_path)? {
            let entry = entry?;
            let entry_path = entry.path();
            let entry_archive_path = archive_path.join(entry.file_name());
            copy_path_into_zip(writer, &entry_path, &entry_archive_path, options)?;
        }

        return Ok(());
    }

    if source_path.is_file() {
        let entry_name = normalize_archive_path(archive_path);
        let mut source_file = fs::File::open(source_path)?;
        writer.start_file(entry_name, options)?;
        io::copy(&mut source_file, writer)?;
    }

    Ok(())
}

fn copy_directory_children_into_zip(
    writer: &mut ZipWriter<fs::File>,
    source_dir: &Path,
    archive_base: &Path,
    options: SimpleFileOptions,
) -> Result<(), InternalError> {
    if !source_dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(source_dir)? {
        let entry = entry?;
        let source_path = entry.path();
        let archive_path = archive_base.join(entry.file_name());
        copy_path_into_zip(writer, &source_path, &archive_path, options)?;
    }

    Ok(())
}

struct ProfilePackPayloadSummary {
    payload_mode: String,
    embedded_mod_count: usize,
    referenced_mod_count: usize,
    has_legacy_runtime_plugins_payload: bool,
}

struct ProfilePackReadDocuments {
    profile_document: ProfilePackProfileDocument,
    mods_lock_document: ProfilePackModsLockDocument,
    payload_summary: ProfilePackPayloadSummary,
}

fn parse_archive_relative_segments(path: &str) -> Option<Vec<String>> {
    let mut segments = Vec::new();
    for segment in normalize_archive_path_str(path).split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return None;
        }
        segments.push(segment.to_string());
    }

    if segments.is_empty() {
        return None;
    }

    Some(segments)
}

fn path_has_prefix(path: &[String], prefix: &[String]) -> bool {
    if path.len() < prefix.len() {
        return false;
    }
    prefix
        .iter()
        .enumerate()
        .all(|(index, segment)| path[index] == *segment)
}

fn summarize_profile_pack_payload(
    archive: &mut ZipArchive<fs::File>,
    mods_lock_document: &ProfilePackModsLockDocument,
) -> Result<ProfilePackPayloadSummary, InternalError> {
    let mut archive_paths = Vec::<Vec<String>>::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let Some(enclosed_path) = entry.enclosed_name().map(|value| value.to_path_buf()) else {
            continue;
        };
        let segments = enclosed_path
            .components()
            .filter_map(|component| match component {
                std::path::Component::Normal(value) => Some(value.to_string_lossy().to_string()),
                _ => None,
            })
            .collect::<Vec<_>>();
        if !segments.is_empty() {
            archive_paths.push(segments);
        }
    }

    let referenced_mods = mods_lock_document
        .mods
        .iter()
        .filter(|entry| !entry.package_id.trim().is_empty() && !entry.version_id.trim().is_empty())
        .collect::<Vec<_>>();
    let referenced_mod_count = referenced_mods.len();

    let mut embedded_mod_count = 0_usize;
    for entry in referenced_mods {
        let Some(install_dir_segments) = parse_archive_relative_segments(&entry.install_dir) else {
            continue;
        };
        if archive_paths
            .iter()
            .any(|path_segments| path_has_prefix(path_segments, &install_dir_segments))
        {
            embedded_mod_count += 1;
        }
    }

    let legacy_plugins_prefix = vec![
        "config".to_string(),
        "BepInEx".to_string(),
        "plugins".to_string(),
    ];
    let has_legacy_runtime_plugins_payload = archive_paths
        .iter()
        .any(|path_segments| path_has_prefix(path_segments, &legacy_plugins_prefix));

    let payload_mode = if referenced_mod_count == 0 || embedded_mod_count == 0 {
        "compact".to_string()
    } else if embedded_mod_count >= referenced_mod_count {
        "full".to_string()
    } else {
        "hybrid".to_string()
    };

    Ok(ProfilePackPayloadSummary {
        payload_mode,
        embedded_mod_count,
        referenced_mod_count,
        has_legacy_runtime_plugins_payload,
    })
}

fn read_pack_documents_from_path(
    source_path: &Path,
) -> Result<ProfilePackReadDocuments, InternalError> {
    let archive_file = fs::File::open(source_path)?;
    let mut archive = ZipArchive::new(archive_file)?;
    let profile_document = read_zip_json_entry::<ProfilePackProfileDocument>(
        &mut archive,
        "profile.json",
        "That .49pack is missing a valid profile.json.",
    )?;
    let mods_lock_document = read_zip_json_entry::<ProfilePackModsLockDocument>(
        &mut archive,
        "mods.lock.json",
        "That .49pack is missing a valid mods.lock.json.",
    )?;
    let payload_summary = summarize_profile_pack_payload(&mut archive, &mods_lock_document)?;
    Ok(ProfilePackReadDocuments {
        profile_document,
        mods_lock_document,
        payload_summary,
    })
}

fn read_zip_json_entry<T: DeserializeOwned>(
    archive: &mut ZipArchive<fs::File>,
    entry_path: &str,
    invalid_message: &'static str,
) -> Result<T, InternalError> {
    let mut entry = archive
        .by_name(entry_path)
        .map_err(|_| InternalError::app("PROFILE_PACK_INVALID", invalid_message.to_string()))?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes)?;
    serde_json::from_slice::<T>(&bytes).map_err(|error| {
        InternalError::with_detail(
            "PROFILE_PACK_INVALID",
            invalid_message.to_string(),
            error.to_string(),
        )
    })
}

fn extract_zip_tree_into_directory(
    archive: &mut ZipArchive<fs::File>,
    source_prefix: &[&str],
    destination_root: &Path,
) -> Result<(), InternalError> {
    fs::create_dir_all(destination_root)?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let Some(enclosed_path) = entry.enclosed_name().map(|value| value.to_path_buf()) else {
            continue;
        };

        let enclosed_segments = enclosed_path
            .components()
            .filter_map(|component| match component {
                std::path::Component::Normal(value) => Some(value.to_string_lossy().to_string()),
                _ => None,
            })
            .collect::<Vec<_>>();

        if enclosed_segments.len() < source_prefix.len()
            || !source_prefix
                .iter()
                .enumerate()
                .all(|(index, segment)| enclosed_segments[index] == *segment)
        {
            continue;
        }

        let relative_segments = &enclosed_segments[source_prefix.len()..];
        if relative_segments.is_empty() {
            continue;
        }

        let output_path = relative_segments
            .iter()
            .fold(destination_root.to_path_buf(), |path, segment| {
                path.join(segment)
            });
        if entry.is_dir() {
            fs::create_dir_all(&output_path)?;
            continue;
        }

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut output_file = fs::File::create(&output_path)?;
        io::copy(&mut entry, &mut output_file)?;
    }

    Ok(())
}

fn open_folder_path(
    folder_path: &Path,
    code: &'static str,
    message: &'static str,
) -> Result<(), InternalError> {
    let mut command = if cfg!(target_os = "windows") {
        let mut command = Command::new("explorer");
        command.arg(folder_path);
        command
    } else if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(folder_path);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(folder_path);
        command
    };

    // Launch the system opener without waiting for the file explorer to close.
    // Reap in a detached thread to avoid leaving a zombie process.
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| InternalError::app(code, message))?;

    std::thread::spawn(move || {
        let _ = child.wait();
    });

    Ok(())
}

fn clear_directory_contents(path: &Path) -> Result<(), InternalError> {
    fs::create_dir_all(path)?;

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();

        if entry_path.is_dir() {
            fs::remove_dir_all(&entry_path)?;
        } else if entry_path.exists() {
            fs::remove_file(&entry_path)?;
        }
    }

    Ok(())
}

fn directory_size_bytes(path: &Path) -> Result<u64, InternalError> {
    if !path.exists() {
        return Ok(0);
    }

    let mut total = 0_u64;

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        let metadata = entry.metadata()?;

        if metadata.is_dir() {
            total = total.saturating_add(directory_size_bytes(&entry_path)?);
        } else if metadata.is_file() {
            total = total.saturating_add(metadata.len());
        }
    }

    Ok(total)
}

pub fn get_active_profile_id(connection: &Connection) -> Result<String, InternalError> {
    let active_profile_id = get_setting(connection, "profiles.active_id")?
        .and_then(|value_json| serde_json::from_str::<String>(&value_json).ok());

    match active_profile_id {
        Some(profile_id) if profile_exists(connection, &profile_id)? => Ok(profile_id),
        _ => {
            upsert_setting(
                connection,
                "profiles.active_id",
                &serde_json::to_string("default")?,
                &now_rfc3339()?,
            )?;
            Ok("default".to_string())
        }
    }
}

fn profile_exists(connection: &Connection, profile_id: &str) -> Result<bool, InternalError> {
    Ok(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM profiles WHERE id = ?1)",
        params![profile_id],
        |row| row.get::<_, i64>(0),
    )? != 0)
}

fn map_profile_summary_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProfileSummaryDto> {
    Ok(ProfileSummaryDto {
        id: row.get(0)?,
        name: row.get(1)?,
        notes: row.get(2)?,
        game_path: row.get(3)?,
        last_played: row.get(4)?,
        launch_mode_default: row.get(5)?,
        installed_count: 0,
        enabled_count: 0,
        is_builtin_default: row.get::<_, i64>(6)? != 0,
        profile_size_bytes: 0,
    })
}

fn map_profile_detail_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProfileDetailDto> {
    Ok(ProfileDetailDto {
        id: row.get(0)?,
        name: row.get(1)?,
        notes: row.get(2)?,
        game_path: row.get(3)?,
        last_played: row.get(4)?,
        launch_mode_default: row.get(5)?,
        is_builtin_default: row.get::<_, i64>(6)? != 0,
        installed_mods: Vec::new(),
    })
}
