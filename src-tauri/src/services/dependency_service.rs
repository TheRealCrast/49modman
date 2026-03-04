use std::collections::HashSet;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::{
    domain::status::{parse_reference_state, resolve_effective_status, EffectiveStatus},
    error::InternalError,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetVersionDependenciesInput {
    pub package_id: String,
    pub version_id: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyResolutionKind {
    Resolved,
    Unresolved,
    Cycle,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyNodeDto {
    pub raw: String,
    pub package_id: Option<String>,
    pub package_name: Option<String>,
    pub version_id: Option<String>,
    pub version_number: Option<String>,
    pub effective_status: Option<EffectiveStatus>,
    pub reference_note: Option<String>,
    pub resolution: DependencyResolutionKind,
    pub children: Vec<DependencyNodeDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionDependencyTreeDto {
    pub root_package_id: String,
    pub root_package_name: String,
    pub root_version_id: String,
    pub root_version_number: String,
    pub items: Vec<DependencyNodeDto>,
}

#[derive(Debug, Clone)]
struct ResolvedVersionRecord {
    package_id: String,
    package_name: String,
    version_id: String,
    version_number: String,
    effective_status: EffectiveStatus,
    reference_note: Option<String>,
    dependencies: Vec<String>,
}

pub fn get_version_dependencies(
    connection: &Connection,
    input: GetVersionDependenciesInput,
) -> Result<VersionDependencyTreeDto, InternalError> {
    let root = load_version_record_by_ids(connection, &input.package_id, &input.version_id)?
        .ok_or_else(|| {
            InternalError::app(
                "CATALOG_NOT_FOUND",
                format!(
                    "Selected package version was not found in the cached catalog: {}:{}",
                    input.package_id, input.version_id
                ),
            )
        })?;

    let mut ancestry = HashSet::new();
    ancestry.insert(version_key(&root));

    let mut items = Vec::with_capacity(root.dependencies.len());
    for raw in &root.dependencies {
        items.push(resolve_dependency_node(connection, raw, &ancestry)?);
    }

    Ok(VersionDependencyTreeDto {
        root_package_id: root.package_id,
        root_package_name: root.package_name,
        root_version_id: root.version_id,
        root_version_number: root.version_number,
        items,
    })
}

fn resolve_dependency_node(
    connection: &Connection,
    raw: &str,
    ancestry: &HashSet<String>,
) -> Result<DependencyNodeDto, InternalError> {
    let normalized_raw = raw.trim();

    if normalized_raw.is_empty() {
        return Ok(unresolved_dependency_node(raw.to_string()));
    }

    let Some(resolved) = load_version_record_by_dependency_raw(connection, normalized_raw)? else {
        return Ok(unresolved_dependency_node(raw.to_string()));
    };

    let key = version_key(&resolved);
    if ancestry.contains(&key) {
        return Ok(DependencyNodeDto {
            raw: raw.to_string(),
            package_id: Some(resolved.package_id),
            package_name: Some(resolved.package_name),
            version_id: Some(resolved.version_id),
            version_number: Some(resolved.version_number),
            effective_status: Some(resolved.effective_status),
            reference_note: resolved.reference_note,
            resolution: DependencyResolutionKind::Cycle,
            children: Vec::new(),
        });
    }

    let mut next_ancestry = ancestry.clone();
    next_ancestry.insert(key);

    let mut children = Vec::with_capacity(resolved.dependencies.len());
    for child_raw in &resolved.dependencies {
        children.push(resolve_dependency_node(connection, child_raw, &next_ancestry)?);
    }

    Ok(DependencyNodeDto {
        raw: raw.to_string(),
        package_id: Some(resolved.package_id),
        package_name: Some(resolved.package_name),
        version_id: Some(resolved.version_id),
        version_number: Some(resolved.version_number),
        effective_status: Some(resolved.effective_status),
        reference_note: resolved.reference_note,
        resolution: DependencyResolutionKind::Resolved,
        children,
    })
}

fn unresolved_dependency_node(raw: String) -> DependencyNodeDto {
    DependencyNodeDto {
        raw,
        package_id: None,
        package_name: None,
        version_id: None,
        version_number: None,
        effective_status: None,
        reference_note: None,
        resolution: DependencyResolutionKind::Unresolved,
        children: Vec::new(),
    }
}

fn load_version_record_by_ids(
    connection: &Connection,
    package_id: &str,
    version_id: &str,
) -> Result<Option<ResolvedVersionRecord>, InternalError> {
    load_version_record(
        connection,
        "WHERE p.id = ?1 AND pv.id = ?2",
        params![package_id, version_id],
    )
}

fn load_version_record_by_dependency_raw(
    connection: &Connection,
    dependency_raw: &str,
) -> Result<Option<ResolvedVersionRecord>, InternalError> {
    load_version_record(
        connection,
        "WHERE p.full_name || '-' || pv.version_number = ?1",
        params![dependency_raw],
    )
}

fn load_version_record<P>(
    connection: &Connection,
    where_clause: &str,
    params: P,
) -> Result<Option<ResolvedVersionRecord>, InternalError>
where
    P: rusqlite::Params,
{
    let query = format!(
        "SELECT
            p.id,
            p.full_name,
            pv.id,
            pv.version_number,
            pv.dependencies_json,
            pv.base_zone,
            pv.bundled_reference_state,
            pv.bundled_reference_note,
            ro.reference_state,
            ro.note
         FROM packages p
         INNER JOIN package_versions pv
           ON pv.package_id = p.id
         LEFT JOIN reference_overrides ro
           ON ro.package_id = pv.package_id AND ro.version_id = pv.id
         {where_clause}
         LIMIT 1"
    );

    connection
        .query_row(&query, params, |row| {
            let bundled_reference_state = row
                .get::<_, Option<String>>(6)?
                .as_deref()
                .and_then(parse_reference_state);
            let override_reference_state = row
                .get::<_, Option<String>>(8)?
                .as_deref()
                .and_then(parse_reference_state);
            let effective_status = resolve_effective_status(
                parse_base_zone(&row.get::<_, String>(5)?),
                bundled_reference_state,
                override_reference_state,
            );
            let reference_note = match override_reference_state {
                Some(crate::domain::status::ReferenceState::Broken)
                | Some(crate::domain::status::ReferenceState::Verified) => {
                    row.get::<_, Option<String>>(9)?
                }
                Some(crate::domain::status::ReferenceState::Neutral) => None,
                None => match bundled_reference_state {
                    Some(crate::domain::status::ReferenceState::Broken)
                    | Some(crate::domain::status::ReferenceState::Verified) => {
                        row.get::<_, Option<String>>(7)?
                    }
                    _ => None,
                },
            };

            Ok(ResolvedVersionRecord {
                package_id: row.get(0)?,
                package_name: row.get(1)?,
                version_id: row.get(2)?,
                version_number: row.get(3)?,
                effective_status,
                reference_note,
                dependencies: parse_dependency_entries(&row.get::<_, String>(4)?),
            })
        })
        .optional()
        .map_err(InternalError::from)
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

fn parse_base_zone(value: &str) -> crate::domain::status::BaseZone {
    match value {
        "green" => crate::domain::status::BaseZone::Green,
        "yellow" => crate::domain::status::BaseZone::Yellow,
        "red" => crate::domain::status::BaseZone::Red,
        _ => crate::domain::status::BaseZone::Orange,
    }
}

fn version_key(record: &ResolvedVersionRecord) -> String {
    format!("{}:{}", record.package_id, record.version_id)
}

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use super::{get_version_dependencies, DependencyResolutionKind, GetVersionDependenciesInput};
    use crate::{db::migrate, domain::status::EffectiveStatus};

    #[test]
    fn resolves_nested_dependencies_and_cycles() {
        let connection = setup_connection();
        insert_package(&connection, "pack-a", "AuthorA-PackA");
        insert_package(&connection, "pack-b", "AuthorB-PackB");
        insert_package(&connection, "pack-c", "AuthorC-PackC");

        insert_version(
            &connection,
            "pack-a",
            "pack-a-100",
            "1.0.0",
            "green",
            &["AuthorB-PackB-1.0.0", "Missing-Pack-9.9.9"],
            Some(("verified", "Root note")),
            None,
        );
        insert_version(
            &connection,
            "pack-b",
            "pack-b-100",
            "1.0.0",
            "yellow",
            &["AuthorC-PackC-1.0.0"],
            None,
            Some(("broken", "Broken override note")),
        );
        insert_version(
            &connection,
            "pack-c",
            "pack-c-100",
            "1.0.0",
            "orange",
            &["AuthorA-PackA-1.0.0"],
            None,
            None,
        );

        let tree = get_version_dependencies(
            &connection,
            GetVersionDependenciesInput {
                package_id: "pack-a".to_string(),
                version_id: "pack-a-100".to_string(),
            },
        )
        .expect("dependency tree should resolve");

        assert_eq!(tree.root_package_id, "pack-a");
        assert_eq!(tree.items.len(), 2);

        let first = &tree.items[0];
        assert!(matches!(first.resolution, DependencyResolutionKind::Resolved));
        assert_eq!(first.package_id.as_deref(), Some("pack-b"));
        assert_eq!(first.reference_note.as_deref(), Some("Broken override note"));
        assert_eq!(first.children.len(), 1);
        assert_eq!(first.effective_status, Some(EffectiveStatus::Broken));

        let cycle = &first.children[0].children[0];
        assert!(matches!(cycle.resolution, DependencyResolutionKind::Cycle));
        assert_eq!(cycle.package_id.as_deref(), Some("pack-a"));

        let unresolved = &tree.items[1];
        assert!(matches!(
            unresolved.resolution,
            DependencyResolutionKind::Unresolved
        ));
        assert_eq!(unresolved.raw, "Missing-Pack-9.9.9");
    }

    #[test]
    fn returns_not_found_for_missing_root_version() {
        let connection = setup_connection();
        insert_package(&connection, "pack-a", "AuthorA-PackA");

        let error = get_version_dependencies(
            &connection,
            GetVersionDependenciesInput {
                package_id: "pack-a".to_string(),
                version_id: "missing".to_string(),
            },
        )
        .expect_err("missing version should fail");

        assert_eq!(error.to_app_error().code, "CATALOG_NOT_FOUND");
    }

    fn setup_connection() -> Connection {
        let connection = Connection::open_in_memory().expect("in-memory sqlite");
        migrate(&connection).expect("migrations should apply");
        connection
    }

    fn insert_package(connection: &Connection, id: &str, full_name: &str) {
        connection
            .execute(
                "INSERT INTO packages (
                    id, full_name, author, summary, categories_json, total_downloads, rating, website_url, synced_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    id,
                    full_name,
                    full_name.split('-').next().unwrap_or(full_name),
                    "summary",
                    "[]",
                    0_i64,
                    0.0_f64,
                    "https://example.invalid",
                    "2026-03-04T00:00:00Z"
                ],
            )
            .expect("package insert");
    }

    fn insert_version(
        connection: &Connection,
        package_id: &str,
        version_id: &str,
        version_number: &str,
        base_zone: &str,
        dependencies: &[&str],
        bundled_reference: Option<(&str, &str)>,
        override_reference: Option<(&str, &str)>,
    ) {
        connection
            .execute(
                "INSERT INTO package_versions (
                    id, package_id, version_number, published_at, downloads, base_zone,
                    bundled_reference_state, bundled_reference_note, download_url, file_size, dependencies_json, sha256
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    version_id,
                    package_id,
                    version_number,
                    "2024-01-01",
                    0_i64,
                    base_zone,
                    bundled_reference.map(|(state, _)| state.to_string()),
                    bundled_reference.map(|(_, note)| note.to_string()),
                    "https://example.invalid/archive.zip",
                    0_i64,
                    serde_json::to_string(dependencies).expect("dependencies json"),
                    Option::<String>::None
                ],
            )
            .expect("version insert");

        if let Some((reference_state, note)) = override_reference {
            connection
                .execute(
                    "INSERT INTO reference_overrides (package_id, version_id, reference_state, note, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        package_id,
                        version_id,
                        reference_state,
                        note,
                        "2026-03-04T00:00:00Z"
                    ],
                )
                .expect("override insert");
        }
    }
}
