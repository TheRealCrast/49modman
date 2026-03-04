use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{
    db::{get_setting, now_rfc3339, reset_user_data, upsert_setting},
    error::InternalError,
};

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
    pub installed_mods: Vec<serde_json::Value>,
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteProfileResult {
    pub deleted_id: String,
    pub next_active_profile_id: Option<String>,
}

pub fn list_profiles(connection: &Connection) -> Result<Vec<ProfileSummaryDto>, InternalError> {
    let mut statement = connection.prepare(
        "SELECT id, name, notes, game_path, last_played_at, launch_mode_default, is_builtin_default
         FROM profiles
         ORDER BY is_builtin_default DESC, CASE WHEN is_builtin_default = 1 THEN 0 ELSE 1 END, updated_at DESC, name COLLATE NOCASE ASC",
    )?;

    let rows = statement.query_map([], map_profile_summary_row)?;

    rows.collect::<Result<Vec<_>, _>>().map_err(InternalError::from)
}

pub fn get_active_profile(connection: &Connection) -> Result<Option<ProfileDetailDto>, InternalError> {
    let profile_id = get_active_profile_id(connection)?;
    match profile_id {
        Some(profile_id) => get_profile_detail(connection, &profile_id),
        None => Ok(None),
    }
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

    let duplicate_exists = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM profiles WHERE name = ?1 COLLATE NOCASE)",
            params![name],
            |row| row.get::<_, i64>(0),
        )?
        != 0;

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

    let profile_id = format!("profile-{}", OffsetDateTime::now_utc().unix_timestamp_nanos());
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

    let duplicate_exists = connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1
                FROM profiles
                WHERE name = ?1 COLLATE NOCASE
                  AND id != ?2
            )",
            params![name, input.profile_id],
            |row| row.get::<_, i64>(0),
        )?
        != 0;

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
        Some(active_id) if active_id == profile_id => {
            upsert_setting(
                connection,
                "profiles.active_id",
                &serde_json::to_string("default")?,
                &now_rfc3339()?,
            )?;
            Some("default".to_string())
        }
        Some(active_id) => Some(active_id),
        None => {
            upsert_setting(
                connection,
                "profiles.active_id",
                &serde_json::to_string("default")?,
                &now_rfc3339()?,
            )?;
            Some("default".to_string())
        }
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

fn get_active_profile_id(connection: &Connection) -> Result<Option<String>, InternalError> {
    let active_profile_id = get_setting(connection, "profiles.active_id")?
        .and_then(|value_json| serde_json::from_str::<String>(&value_json).ok());

    match active_profile_id {
        Some(profile_id) if profile_exists(connection, &profile_id)? => Ok(Some(profile_id)),
        _ => {
            upsert_setting(
                connection,
                "profiles.active_id",
                &serde_json::to_string("default")?,
                &now_rfc3339()?,
            )?;
            Ok(Some("default".to_string()))
        }
    }
}

fn profile_exists(connection: &Connection, profile_id: &str) -> Result<bool, InternalError> {
    Ok(connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM profiles WHERE id = ?1)",
            params![profile_id],
            |row| row.get::<_, i64>(0),
        )?
        != 0)
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
