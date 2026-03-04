use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, OnceLock},
};

use rusqlite::Connection;
use semver::{BuildMetadata, Prerelease, Version};
use serde::{Deserialize, Serialize};

use crate::{
    domain::status::{
        parse_reference_state, resolve_effective_status, EffectiveStatus, ReferenceState,
    },
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
    Repeated,
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
pub struct DependencySummaryItemDto {
    pub package_id: String,
    pub package_name: String,
    pub version_id: String,
    pub version_number: String,
    pub effective_status: EffectiveStatus,
    pub reference_note: Option<String>,
    pub min_depth: usize,
    pub collapsed_version_numbers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnresolvedDependencySummaryItemDto {
    pub raw: String,
    pub min_depth: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencySummaryDto {
    pub direct: Vec<DependencySummaryItemDto>,
    pub transitive: Vec<DependencySummaryItemDto>,
    pub unresolved: Vec<UnresolvedDependencySummaryItemDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionDependenciesDto {
    pub root_package_id: String,
    pub root_package_name: String,
    pub root_version_id: String,
    pub root_version_number: String,
    pub summary: DependencySummaryDto,
    pub tree_items: Vec<DependencyNodeDto>,
}

#[derive(Debug, Clone)]
struct IndexedVersionRecord {
    package_id: String,
    package_name: String,
    version_id: String,
    version_number: String,
    effective_status: EffectiveStatus,
    reference_note: Option<String>,
    dependencies_raw: String,
    dependencies_parsed: OnceLock<Vec<String>>,
}

#[derive(Debug, Default)]
struct DependencyCatalogIndex {
    versions_by_id: HashMap<String, Arc<IndexedVersionRecord>>,
    version_id_by_dependency_raw: HashMap<String, String>,
}

#[derive(Debug, Default)]
pub struct DependencyCatalogIndexCache {
    index: Option<Arc<DependencyCatalogIndex>>,
}

pub type SharedDependencyCatalogIndexCache = Arc<Mutex<DependencyCatalogIndexCache>>;

#[derive(Debug, Default)]
struct SummaryCollector {
    resolved_by_package: HashMap<String, ResolvedSummaryPackageAccumulator>,
    unresolved_by_raw: HashMap<String, UnresolvedDependencySummaryItemDto>,
    resolved_order: Vec<String>,
    unresolved_order: Vec<String>,
    visited_resolved: HashSet<String>,
}

#[derive(Debug, Clone)]
struct ResolvedSummaryPackageAccumulator {
    package_id: String,
    package_name: String,
    version_id: String,
    version_number: String,
    effective_status: EffectiveStatus,
    reference_note: Option<String>,
    min_depth: usize,
    collapsed_version_numbers: HashSet<String>,
}

pub fn get_version_dependencies(
    connection: &Connection,
    cache: &SharedDependencyCatalogIndexCache,
    input: GetVersionDependenciesInput,
) -> Result<VersionDependenciesDto, InternalError> {
    let index = get_or_build_dependency_catalog_index(connection, cache)?;
    let root = index
        .versions_by_id
        .get(&input.version_id)
        .filter(|record| record.package_id == input.package_id)
        .ok_or_else(|| {
            InternalError::app(
                "CATALOG_NOT_FOUND",
                format!(
                    "Selected package version was not found in the cached catalog: {}:{}",
                    input.package_id, input.version_id
                ),
            )
        })?
        .clone();

    let root_key = version_key(&root.package_id, &root.version_id);

    let mut summary_collector = SummaryCollector::default();
    let mut summary_ancestry = HashSet::from([root_key.clone()]);
    for raw in root.dependencies() {
        collect_summary_dependency(
            &index,
            raw,
            1,
            &mut summary_ancestry,
            &mut summary_collector,
        );
    }

    let mut tree_ancestry = HashSet::from([root_key]);
    let mut expanded_versions = HashSet::new();
    let mut tree_items = Vec::with_capacity(root.dependencies().len());
    for raw in root.dependencies() {
        tree_items.push(build_tree_dependency_node(
            &index,
            raw,
            &mut tree_ancestry,
            &mut expanded_versions,
        ));
    }

    Ok(VersionDependenciesDto {
        root_package_id: root.package_id.clone(),
        root_package_name: root.package_name.clone(),
        root_version_id: root.version_id.clone(),
        root_version_number: root.version_number.clone(),
        summary: summary_collector.into_dto(),
        tree_items,
    })
}

pub fn warm_dependency_catalog_index(
    connection: &Connection,
    cache: &SharedDependencyCatalogIndexCache,
) -> Result<(), InternalError> {
    let _ = get_or_build_dependency_catalog_index(connection, cache)?;
    Ok(())
}

pub fn invalidate_dependency_catalog_index(
    cache: &SharedDependencyCatalogIndexCache,
) -> Result<(), InternalError> {
    let mut guard = cache
        .lock()
        .map_err(|_| InternalError::app("DB_INIT_FAILED", "Failed to lock dependency index cache"))?;
    guard.index = None;
    Ok(())
}

pub fn new_dependency_catalog_index_cache() -> SharedDependencyCatalogIndexCache {
    Arc::new(Mutex::new(DependencyCatalogIndexCache::default()))
}

fn get_or_build_dependency_catalog_index(
    connection: &Connection,
    cache: &SharedDependencyCatalogIndexCache,
) -> Result<Arc<DependencyCatalogIndex>, InternalError> {
    if let Some(existing) = cache
        .lock()
        .map_err(|_| InternalError::app("DB_INIT_FAILED", "Failed to lock dependency index cache"))?
        .index
        .clone()
    {
        return Ok(existing);
    }

    let built = Arc::new(load_dependency_catalog_index(connection)?);

    let mut guard = cache
        .lock()
        .map_err(|_| InternalError::app("DB_INIT_FAILED", "Failed to lock dependency index cache"))?;
    if guard.index.is_none() {
        guard.index = Some(built.clone());
    }

    Ok(guard
        .index
        .clone()
        .unwrap_or(built))
}

fn load_dependency_catalog_index(
    connection: &Connection,
) -> Result<DependencyCatalogIndex, InternalError> {
    let mut statement = connection.prepare(
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
           ON ro.package_id = pv.package_id AND ro.version_id = pv.id",
    )?;

    let records = statement.query_map([], |row| {
        let bundled_reference_state = row
            .get::<_, Option<String>>(6)?
            .as_deref()
            .and_then(parse_reference_state);
        let override_reference_state = row
            .get::<_, Option<String>>(8)?
            .as_deref()
            .and_then(parse_reference_state);

        Ok(IndexedVersionRecord {
            package_id: row.get(0)?,
            package_name: row.get(1)?,
            version_id: row.get(2)?,
            version_number: row.get(3)?,
            effective_status: resolve_effective_status(
                parse_base_zone(&row.get::<_, String>(5)?),
                bundled_reference_state,
                override_reference_state,
            ),
            reference_note: resolve_reference_note(
                bundled_reference_state,
                override_reference_state,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(9)?,
            ),
            dependencies_raw: row.get(4)?,
            dependencies_parsed: OnceLock::new(),
        })
    })?;

    let mut index = DependencyCatalogIndex::default();
    for record in records {
        let record = record?;
        let dependency_raw = dependency_raw_key(&record.package_name, &record.version_number);
        index
            .version_id_by_dependency_raw
            .entry(dependency_raw)
            .or_insert_with(|| record.version_id.clone());
        let version_id = record.version_id.clone();
        index.versions_by_id.insert(version_id, Arc::new(record));
    }

    Ok(index)
}

fn collect_summary_dependency(
    index: &DependencyCatalogIndex,
    raw: &str,
    depth: usize,
    ancestry: &mut HashSet<String>,
    collector: &mut SummaryCollector,
) {
    let normalized_raw = raw.trim();
    if normalized_raw.is_empty() {
        collector.record_unresolved(raw.to_string(), depth);
        return;
    }

    let Some(resolved) = resolve_dependency_raw(index, normalized_raw) else {
        collector.record_unresolved(raw.to_string(), depth);
        return;
    };

    let key = version_key(&resolved.package_id, &resolved.version_id);
    if ancestry.contains(&key) {
        return;
    }

    collector.record_resolved(resolved, depth);

    if !collector.visited_resolved.insert(key.clone()) {
        return;
    }

    ancestry.insert(key.clone());
    for child_raw in resolved.dependencies() {
        collect_summary_dependency(index, child_raw, depth + 1, ancestry, collector);
    }
    ancestry.remove(&key);
}

fn build_tree_dependency_node(
    index: &DependencyCatalogIndex,
    raw: &str,
    ancestry: &mut HashSet<String>,
    expanded_versions: &mut HashSet<String>,
) -> DependencyNodeDto {
    let normalized_raw = raw.trim();
    if normalized_raw.is_empty() {
        return unresolved_dependency_node(raw.to_string());
    }

    let Some(resolved) = resolve_dependency_raw(index, normalized_raw) else {
        return unresolved_dependency_node(raw.to_string());
    };

    let key = version_key(&resolved.package_id, &resolved.version_id);
    if ancestry.contains(&key) {
        return resolved_dependency_node(raw.to_string(), resolved, DependencyResolutionKind::Cycle, Vec::new());
    }

    if expanded_versions.contains(&key) {
        return resolved_dependency_node(
            raw.to_string(),
            resolved,
            DependencyResolutionKind::Repeated,
            Vec::new(),
        );
    }

    expanded_versions.insert(key.clone());
    ancestry.insert(key.clone());

    let children = resolved
        .dependencies()
        .iter()
        .map(|child_raw| build_tree_dependency_node(index, child_raw, ancestry, expanded_versions))
        .collect();

    ancestry.remove(&key);

    resolved_dependency_node(raw.to_string(), resolved, DependencyResolutionKind::Resolved, children)
}

fn resolve_dependency_raw<'a>(
    index: &'a DependencyCatalogIndex,
    raw: &str,
) -> Option<&'a IndexedVersionRecord> {
    index
        .version_id_by_dependency_raw
        .get(raw)
        .and_then(|version_id| index.versions_by_id.get(version_id))
        .map(Arc::as_ref)
}

fn resolved_dependency_node(
    raw: String,
    resolved: &IndexedVersionRecord,
    resolution: DependencyResolutionKind,
    children: Vec<DependencyNodeDto>,
) -> DependencyNodeDto {
    DependencyNodeDto {
        raw,
        package_id: Some(resolved.package_id.clone()),
        package_name: Some(resolved.package_name.clone()),
        version_id: Some(resolved.version_id.clone()),
        version_number: Some(resolved.version_number.clone()),
        effective_status: Some(resolved.effective_status),
        reference_note: resolved.reference_note.clone(),
        resolution,
        children,
    }
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

impl IndexedVersionRecord {
    fn dependencies(&self) -> &[String] {
        self.dependencies_parsed
            .get_or_init(|| parse_dependency_entries(&self.dependencies_raw))
            .as_slice()
    }
}

impl SummaryCollector {
    fn record_resolved(&mut self, record: &IndexedVersionRecord, depth: usize) {
        let package_id = record.package_id.clone();
        let entry = self.resolved_by_package.entry(package_id.clone());

        match entry {
            std::collections::hash_map::Entry::Vacant(entry) => {
                self.resolved_order.push(package_id.clone());
                entry.insert(ResolvedSummaryPackageAccumulator {
                    package_id,
                    package_name: record.package_name.clone(),
                    version_id: record.version_id.clone(),
                    version_number: record.version_number.clone(),
                    effective_status: record.effective_status,
                    reference_note: record.reference_note.clone(),
                    min_depth: depth,
                    collapsed_version_numbers: HashSet::new(),
                });
            }
            std::collections::hash_map::Entry::Occupied(mut occupied) => {
                let entry = occupied.get_mut();
                entry.min_depth = entry.min_depth.min(depth);

                match compare_version_number_strings(&record.version_number, &entry.version_number) {
                    Ordering::Greater => {
                        entry
                            .collapsed_version_numbers
                            .insert(entry.version_number.clone());
                        entry.version_id = record.version_id.clone();
                        entry.version_number = record.version_number.clone();
                        entry.effective_status = record.effective_status;
                        entry.reference_note = record.reference_note.clone();
                        entry
                            .collapsed_version_numbers
                            .remove(&entry.version_number);
                    }
                    Ordering::Less => {
                        entry
                            .collapsed_version_numbers
                            .insert(record.version_number.clone());
                    }
                    Ordering::Equal => {}
                }
            }
        }
    }

    fn record_unresolved(&mut self, raw: String, depth: usize) {
        if !self.unresolved_by_raw.contains_key(&raw) {
            self.unresolved_order.push(raw.clone());
            self.unresolved_by_raw.insert(
                raw.clone(),
                UnresolvedDependencySummaryItemDto {
                    raw,
                    min_depth: depth,
                },
            );
            return;
        }

        if let Some(entry) = self.unresolved_by_raw.get_mut(&raw) {
            entry.min_depth = entry.min_depth.min(depth);
        }
    }

    fn into_dto(self) -> DependencySummaryDto {
        let mut direct = Vec::new();
        let mut transitive = Vec::new();

        for package_id in self.resolved_order {
            let Some(entry) = self.resolved_by_package.get(&package_id) else {
                continue;
            };

            let mut collapsed_version_numbers =
                entry.collapsed_version_numbers.iter().cloned().collect::<Vec<_>>();
            collapsed_version_numbers.sort_by(|left, right| {
                compare_version_number_strings(right, left).then_with(|| right.cmp(left))
            });

            let item = DependencySummaryItemDto {
                package_id: entry.package_id.clone(),
                package_name: entry.package_name.clone(),
                version_id: entry.version_id.clone(),
                version_number: entry.version_number.clone(),
                effective_status: entry.effective_status,
                reference_note: entry.reference_note.clone(),
                min_depth: entry.min_depth,
                collapsed_version_numbers,
            };

            if entry.min_depth <= 1 {
                direct.push(item);
            } else {
                transitive.push(item);
            }
        }

        let mut unresolved = Vec::new();
        for raw in self.unresolved_order {
            if let Some(item) = self.unresolved_by_raw.get(&raw) {
                unresolved.push(item.clone());
            }
        }

        DependencySummaryDto {
            direct,
            transitive,
            unresolved,
        }
    }
}

fn resolve_reference_note(
    bundled_reference_state: Option<ReferenceState>,
    override_reference_state: Option<ReferenceState>,
    bundled_reference_note: Option<String>,
    override_reference_note: Option<String>,
) -> Option<String> {
    match override_reference_state {
        Some(ReferenceState::Broken) | Some(ReferenceState::Verified) => override_reference_note,
        Some(ReferenceState::Neutral) => None,
        None => match bundled_reference_state {
            Some(ReferenceState::Broken) | Some(ReferenceState::Verified) => bundled_reference_note,
            _ => None,
        },
    }
}

fn dependency_raw_key(package_name: &str, version_number: &str) -> String {
    format!("{package_name}-{version_number}")
}

fn version_key(package_id: &str, version_id: &str) -> String {
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

fn parse_base_zone(value: &str) -> crate::domain::status::BaseZone {
    match value {
        "green" => crate::domain::status::BaseZone::Green,
        "yellow" => crate::domain::status::BaseZone::Yellow,
        "red" => crate::domain::status::BaseZone::Red,
        _ => crate::domain::status::BaseZone::Orange,
    }
}

fn compare_version_number_strings(left: &str, right: &str) -> Ordering {
    match (parse_semverish(left), parse_semverish(right)) {
        (Some(left), Some(right)) => left.cmp(&right),
        _ => left.cmp(right),
    }
}

fn parse_semverish(value: &str) -> Option<Version> {
    let trimmed = value.trim().trim_start_matches('v');
    let (without_build, build) = match trimmed.split_once('+') {
        Some((core, build)) => (core, Some(build)),
        None => (trimmed, None),
    };
    let (core, prerelease) = match without_build.split_once('-') {
        Some((core, prerelease)) => (core, Some(prerelease)),
        None => (without_build, None),
    };

    let mut components = core.split('.').collect::<Vec<_>>();
    if components.is_empty() || components.len() > 3 {
        return None;
    }

    while components.len() < 3 {
        components.push("0");
    }

    let normalized = components.join(".");
    let mut version = Version::parse(&normalized).ok()?;
    if let Some(prerelease) = prerelease {
        version.pre = Prerelease::new(prerelease).ok()?;
    }
    if let Some(build) = build {
        version.build = BuildMetadata::new(build).ok()?;
    }

    Some(version)
}

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use super::{
        get_version_dependencies, new_dependency_catalog_index_cache, DependencyResolutionKind,
        GetVersionDependenciesInput,
    };
    use crate::{db::migrate, domain::status::EffectiveStatus};

    #[test]
    fn resolves_summary_tree_cycles_and_repeated_nodes() {
        let connection = setup_connection();
        let cache = new_dependency_catalog_index_cache();
        insert_package(&connection, "pack-a", "AuthorA-PackA");
        insert_package(&connection, "pack-b", "AuthorB-PackB");
        insert_package(&connection, "pack-c", "AuthorC-PackC");
        insert_package(&connection, "pack-d", "AuthorD-PackD");

        insert_version(
            &connection,
            "pack-a",
            "pack-a-100",
            "1.0.0",
            "green",
            &["AuthorB-PackB-1.0.0", "AuthorC-PackC-1.0.0", "Missing-Pack-9.9.9"],
            Some(("verified", "Root note")),
            None,
        );
        insert_version(
            &connection,
            "pack-b",
            "pack-b-100",
            "1.0.0",
            "yellow",
            &["AuthorD-PackD-1.0.0"],
            None,
            Some(("broken", "Broken override note")),
        );
        insert_version(
            &connection,
            "pack-c",
            "pack-c-100",
            "1.0.0",
            "orange",
            &["AuthorD-PackD-1.0.0"],
            None,
            None,
        );
        insert_version(
            &connection,
            "pack-d",
            "pack-d-100",
            "1.0.0",
            "green",
            &["AuthorA-PackA-1.0.0"],
            None,
            None,
        );

        let dependencies = get_version_dependencies(
            &connection,
            &cache,
            GetVersionDependenciesInput {
                package_id: "pack-a".to_string(),
                version_id: "pack-a-100".to_string(),
            },
        )
        .expect("dependencies should resolve");

        assert_eq!(dependencies.root_package_id, "pack-a");
        assert_eq!(dependencies.summary.direct.len(), 2);
        assert_eq!(dependencies.summary.transitive.len(), 1);
        assert_eq!(dependencies.summary.unresolved.len(), 1);

        assert_eq!(dependencies.summary.direct[0].package_id, "pack-b");
        assert_eq!(
            dependencies.summary.direct[0].reference_note.as_deref(),
            Some("Broken override note")
        );
        assert_eq!(
            dependencies.summary.direct[0].effective_status,
            EffectiveStatus::Broken
        );
        assert!(dependencies.summary.direct[0]
            .collapsed_version_numbers
            .is_empty());
        assert_eq!(dependencies.summary.direct[1].package_id, "pack-c");
        assert_eq!(dependencies.summary.transitive[0].package_id, "pack-d");
        assert_eq!(dependencies.summary.transitive[0].min_depth, 2);
        assert!(dependencies.summary.transitive[0]
            .collapsed_version_numbers
            .is_empty());
        assert_eq!(dependencies.summary.unresolved[0].raw, "Missing-Pack-9.9.9");

        assert_eq!(dependencies.tree_items.len(), 3);
        let first = &dependencies.tree_items[0];
        assert!(matches!(first.resolution, DependencyResolutionKind::Resolved));
        assert_eq!(first.package_id.as_deref(), Some("pack-b"));
        assert_eq!(first.children.len(), 1);

        let nested = &first.children[0];
        assert!(matches!(nested.resolution, DependencyResolutionKind::Resolved));
        assert_eq!(nested.package_id.as_deref(), Some("pack-d"));

        let cycle = &nested.children[0];
        assert!(matches!(cycle.resolution, DependencyResolutionKind::Cycle));
        assert_eq!(cycle.package_id.as_deref(), Some("pack-a"));

        let repeated = &dependencies.tree_items[1].children[0];
        assert!(matches!(
            repeated.resolution,
            DependencyResolutionKind::Repeated
        ));
        assert_eq!(repeated.package_id.as_deref(), Some("pack-d"));
        assert!(repeated.children.is_empty());
    }

    #[test]
    fn collapses_different_versions_of_same_package_in_summary() {
        let connection = setup_connection();
        let cache = new_dependency_catalog_index_cache();
        insert_package(&connection, "pack-a", "AuthorA-PackA");
        insert_package(&connection, "pack-b", "AuthorB-PackB");
        insert_package(&connection, "pack-c", "AuthorC-PackC");

        insert_version(
            &connection,
            "pack-a",
            "pack-a-100",
            "1.0.0",
            "green",
            &["AuthorB-PackB-1.0.0", "AuthorC-PackC-1.0.0"],
            None,
            None,
        );
        insert_version(
            &connection,
            "pack-b",
            "pack-b-100",
            "1.0.0",
            "green",
            &[],
            None,
            None,
        );
        insert_version(
            &connection,
            "pack-b",
            "pack-b-200",
            "2.0.0",
            "yellow",
            &[],
            None,
            None,
        );
        insert_version(
            &connection,
            "pack-c",
            "pack-c-100",
            "1.0.0",
            "orange",
            &["AuthorB-PackB-2.0.0"],
            None,
            None,
        );

        let dependencies = get_version_dependencies(
            &connection,
            &cache,
            GetVersionDependenciesInput {
                package_id: "pack-a".to_string(),
                version_id: "pack-a-100".to_string(),
            },
        )
        .expect("dependencies should resolve");

        assert_eq!(dependencies.summary.direct.len(), 2);
        assert_eq!(dependencies.summary.transitive.len(), 0);
        assert_eq!(dependencies.summary.direct[0].package_id, "pack-b");
        assert_eq!(dependencies.summary.direct[0].version_id, "pack-b-200");
        assert_eq!(dependencies.summary.direct[0].version_number, "2.0.0");
        assert_eq!(
            dependencies.summary.direct[0].collapsed_version_numbers,
            vec!["1.0.0".to_string()]
        );
    }

    #[test]
    fn returns_not_found_for_missing_root_version() {
        let connection = setup_connection();
        let cache = new_dependency_catalog_index_cache();
        insert_package(&connection, "pack-a", "AuthorA-PackA");

        let error = get_version_dependencies(
            &connection,
            &cache,
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
