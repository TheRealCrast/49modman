use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::{db::now_rfc3339, error::InternalError};

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
pub struct ListReferenceRowsInput {
    pub query: String,
    pub cursor: Option<usize>,
    pub page_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListReferenceRowsResult {
    pub items: Vec<ReferenceRowDto>,
    pub next_cursor: Option<usize>,
    pub has_more: bool,
    pub page_size: usize,
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
    input: ListReferenceRowsInput,
) -> Result<ListReferenceRowsResult, InternalError> {
    let search = input.query.trim().to_lowercase();
    let page_size = input.page_size.unwrap_or(50).clamp(1, 100);
    let cursor = input.cursor.unwrap_or(0);
    let mut statement = connection.prepare(
        "SELECT
            p.id,
            p.full_name,
            pv.id,
            pv.version_number,
            pv.published_at,
            pv.base_zone,
            CASE
                WHEN ro.reference_state = 'broken' THEN 'broken'
                WHEN ro.reference_state = 'verified' THEN 'verified'
                WHEN ro.reference_state = 'neutral' THEN pv.base_zone
                WHEN pv.bundled_reference_state = 'broken' THEN 'broken'
                WHEN pv.bundled_reference_state = 'verified' THEN 'verified'
                WHEN pv.bundled_reference_state = 'neutral' THEN pv.base_zone
                ELSE pv.base_zone
            END AS effective_status,
            CASE
                WHEN ro.reference_state IS NOT NULL THEN 'override'
                WHEN pv.bundled_reference_state IS NOT NULL THEN 'bundled'
                ELSE NULL
            END AS reference_source,
            CASE
                WHEN ro.reference_state = 'neutral' THEN NULL
                ELSE COALESCE(ro.reference_state, pv.bundled_reference_state)
            END AS reference_state,
            CASE
                WHEN ro.reference_state = 'neutral' THEN NULL
                ELSE COALESCE(ro.note, pv.bundled_reference_note)
            END AS note
         FROM package_versions pv
         JOIN packages p
           ON p.id = pv.package_id
         LEFT JOIN reference_overrides ro
           ON ro.package_id = pv.package_id AND ro.version_id = pv.id
         WHERE
           (?1 = '' OR lower(
             p.full_name || ' ' ||
             pv.version_number || ' ' ||
             COALESCE(ro.note, pv.bundled_reference_note, '') || ' ' ||
             CASE
               WHEN ro.reference_state = 'broken' THEN 'broken'
               WHEN ro.reference_state = 'verified' THEN 'verified'
               WHEN ro.reference_state = 'neutral' THEN pv.base_zone
               WHEN pv.bundled_reference_state = 'broken' THEN 'broken'
               WHEN pv.bundled_reference_state = 'verified' THEN 'verified'
               WHEN pv.bundled_reference_state = 'neutral' THEN pv.base_zone
               ELSE pv.base_zone
             END
           ) LIKE '%' || ?1 || '%')
         ORDER BY
           CASE
             WHEN
               CASE
                 WHEN ro.reference_state = 'broken' THEN 'broken'
                 WHEN ro.reference_state = 'verified' THEN 'verified'
                 WHEN ro.reference_state = 'neutral' THEN pv.base_zone
                 WHEN pv.bundled_reference_state = 'broken' THEN 'broken'
                 WHEN pv.bundled_reference_state = 'verified' THEN 'verified'
                 WHEN pv.bundled_reference_state = 'neutral' THEN pv.base_zone
                 ELSE pv.base_zone
               END = 'verified' THEN 5
             WHEN
               CASE
                 WHEN ro.reference_state = 'broken' THEN 'broken'
                 WHEN ro.reference_state = 'verified' THEN 'verified'
                 WHEN ro.reference_state = 'neutral' THEN pv.base_zone
                 WHEN pv.bundled_reference_state = 'broken' THEN 'broken'
                 WHEN pv.bundled_reference_state = 'verified' THEN 'verified'
                 WHEN pv.bundled_reference_state = 'neutral' THEN pv.base_zone
                 ELSE pv.base_zone
               END = 'green' THEN 4
             WHEN
               CASE
                 WHEN ro.reference_state = 'broken' THEN 'broken'
                 WHEN ro.reference_state = 'verified' THEN 'verified'
                 WHEN ro.reference_state = 'neutral' THEN pv.base_zone
                 WHEN pv.bundled_reference_state = 'broken' THEN 'broken'
                 WHEN pv.bundled_reference_state = 'verified' THEN 'verified'
                 WHEN pv.bundled_reference_state = 'neutral' THEN pv.base_zone
                 ELSE pv.base_zone
               END = 'yellow' THEN 3
             WHEN
               CASE
                 WHEN ro.reference_state = 'broken' THEN 'broken'
                 WHEN ro.reference_state = 'verified' THEN 'verified'
                 WHEN ro.reference_state = 'neutral' THEN pv.base_zone
                 WHEN pv.bundled_reference_state = 'broken' THEN 'broken'
                 WHEN pv.bundled_reference_state = 'verified' THEN 'verified'
                 WHEN pv.bundled_reference_state = 'neutral' THEN pv.base_zone
                 ELSE pv.base_zone
               END = 'orange' THEN 2
             WHEN
               CASE
                 WHEN ro.reference_state = 'broken' THEN 'broken'
                 WHEN ro.reference_state = 'verified' THEN 'verified'
                 WHEN ro.reference_state = 'neutral' THEN pv.base_zone
                 WHEN pv.bundled_reference_state = 'broken' THEN 'broken'
                 WHEN pv.bundled_reference_state = 'verified' THEN 'verified'
                 WHEN pv.bundled_reference_state = 'neutral' THEN pv.base_zone
                 ELSE pv.base_zone
               END = 'red' THEN 1
             ELSE 0
           END DESC,
           pv.published_at DESC
         LIMIT ?2 OFFSET ?3
        ",
    )?;

    let mapped_rows = statement.query_map(params![search, (page_size + 1) as i64, cursor as i64], |row| {
        Ok(ReferenceRowDto {
            package_id: row.get(0)?,
            package_name: row.get(1)?,
            version_id: row.get(2)?,
            version_number: row.get(3)?,
            published_at: row.get(4)?,
            base_zone: parse_base_zone(&row.get::<_, String>(5)?),
            effective_status: parse_effective_status(&row.get::<_, String>(6)?),
            reference_source: row.get(7)?,
            reference_state: row
                .get::<_, Option<String>>(8)?
                .as_deref()
                .and_then(parse_reference_state),
            note: row.get(9)?,
        })
    })?;

    let mut rows = Vec::new();
    for row in mapped_rows {
        rows.push(row?);
    }

    let has_more = rows.len() > page_size;
    if has_more {
        rows.truncate(page_size);
    }

    Ok(ListReferenceRowsResult {
        items: rows,
        next_cursor: if has_more { Some(cursor + page_size) } else { None },
        has_more,
        page_size,
    })
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

    get_reference_row(connection, &input.package_id, &input.version_id)?
        .ok_or_else(|| InternalError::app("REFERENCE_NOT_FOUND", "Reference row was not found after update"))
}

fn format_reference_state(state: crate::domain::status::ReferenceState) -> &'static str {
    match state {
        crate::domain::status::ReferenceState::Verified => "verified",
        crate::domain::status::ReferenceState::Broken => "broken",
        crate::domain::status::ReferenceState::Neutral => "neutral",
    }
}

fn get_reference_row(
    connection: &Connection,
    package_id: &str,
    version_id: &str,
) -> Result<Option<ReferenceRowDto>, InternalError> {
    let mut statement = connection.prepare(
        "SELECT
            p.id,
            p.full_name,
            pv.id,
            pv.version_number,
            pv.published_at,
            pv.base_zone,
            CASE
                WHEN ro.reference_state = 'broken' THEN 'broken'
                WHEN ro.reference_state = 'verified' THEN 'verified'
                WHEN ro.reference_state = 'neutral' THEN pv.base_zone
                WHEN pv.bundled_reference_state = 'broken' THEN 'broken'
                WHEN pv.bundled_reference_state = 'verified' THEN 'verified'
                WHEN pv.bundled_reference_state = 'neutral' THEN pv.base_zone
                ELSE pv.base_zone
            END AS effective_status,
            CASE
                WHEN ro.reference_state IS NOT NULL THEN 'override'
                WHEN pv.bundled_reference_state IS NOT NULL THEN 'bundled'
                ELSE NULL
            END AS reference_source,
            CASE
                WHEN ro.reference_state = 'neutral' THEN NULL
                ELSE COALESCE(ro.reference_state, pv.bundled_reference_state)
            END AS reference_state,
            CASE
                WHEN ro.reference_state = 'neutral' THEN NULL
                ELSE COALESCE(ro.note, pv.bundled_reference_note)
            END AS note
         FROM package_versions pv
         JOIN packages p
           ON p.id = pv.package_id
         LEFT JOIN reference_overrides ro
           ON ro.package_id = pv.package_id AND ro.version_id = pv.id
         WHERE pv.package_id = ?1 AND pv.id = ?2",
    )?;

    let row = statement
        .query_row(params![package_id, version_id], |row| {
            Ok(ReferenceRowDto {
                package_id: row.get(0)?,
                package_name: row.get(1)?,
                version_id: row.get(2)?,
                version_number: row.get(3)?,
                published_at: row.get(4)?,
                base_zone: parse_base_zone(&row.get::<_, String>(5)?),
                effective_status: parse_effective_status(&row.get::<_, String>(6)?),
                reference_source: row.get(7)?,
                reference_state: row
                    .get::<_, Option<String>>(8)?
                    .as_deref()
                    .and_then(parse_reference_state),
                note: row.get(9)?,
            })
        })
        .optional()?;

    Ok(row)
}

fn parse_base_zone(value: &str) -> crate::domain::status::BaseZone {
    match value {
        "green" => crate::domain::status::BaseZone::Green,
        "yellow" => crate::domain::status::BaseZone::Yellow,
        "red" => crate::domain::status::BaseZone::Red,
        _ => crate::domain::status::BaseZone::Orange,
    }
}

fn parse_effective_status(value: &str) -> crate::domain::status::EffectiveStatus {
    match value {
        "verified" => crate::domain::status::EffectiveStatus::Verified,
        "green" => crate::domain::status::EffectiveStatus::Green,
        "yellow" => crate::domain::status::EffectiveStatus::Yellow,
        "orange" => crate::domain::status::EffectiveStatus::Orange,
        "red" => crate::domain::status::EffectiveStatus::Red,
        _ => crate::domain::status::EffectiveStatus::Broken,
    }
}

fn parse_reference_state(value: &str) -> Option<crate::domain::status::ReferenceState> {
    match value {
        "verified" => Some(crate::domain::status::ReferenceState::Verified),
        "broken" => Some(crate::domain::status::ReferenceState::Broken),
        "neutral" => Some(crate::domain::status::ReferenceState::Neutral),
        _ => None,
    }
}
