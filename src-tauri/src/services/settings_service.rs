use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::{
    db::{get_setting, now_rfc3339, upsert_setting},
    error::InternalError,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WarningPrefsDto {
    pub red: bool,
    pub broken: bool,
    pub install_without_dependencies: bool,
    pub uninstall_with_dependants: bool,
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
    })
}

pub fn set_warning_preference(
    connection: &Connection,
    kind: &str,
    enabled: bool,
) -> Result<WarningPrefsDto, InternalError> {
    if !matches!(
        kind,
        "red" | "broken" | "installWithoutDependencies" | "uninstallWithDependants"
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
        _ => unreachable!("validated above"),
    };
    let updated_at = now_rfc3339()?;
    let value_json = serde_json::to_string(&enabled)?;

    upsert_setting(connection, key, &value_json, &updated_at)?;
    get_warning_prefs(connection)
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
