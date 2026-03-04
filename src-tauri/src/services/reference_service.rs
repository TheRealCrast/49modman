use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::{
    db::now_rfc3339, error::InternalError, services::catalog_service::load_package_records,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceRowDto {
    pub package_id: String,
    pub package_name: String,
    pub version_id: String,
    pub version_number: String,
    pub published_at: String,
    pub base_zone: crate::domain::status::BaseZone,
    pub effective_status: crate::domain::status::EffectiveStatus,
    pub reference_source: Option<String>,
    pub reference_state: Option<crate::domain::status::ReferenceState>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetReferenceStateInput {
    pub package_id: String,
    pub version_id: String,
    pub reference_state: crate::domain::status::ReferenceState,
}

pub fn list_reference_rows(
    connection: &Connection,
    query: &str,
) -> Result<Vec<ReferenceRowDto>, InternalError> {
    let search = query.trim().to_lowercase();
    let mut rows = load_package_records(connection)?
        .into_iter()
        .flat_map(|package| {
            package
                .versions
                .into_iter()
                .map(move |version| ReferenceRowDto {
                    package_id: package.id.clone(),
                    package_name: package.full_name.clone(),
                    version_id: version.id,
                    version_number: version.version_number,
                    published_at: version.published_at,
                    base_zone: version.base_zone,
                    effective_status: version.effective_status,
                    reference_source: version.reference_source,
                    reference_state: version
                        .override_reference_state
                        .or(version.bundled_reference_state),
                    note: version
                        .override_reference_note
                        .or(version.bundled_reference_note),
                })
        })
        .filter(|row| {
            if search.is_empty() {
                return true;
            }

            format!(
                "{} {} {} {:?}",
                row.package_name,
                row.version_number,
                row.note.clone().unwrap_or_default(),
                row.effective_status
            )
            .to_lowercase()
            .contains(&search)
        })
        .collect::<Vec<_>>();

    rows.sort_by(|left, right| {
        crate::domain::status::browse_status_priority(right.effective_status)
            .cmp(&crate::domain::status::browse_status_priority(
                left.effective_status,
            ))
            .then(right.published_at.cmp(&left.published_at))
    });

    Ok(rows)
}

pub fn set_reference_state(
    connection: &Connection,
    input: SetReferenceStateInput,
) -> Result<ReferenceRowDto, InternalError> {
    let updated_at = now_rfc3339()?;
    let note = match input.reference_state {
        crate::domain::status::ReferenceState::Verified => {
            Some("Locally marked verified from the reference library.".to_string())
        }
        crate::domain::status::ReferenceState::Broken => {
            Some("Locally marked broken from the reference library.".to_string())
        }
        crate::domain::status::ReferenceState::Neutral => None,
    };

    connection.execute(
        "INSERT INTO reference_overrides (package_id, version_id, reference_state, note, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(package_id, version_id) DO UPDATE SET
            reference_state = excluded.reference_state,
            note = excluded.note,
            updated_at = excluded.updated_at",
        params![
            input.package_id,
            input.version_id,
            format_reference_state(input.reference_state),
            note,
            updated_at,
        ],
    )?;

    list_reference_rows(connection, "")?
        .into_iter()
        .find(|row| row.package_id == input.package_id && row.version_id == input.version_id)
        .ok_or_else(|| {
            InternalError::app(
                "REFERENCE_NOT_FOUND",
                "Reference row was not found after update",
            )
        })
}

fn format_reference_state(state: crate::domain::status::ReferenceState) -> &'static str {
    match state {
        crate::domain::status::ReferenceState::Verified => "verified",
        crate::domain::status::ReferenceState::Broken => "broken",
        crate::domain::status::ReferenceState::Neutral => "neutral",
    }
}
