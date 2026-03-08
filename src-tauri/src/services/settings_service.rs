use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::{
    db::{get_setting, now_rfc3339, upsert_setting},
    error::InternalError,
};

const ONBOARDING_COMPLETED_KEY: &str = "onboarding.v49.completed";
const ONBOARDING_COMPLETED_AT_KEY: &str = "onboarding.v49.completed_at";
const ONBOARDING_LAST_VALIDATED_GAME_PATH_KEY: &str = "onboarding.v49.last_validated_game_path";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WarningPrefsDto {
    pub red: bool,
    pub broken: bool,
    pub install_without_dependencies: bool,
    pub uninstall_with_dependants: bool,
    pub import_profile_pack: bool,
    pub conserve_while_game_running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingStatusDto {
    pub completed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_validated_game_path: Option<String>,
}

pub fn get_warning_prefs(connection: &Connection) -> Result<WarningPrefsDto, InternalError> {
    Ok(WarningPrefsDto {
        red: get_bool_setting(connection, "warning.red", true)?,
        broken: get_bool_setting(connection, "warning.broken", true)?,
        install_without_dependencies: get_bool_setting(
            connection,
            "warning.install_without_dependencies",
            true,
        )?,
        uninstall_with_dependants: get_bool_setting(
            connection,
            "warning.uninstall_with_dependants",
            true,
        )?,
        import_profile_pack: get_bool_setting(connection, "warning.import_profile_pack", true)?,
        conserve_while_game_running: get_bool_setting(
            connection,
            "launch.conserve_while_game_running",
            false,
        )?,
    })
}

pub fn set_warning_preference(
    connection: &Connection,
    kind: &str,
    enabled: bool,
) -> Result<WarningPrefsDto, InternalError> {
    if !matches!(
        kind,
        "red"
            | "broken"
            | "installWithoutDependencies"
            | "uninstallWithDependants"
            | "importProfilePack"
            | "conserveWhileGameRunning"
    ) {
        return Err(InternalError::app(
            "SETTINGS_SAVE_FAILED",
            format!("Unsupported warning preference: {kind}"),
        ));
    }

    let key = match kind {
        "red" => "warning.red",
        "broken" => "warning.broken",
        "installWithoutDependencies" => "warning.install_without_dependencies",
        "uninstallWithDependants" => "warning.uninstall_with_dependants",
        "importProfilePack" => "warning.import_profile_pack",
        "conserveWhileGameRunning" => "launch.conserve_while_game_running",
        _ => unreachable!("validated above"),
    };
    let updated_at = now_rfc3339()?;
    let value_json = serde_json::to_string(&enabled)?;

    upsert_setting(connection, key, &value_json, &updated_at)?;
    get_warning_prefs(connection)
}

pub fn get_onboarding_status(connection: &Connection) -> Result<OnboardingStatusDto, InternalError> {
    Ok(OnboardingStatusDto {
        completed: get_bool_setting_lossy(connection, ONBOARDING_COMPLETED_KEY, false)?,
        completed_at: get_optional_string_setting_lossy(connection, ONBOARDING_COMPLETED_AT_KEY)?,
        last_validated_game_path: get_optional_string_setting_lossy(
            connection,
            ONBOARDING_LAST_VALIDATED_GAME_PATH_KEY,
        )?,
    })
}

pub fn complete_onboarding(
    connection: &Connection,
    validated_game_path: &str,
) -> Result<OnboardingStatusDto, InternalError> {
    let validated_game_path = validated_game_path.trim();
    if validated_game_path.is_empty() {
        return Err(InternalError::app(
            "SETTINGS_SAVE_FAILED",
            "Validated game path cannot be empty.",
        ));
    }

    let completed_at = now_rfc3339()?;

    upsert_setting(
        connection,
        ONBOARDING_COMPLETED_KEY,
        &serde_json::to_string(&true)?,
        &completed_at,
    )?;
    upsert_setting(
        connection,
        ONBOARDING_COMPLETED_AT_KEY,
        &serde_json::to_string(&completed_at)?,
        &completed_at,
    )?;
    upsert_setting(
        connection,
        ONBOARDING_LAST_VALIDATED_GAME_PATH_KEY,
        &serde_json::to_string(&validated_game_path)?,
        &completed_at,
    )?;

    get_onboarding_status(connection)
}

fn get_bool_setting(
    connection: &Connection,
    key: &str,
    default: bool,
) -> Result<bool, InternalError> {
    match get_setting(connection, key)? {
        Some(value_json) => Ok(serde_json::from_str::<bool>(&value_json)?),
        None => Ok(default),
    }
}

fn get_bool_setting_lossy(
    connection: &Connection,
    key: &str,
    default: bool,
) -> Result<bool, InternalError> {
    match get_setting(connection, key)? {
        Some(value_json) => Ok(serde_json::from_str::<bool>(&value_json).unwrap_or(default)),
        None => Ok(default),
    }
}

fn get_optional_string_setting_lossy(
    connection: &Connection,
    key: &str,
) -> Result<Option<String>, InternalError> {
    match get_setting(connection, key)? {
        Some(value_json) => {
            if let Ok(value) = serde_json::from_str::<String>(&value_json) {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return Ok(None);
                }
                return Ok(Some(trimmed.to_string()));
            }

            if let Ok(value) = serde_json::from_str::<Option<String>>(&value_json) {
                return Ok(value.and_then(|entry| {
                    let trimmed = entry.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                }));
            }

            Ok(None)
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::{
        db::{migrate, now_rfc3339, seed_defaults, upsert_setting},
        error::InternalError,
    };

    use super::{complete_onboarding, get_onboarding_status};

    fn setup_connection() -> Connection {
        let connection = Connection::open_in_memory().expect("in-memory database should open");
        migrate(&connection).expect("migrations should succeed");
        seed_defaults(&connection).expect("seed defaults should succeed");
        connection
    }

    #[test]
    fn onboarding_status_defaults_to_not_completed() {
        let connection = setup_connection();
        let status = get_onboarding_status(&connection).expect("status read should succeed");

        assert!(!status.completed);
        assert!(status.completed_at.is_none());
        assert!(status.last_validated_game_path.is_none());
    }

    #[test]
    fn complete_onboarding_persists_status() {
        let connection = setup_connection();
        let status = complete_onboarding(&connection, "/games/Lethal Company")
            .expect("completion write should succeed");

        assert!(status.completed);
        assert!(status.completed_at.is_some());
        assert_eq!(
            status.last_validated_game_path.as_deref(),
            Some("/games/Lethal Company")
        );

        let reloaded = get_onboarding_status(&connection).expect("status reload should succeed");
        assert!(reloaded.completed);
        assert!(reloaded.completed_at.is_some());
        assert_eq!(
            reloaded.last_validated_game_path.as_deref(),
            Some("/games/Lethal Company")
        );
    }

    #[test]
    fn complete_onboarding_rejects_empty_game_path() {
        let connection = setup_connection();
        let error = complete_onboarding(&connection, "   ")
            .expect_err("empty game path should fail");

        match error {
            InternalError::App { code, message, .. } => {
                assert_eq!(code, "SETTINGS_SAVE_FAILED");
                assert!(
                    message.contains("cannot be empty"),
                    "unexpected error message: {message}"
                );
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn onboarding_status_tolerates_malformed_values() {
        let connection = setup_connection();
        let updated_at = now_rfc3339().expect("timestamp should be generated");

        upsert_setting(
            &connection,
            "onboarding.v49.completed",
            &serde_json::to_string(&"definitely-not-a-bool")
                .expect("value should serialize"),
            &updated_at,
        )
        .expect("completed setting write should succeed");
        upsert_setting(
            &connection,
            "onboarding.v49.completed_at",
            &serde_json::to_string(&1234).expect("value should serialize"),
            &updated_at,
        )
        .expect("completed_at setting write should succeed");
        upsert_setting(
            &connection,
            "onboarding.v49.last_validated_game_path",
            &serde_json::to_string(&true).expect("value should serialize"),
            &updated_at,
        )
        .expect("path setting write should succeed");

        let status = get_onboarding_status(&connection).expect("status read should succeed");
        assert!(!status.completed);
        assert!(status.completed_at.is_none());
        assert!(status.last_validated_game_path.is_none());
    }
}
