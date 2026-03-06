use std::{cmp::Ordering, collections::HashMap};

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use semver::{BuildMetadata, Prerelease, Version};
use serde::{Deserialize, Serialize};

use crate::{
    app_state::AppState,
    db::{get_setting, now_rfc3339, upsert_setting},
    domain::status::{
        classify_base_zone, parse_reference_state, resolve_effective_status, BaseZone,
        EffectiveStatus, ReferenceState,
    },
    error::InternalError,
    services::dependency_service::invalidate_dependency_catalog_index,
    thunderstore::{client::fetch_lethal_company_packages, models::ThunderstorePackage},
};

const LAST_SYNC_SETTING_KEY: &str = "catalog.last_sync_at";
const FRESHNESS_WINDOW_SECONDS: i64 = 900;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncCatalogInput {
    pub force: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncCatalogResult {
    pub outcome: String,
    pub package_count: usize,
    pub version_count: usize,
    pub synced_at: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSummaryDto {
    pub has_catalog: bool,
    pub package_count: usize,
    pub version_count: usize,
    pub last_sync_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPackagesInput {
    pub query: String,
    pub visible_statuses: Vec<EffectiveStatus>,
    pub sort_mode: Option<BrowseSortMode>,
    pub cursor: Option<usize>,
    pub page_size: Option<usize>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum BrowseSortMode {
    MostDownloads,
    Compatibility,
    LastUpdated,
    NameAsc,
    NameDesc,
}

impl BrowseSortMode {
    fn order_clause_sql(self) -> &'static str {
        match self {
            BrowseSortMode::MostDownloads => {
                "rv.total_version_downloads DESC,
            p.total_downloads DESC,
            p.full_name COLLATE NOCASE ASC"
            }
            BrowseSortMode::Compatibility => {
                "CASE rv.effective_status
                WHEN 'verified' THEN 5
                WHEN 'green' THEN 4
                WHEN 'yellow' THEN 3
                WHEN 'orange' THEN 2
                WHEN 'red' THEN 1
                ELSE 0
            END DESC,
            rv.total_version_downloads DESC,
            p.total_downloads DESC,
            p.full_name COLLATE NOCASE ASC"
            }
            BrowseSortMode::LastUpdated => {
                "rv.latest_published_at DESC,
            p.total_downloads DESC,
            p.full_name COLLATE NOCASE ASC"
            }
            BrowseSortMode::NameAsc => {
                "p.full_name COLLATE NOCASE ASC,
            p.total_downloads DESC"
            }
            BrowseSortMode::NameDesc => {
                "p.full_name COLLATE NOCASE DESC,
            p.total_downloads DESC"
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPackagesResult {
    pub items: Vec<PackageCardDto>,
    pub next_cursor: Option<usize>,
    pub has_more: bool,
    pub page_size: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageCardDto {
    pub id: String,
    pub full_name: String,
    pub author: String,
    pub summary: String,
    pub categories: Vec<String>,
    pub total_downloads: i64,
    pub rating: f64,
    pub version_count: usize,
    pub recommended_version_id: String,
    pub recommended_version: String,
    pub icon_url: Option<String>,
    pub effective_status: EffectiveStatus,
    pub every_relevant_version_broken: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageVersionDto {
    pub id: String,
    pub version_number: String,
    pub published_at: String,
    pub icon_url: Option<String>,
    pub downloads: i64,
    pub dependencies: Vec<String>,
    pub base_zone: BaseZone,
    pub bundled_reference_state: Option<ReferenceState>,
    pub bundled_reference_note: Option<String>,
    pub override_reference_state: Option<ReferenceState>,
    pub override_reference_note: Option<String>,
    pub effective_status: EffectiveStatus,
    pub reference_source: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageDetailDto {
    pub id: String,
    pub full_name: String,
    pub author: String,
    pub summary: String,
    pub categories: Vec<String>,
    pub total_downloads: i64,
    pub rating: f64,
    pub website_url: String,
    pub versions: Vec<PackageVersionDto>,
}

#[derive(Debug, Clone)]
pub(crate) struct PackageRecord {
    pub id: String,
    pub full_name: String,
    pub author: String,
    pub summary: String,
    pub categories: Vec<String>,
    pub total_downloads: i64,
    pub rating: f64,
    pub website_url: String,
    pub versions: Vec<PackageVersionDto>,
}

#[derive(Debug, Clone)]
struct RawPackageCardRow {
    id: String,
    full_name: String,
    author: String,
    summary: String,
    categories: Vec<String>,
    total_downloads: i64,
    rating: f64,
    version_count: usize,
    recommended_version_id: String,
    recommended_version: String,
    icon_url: Option<String>,
    effective_status: String,
    every_relevant_version_broken: bool,
}

pub fn sync_catalog(
    state: &AppState,
    input: SyncCatalogInput,
) -> Result<SyncCatalogResult, InternalError> {
    let force = input.force.unwrap_or(false);
    let mut connection = state.connection.lock().map_err(|_| {
        InternalError::app("DB_INIT_FAILED", "Failed to lock the SQLite connection")
    })?;

    if !force && catalog_is_fresh(&connection)? {
        let summary = get_catalog_summary(&connection)?;
        let synced_at = get_last_sync_at(&connection)?;

        return Ok(SyncCatalogResult {
            outcome: "skipped".into(),
            package_count: summary.package_count,
            version_count: summary.version_count,
            synced_at,
            message: summary.last_sync_label,
        });
    }

    let remote_packages = fetch_lethal_company_packages(&state.http_client)?;
    let synced_at = now_rfc3339()?;

    {
        let transaction = connection.transaction()?;
        persist_catalog(&transaction, &remote_packages, state, &synced_at)?;
        transaction.commit()?;
    }

    upsert_setting(
        &connection,
        LAST_SYNC_SETTING_KEY,
        &serde_json::to_string(&synced_at)?,
        &synced_at,
    )?;
    invalidate_dependency_catalog_index(&state.dependency_index_cache)?;

    let summary = get_catalog_summary(&connection)?;

    Ok(SyncCatalogResult {
        outcome: "synced".into(),
        package_count: summary.package_count,
        version_count: summary.version_count,
        synced_at: Some(synced_at),
        message: "Cache refreshed just now".into(),
    })
}

pub fn get_catalog_summary(connection: &Connection) -> Result<CatalogSummaryDto, InternalError> {
    let package_count = connection.query_row("SELECT COUNT(*) FROM packages", [], |row| {
        row.get::<_, i64>(0)
    })? as usize;
    let version_count =
        connection.query_row("SELECT COUNT(*) FROM package_versions", [], |row| {
            row.get::<_, i64>(0)
        })? as usize;
    let last_sync_label = if package_count == 0 {
        "Catalog not synced yet".to_string()
    } else {
        "Cached mod list ready".to_string()
    };

    Ok(CatalogSummaryDto {
        has_catalog: package_count > 0,
        package_count,
        version_count,
        last_sync_label,
    })
}

pub fn search_packages(
    connection: &Connection,
    input: SearchPackagesInput,
) -> Result<SearchPackagesResult, InternalError> {
    let query = input.query.trim().to_lowercase();
    let sort_mode = input.sort_mode.unwrap_or(BrowseSortMode::MostDownloads);
    let page_size = input.page_size.unwrap_or(40).clamp(1, 100);
    let cursor = input.cursor.unwrap_or(0);

    if input.visible_statuses.is_empty() {
        return Ok(SearchPackagesResult {
            items: Vec::new(),
            next_cursor: None,
            has_more: false,
            page_size,
        });
    }

    let sql = format!(
        "WITH version_states AS (
            SELECT
                pv.package_id,
                pv.id AS version_id,
                pv.version_number,
                pv.icon_url,
                pv.published_at,
                pv.downloads,
                pv.base_zone,
                CASE
                    WHEN ro.reference_state = 'broken' THEN 'broken'
                    WHEN ro.reference_state = 'verified' THEN 'verified'
                    WHEN ro.reference_state = 'neutral' THEN pv.base_zone
                    WHEN pv.bundled_reference_state = 'broken' THEN 'broken'
                    WHEN pv.bundled_reference_state = 'verified' THEN 'verified'
                    WHEN pv.bundled_reference_state = 'neutral' THEN pv.base_zone
                    ELSE pv.base_zone
                END AS effective_status
            FROM package_versions pv
            LEFT JOIN reference_overrides ro
                ON ro.package_id = pv.package_id AND ro.version_id = pv.id
        ),
        ranked_versions AS (
            SELECT
                version_states.*,
                ROW_NUMBER() OVER (
                    PARTITION BY package_id
                    ORDER BY
                        CASE effective_status
                            WHEN 'verified' THEN 0
                            WHEN 'green' THEN 1
                            WHEN 'yellow' THEN 2
                            WHEN 'orange' THEN 3
                            WHEN 'broken' THEN 4
                            ELSE 5
                        END ASC,
                        published_at DESC
                ) AS recommended_row,
                COUNT(*) OVER (PARTITION BY package_id) AS version_count,
                MAX(published_at) OVER (PARTITION BY package_id) AS latest_published_at,
                COALESCE(SUM(downloads) OVER (PARTITION BY package_id), 0) AS total_version_downloads,
                SUM(CASE WHEN base_zone != 'red' THEN 1 ELSE 0 END) OVER (PARTITION BY package_id) AS relevant_version_count,
                SUM(CASE WHEN base_zone != 'red' AND effective_status = 'broken' THEN 1 ELSE 0 END) OVER (PARTITION BY package_id) AS relevant_broken_count
            FROM version_states
        )
        SELECT
            p.id,
            p.full_name,
            p.author,
            p.summary,
            p.categories_json,
            rv.total_version_downloads,
            p.rating,
            rv.version_count,
            rv.version_id,
            rv.version_number,
            rv.icon_url,
            rv.effective_status,
            CASE
                WHEN rv.relevant_version_count > 0 AND rv.relevant_version_count = rv.relevant_broken_count THEN 1
                ELSE 0
            END AS every_relevant_version_broken
        FROM packages p
        JOIN ranked_versions rv
            ON rv.package_id = p.id AND rv.recommended_row = 1
        WHERE
            (?1 = '' OR lower(p.full_name || ' ' || p.author || ' ' || p.summary || ' ' || p.categories_json) LIKE '%' || ?1 || '%')
            AND (
                (?2 = 1 AND rv.effective_status = 'verified') OR
                (?3 = 1 AND rv.effective_status = 'green') OR
                (?4 = 1 AND rv.effective_status = 'yellow') OR
                (?5 = 1 AND rv.effective_status = 'orange') OR
                (?6 = 1 AND rv.effective_status = 'red') OR
                (?7 = 1 AND rv.effective_status = 'broken')
            )
        ORDER BY
            {}
        LIMIT ?8 OFFSET ?9",
        sort_mode.order_clause_sql()
    );

    let mut statement = connection.prepare(&sql)?;

    let status_flags = VisibleStatusFlags::from_slice(&input.visible_statuses);
    let rows = statement.query_map(
        params![
            query,
            status_flags.verified,
            status_flags.green,
            status_flags.yellow,
            status_flags.orange,
            status_flags.red,
            status_flags.broken,
            (page_size + 1) as i64,
            cursor as i64,
        ],
        |row| {
            let categories_json: String = row.get(4)?;
            Ok(RawPackageCardRow {
                id: row.get(0)?,
                full_name: row.get(1)?,
                author: row.get(2)?,
                summary: row.get(3)?,
                categories: serde_json::from_str(&categories_json).unwrap_or_default(),
                total_downloads: row.get(5)?,
                rating: row.get(6)?,
                version_count: row.get::<_, i64>(7)? as usize,
                recommended_version_id: row.get(8)?,
                recommended_version: row.get(9)?,
                icon_url: row.get(10)?,
                effective_status: row.get(11)?,
                every_relevant_version_broken: row.get::<_, i64>(12)? == 1,
            })
        },
    )?;

    let mut items = Vec::new();
    for row in rows {
        let raw = row?;
        items.push(PackageCardDto {
            id: raw.id,
            full_name: raw.full_name,
            author: raw.author,
            summary: raw.summary,
            categories: raw.categories,
            total_downloads: raw.total_downloads,
            rating: raw.rating,
            version_count: raw.version_count,
            recommended_version_id: raw.recommended_version_id,
            recommended_version: raw.recommended_version,
            icon_url: raw.icon_url,
            effective_status: parse_effective_status(&raw.effective_status)?,
            every_relevant_version_broken: raw.every_relevant_version_broken,
        });
    }

    apply_recommended_versions(connection, &mut items)?;

    let has_more = items.len() > page_size;
    if has_more {
        items.truncate(page_size);
    }

    Ok(SearchPackagesResult {
        next_cursor: if has_more {
            Some(cursor + items.len())
        } else {
            None
        },
        has_more,
        items,
        page_size,
    })
}

pub fn get_package_detail(
    connection: &Connection,
    package_id: &str,
) -> Result<Option<PackageDetailDto>, InternalError> {
    let package = load_package_record_by_id(connection, package_id)?;

    Ok(package.map(package_record_to_detail))
}

fn package_record_to_detail(package: PackageRecord) -> PackageDetailDto {
    PackageDetailDto {
        id: package.id,
        full_name: package.full_name,
        author: package.author,
        summary: package.summary,
        categories: package.categories,
        total_downloads: package.total_downloads,
        rating: package.rating,
        website_url: package.website_url,
        versions: package.versions,
    }
}

fn load_package_record_by_id(
    connection: &Connection,
    package_id: &str,
) -> Result<Option<PackageRecord>, InternalError> {
    let mut statement = connection.prepare(
        "SELECT id, full_name, author, summary, categories_json, total_downloads, rating, website_url
         FROM packages
         WHERE id = ?1",
    )?;

    let package = statement
        .query_row(params![package_id], |row| {
            let categories_json: String = row.get(4)?;
            Ok(PackageRecord {
                id: row.get(0)?,
                full_name: row.get(1)?,
                author: row.get(2)?,
                summary: row.get(3)?,
                categories: serde_json::from_str(&categories_json).unwrap_or_default(),
                total_downloads: row.get(5)?,
                rating: row.get(6)?,
                website_url: row.get(7)?,
                versions: Vec::new(),
            })
        })
        .optional()?;

    match package {
        Some(mut package) => {
            package.versions = load_versions_for_package(connection, &package.id)?;
            Ok(Some(package))
        }
        None => Ok(None),
    }
}

fn load_versions_for_package(
    connection: &Connection,
    package_id: &str,
) -> Result<Vec<PackageVersionDto>, InternalError> {
    let mut statement = connection.prepare(
        "SELECT pv.id,
                pv.version_number,
                pv.published_at,
                pv.icon_url,
                pv.downloads,
                pv.dependencies_json,
                pv.base_zone,
                pv.bundled_reference_state,
                pv.bundled_reference_note,
                ro.reference_state,
                ro.note
         FROM package_versions pv
         LEFT JOIN reference_overrides ro
           ON ro.package_id = pv.package_id AND ro.version_id = pv.id
         WHERE pv.package_id = ?1
         ORDER BY pv.published_at DESC",
    )?;

    let rows = statement.query_map(params![package_id], |row| {
        let base_zone = parse_base_zone(&row.get::<_, String>(6)?);
        let bundled_reference_state = row
            .get::<_, Option<String>>(7)?
            .as_deref()
            .and_then(parse_reference_state);
        let override_reference_state = row
            .get::<_, Option<String>>(9)?
            .as_deref()
            .and_then(parse_reference_state);
        let effective_status =
            resolve_effective_status(base_zone, bundled_reference_state, override_reference_state);
        let reference_source = if row.get::<_, Option<String>>(9)?.is_some() {
            Some("override".to_string())
        } else if row.get::<_, Option<String>>(7)?.is_some() {
            Some("bundled".to_string())
        } else {
            None
        };

        Ok(PackageVersionDto {
            id: row.get(0)?,
            version_number: row.get(1)?,
            published_at: row.get(2)?,
            icon_url: row.get(3)?,
            downloads: row.get(4)?,
            dependencies: parse_dependency_entries(&row.get::<_, String>(5)?),
            base_zone,
            bundled_reference_state,
            bundled_reference_note: row.get(8)?,
            override_reference_state,
            override_reference_note: row.get(10)?,
            effective_status,
            reference_source,
        })
    })?;

    let mut versions = Vec::new();

    for row in rows {
        versions.push(row?);
    }

    Ok(versions)
}

fn apply_recommended_versions(
    connection: &Connection,
    items: &mut [PackageCardDto],
) -> Result<(), InternalError> {
    let package_ids = items.iter().map(|item| item.id.clone()).collect::<Vec<_>>();
    let versions_by_package = load_versions_for_packages(connection, &package_ids)?;

    for item in items.iter_mut() {
        let Some(versions) = versions_by_package.get(&item.id) else {
            continue;
        };
        let Some(recommended) = pick_recommended_version(versions) else {
            continue;
        };

        item.recommended_version = recommended.version_number.clone();
        item.recommended_version_id = recommended.id.clone();
        item.icon_url = recommended.icon_url.clone();
        item.effective_status = recommended.effective_status;
    }

    Ok(())
}

fn load_versions_for_packages(
    connection: &Connection,
    package_ids: &[String],
) -> Result<HashMap<String, Vec<PackageVersionDto>>, InternalError> {
    if package_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let placeholders = (1..=package_ids.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let query = format!(
        "SELECT pv.package_id,
                pv.id,
                pv.version_number,
                pv.published_at,
                pv.icon_url,
                pv.downloads,
                pv.dependencies_json,
                pv.base_zone,
                pv.bundled_reference_state,
                pv.bundled_reference_note,
                ro.reference_state,
                ro.note
         FROM package_versions pv
         LEFT JOIN reference_overrides ro
           ON ro.package_id = pv.package_id AND ro.version_id = pv.id
         WHERE pv.package_id IN ({placeholders})
         ORDER BY pv.package_id ASC, pv.published_at DESC"
    );
    let mut statement = connection.prepare(&query)?;
    let rows = statement.query_map(rusqlite::params_from_iter(package_ids.iter()), |row| {
        let package_id: String = row.get(0)?;
        let base_zone = parse_base_zone(&row.get::<_, String>(7)?);
        let bundled_reference_state = row
            .get::<_, Option<String>>(8)?
            .as_deref()
            .and_then(parse_reference_state);
        let override_reference_state = row
            .get::<_, Option<String>>(10)?
            .as_deref()
            .and_then(parse_reference_state);
        let effective_status =
            resolve_effective_status(base_zone, bundled_reference_state, override_reference_state);
        let reference_source = if row.get::<_, Option<String>>(10)?.is_some() {
            Some("override".to_string())
        } else if row.get::<_, Option<String>>(8)?.is_some() {
            Some("bundled".to_string())
        } else {
            None
        };

        Ok((
            package_id,
            PackageVersionDto {
                id: row.get(1)?,
                version_number: row.get(2)?,
                published_at: row.get(3)?,
                icon_url: row.get(4)?,
                downloads: row.get(5)?,
                dependencies: parse_dependency_entries(&row.get::<_, String>(6)?),
                base_zone,
                bundled_reference_state,
                bundled_reference_note: row.get(9)?,
                override_reference_state,
                override_reference_note: row.get(11)?,
                effective_status,
                reference_source,
            },
        ))
    })?;

    let mut versions_by_package = HashMap::new();

    for row in rows {
        let (package_id, version) = row?;
        versions_by_package
            .entry(package_id)
            .or_insert_with(Vec::new)
            .push(version);
    }

    Ok(versions_by_package)
}

fn pick_recommended_version(versions: &[PackageVersionDto]) -> Option<&PackageVersionDto> {
    versions
        .iter()
        .max_by(|left, right| compare_recommended_versions(left, right))
}

fn compare_recommended_versions(left: &PackageVersionDto, right: &PackageVersionDto) -> Ordering {
    effective_status_rank(left.effective_status)
        .cmp(&effective_status_rank(right.effective_status))
        .then_with(|| compare_version_number_strings(&left.version_number, &right.version_number))
        .then_with(|| left.published_at.cmp(&right.published_at))
        .then_with(|| left.downloads.cmp(&right.downloads))
        .then_with(|| left.version_number.cmp(&right.version_number))
}

fn effective_status_rank(status: EffectiveStatus) -> i32 {
    match status {
        EffectiveStatus::Verified => 5,
        EffectiveStatus::Green => 4,
        EffectiveStatus::Yellow => 3,
        EffectiveStatus::Orange => 2,
        EffectiveStatus::Broken => 1,
        EffectiveStatus::Red => 0,
    }
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

fn persist_catalog(
    transaction: &Transaction<'_>,
    remote_packages: &[ThunderstorePackage],
    state: &AppState,
    synced_at: &str,
) -> Result<(), InternalError> {
    for package in remote_packages {
        let package_id = normalize_package_id(&package.full_name);
        let categories_json =
            serde_json::to_string(&package.categories.clone().unwrap_or_default())?;
        let summary = package
            .versions
            .iter()
            .max_by(|left, right| left.date_created.cmp(&right.date_created))
            .and_then(|version| version.description.clone())
            .unwrap_or_else(|| "No summary available yet.".to_string());
        let website_url = package
            .package_url
            .clone()
            .unwrap_or_else(|| "https://thunderstore.io".to_string());

        transaction.execute(
            "INSERT INTO packages (id, full_name, author, summary, categories_json, total_downloads, rating, website_url, synced_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
               full_name = excluded.full_name,
               author = excluded.author,
               summary = excluded.summary,
               categories_json = excluded.categories_json,
               total_downloads = excluded.total_downloads,
               rating = excluded.rating,
               website_url = excluded.website_url,
               synced_at = excluded.synced_at",
            params![
                package_id,
                package.full_name,
                package.owner,
                summary,
                categories_json,
                package.total_downloads.unwrap_or_default(),
                package.rating_score.unwrap_or_default(),
                website_url,
                synced_at,
            ],
        )?;

        for version in &package.versions {
            let version_id = normalize_version_id(&package_id, &version.version_number);
            let base_zone =
                classify_base_zone(&version.date_created[..10.min(version.date_created.len())]);
            let bundled_reference = state
                .bundled_references
                .get(&package.full_name, &version.version_number);

            transaction.execute(
                "INSERT INTO package_versions (
                    id, package_id, version_number, published_at, downloads, base_zone,
                    bundled_reference_state, bundled_reference_note, download_url, file_size, icon_url, dependencies_json, sha256
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                 ON CONFLICT(id) DO UPDATE SET
                    package_id = excluded.package_id,
                    version_number = excluded.version_number,
                    published_at = excluded.published_at,
                    downloads = excluded.downloads,
                    base_zone = excluded.base_zone,
                    bundled_reference_state = excluded.bundled_reference_state,
                    bundled_reference_note = excluded.bundled_reference_note,
                    download_url = excluded.download_url,
                    file_size = excluded.file_size,
                    icon_url = excluded.icon_url,
                    dependencies_json = excluded.dependencies_json,
                    sha256 = excluded.sha256",
                params![
                    version_id,
                    package_id,
                    version.version_number,
                    version.date_created[..10.min(version.date_created.len())].to_string(),
                    version.downloads.unwrap_or_default(),
                    format_base_zone(base_zone),
                    bundled_reference.map(|entry| entry.reference_state.clone()),
                    bundled_reference.and_then(|entry| entry.note.clone()),
                    version.download_url,
                    version.file_size,
                    version.icon.clone(),
                    serde_json::to_string(&version.dependencies)?,
                    Option::<String>::None,
                ],
            )?;
        }
    }

    Ok(())
}

fn catalog_is_fresh(connection: &Connection) -> Result<bool, InternalError> {
    let last_sync = match get_last_sync_at(connection)? {
        Some(value) => value,
        None => return Ok(false),
    };

    let parsed =
        time::OffsetDateTime::parse(&last_sync, &time::format_description::well_known::Rfc3339)?;
    let age = time::OffsetDateTime::now_utc() - parsed;

    Ok(age.whole_seconds() < FRESHNESS_WINDOW_SECONDS)
}

fn get_last_sync_at(connection: &Connection) -> Result<Option<String>, InternalError> {
    match get_setting(connection, LAST_SYNC_SETTING_KEY)? {
        Some(value_json) => Ok(Some(serde_json::from_str::<String>(&value_json)?)),
        None => Ok(None),
    }
}

fn parse_base_zone(value: &str) -> BaseZone {
    match value {
        "green" => BaseZone::Green,
        "yellow" => BaseZone::Yellow,
        "red" => BaseZone::Red,
        _ => BaseZone::Orange,
    }
}

fn format_base_zone(zone: BaseZone) -> &'static str {
    match zone {
        BaseZone::Orange => "orange",
        BaseZone::Green => "green",
        BaseZone::Yellow => "yellow",
        BaseZone::Red => "red",
    }
}

fn parse_effective_status(value: &str) -> Result<EffectiveStatus, InternalError> {
    match value {
        "verified" => Ok(EffectiveStatus::Verified),
        "green" => Ok(EffectiveStatus::Green),
        "yellow" => Ok(EffectiveStatus::Yellow),
        "orange" => Ok(EffectiveStatus::Orange),
        "red" => Ok(EffectiveStatus::Red),
        "broken" => Ok(EffectiveStatus::Broken),
        _ => Err(InternalError::app(
            "THUNDERSTORE_RESPONSE_INVALID",
            format!("Unknown effective status value returned from SQLite: {value}"),
        )),
    }
}

struct VisibleStatusFlags {
    verified: i64,
    green: i64,
    yellow: i64,
    orange: i64,
    red: i64,
    broken: i64,
}

impl VisibleStatusFlags {
    fn from_slice(statuses: &[EffectiveStatus]) -> Self {
        Self {
            verified: statuses.contains(&EffectiveStatus::Verified) as i64,
            green: statuses.contains(&EffectiveStatus::Green) as i64,
            yellow: statuses.contains(&EffectiveStatus::Yellow) as i64,
            orange: statuses.contains(&EffectiveStatus::Orange) as i64,
            red: statuses.contains(&EffectiveStatus::Red) as i64,
            broken: statuses.contains(&EffectiveStatus::Broken) as i64,
        }
    }
}

fn normalize_package_id(full_name: &str) -> String {
    match full_name {
        "BepInEx-BepInExPack" => "bepinex-pack".to_string(),
        "notnotnotswipez-MoreCompany" => "more-company".to_string(),
        "2018-LC_API" => "lc-api".to_string(),
        "x753-Mimics" => "mimics".to_string(),
        "Renegades-CoilHeadStare" => "coilhead-stare".to_string(),
        _ => full_name
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .trim_matches('-')
            .to_string(),
    }
}

fn normalize_version_id(package_id: &str, version_number: &str) -> String {
    match (package_id, version_number) {
        ("bepinex-pack", "5.4.2100") => "bepinex-5417".to_string(),
        ("bepinex-pack", "5.4.2200") => "bepinex-5418".to_string(),
        ("more-company", "1.7.6") => "more-176".to_string(),
        ("more-company", "1.7.7") => "more-177".to_string(),
        ("more-company", "1.8.0") => "more-180".to_string(),
        ("lc-api", "3.4.3") => "lcapi-343".to_string(),
        ("lc-api", "3.4.5") => "lcapi-345".to_string(),
        ("lc-api", "3.5.0") => "lcapi-350".to_string(),
        ("mimics", "2.1.0") => "mimics-210".to_string(),
        ("mimics", "2.2.0") => "mimics-220".to_string(),
        ("coilhead-stare", "0.9.1") => "coil-091".to_string(),
        ("coilhead-stare", "0.8.0") => "coil-080".to_string(),
        _ => format!("{package_id}-{}", version_number.replace('.', "")),
    }
}
