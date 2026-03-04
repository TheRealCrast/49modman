use rusqlite::{params, Connection, Transaction};
use serde::{Deserialize, Serialize};

use crate::{
    app_state::AppState,
    db::{get_setting, now_rfc3339, upsert_setting},
    domain::status::{
        browse_status_priority, classify_base_zone, parse_reference_state,
        resolve_effective_status, BaseZone, EffectiveStatus, ReferenceState,
    },
    error::InternalError,
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
    pub recommended_version: String,
    pub effective_status: EffectiveStatus,
    pub every_relevant_version_broken: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageVersionDto {
    pub id: String,
    pub version_number: String,
    pub published_at: String,
    pub downloads: i64,
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
) -> Result<Vec<PackageCardDto>, InternalError> {
    let query = input.query.trim().to_lowercase();
    let visible_statuses = input.visible_statuses;

    let mut cards = load_package_records(connection)?
        .into_iter()
        .filter_map(|package| {
            let recommended = pick_recommended_version(&package.versions)?;
            let effective_status = recommended.effective_status;

            if !visible_statuses
                .iter()
                .any(|status| *status == effective_status)
            {
                return None;
            }

            if !query.is_empty() {
                let haystack = format!(
                    "{} {} {} {}",
                    package.full_name,
                    package.author,
                    package.summary,
                    package.categories.join(" ")
                )
                .to_lowercase();

                if !haystack.contains(&query) {
                    return None;
                }
            }

            Some(PackageCardDto {
                id: package.id,
                full_name: package.full_name,
                author: package.author,
                summary: package.summary,
                categories: package.categories,
                total_downloads: package.total_downloads,
                rating: package.rating,
                version_count: package.versions.len(),
                recommended_version: recommended.version_number.clone(),
                effective_status,
                every_relevant_version_broken: every_relevant_version_broken(&package.versions),
            })
        })
        .collect::<Vec<_>>();

    cards.sort_by(|left, right| {
        browse_status_priority(right.effective_status)
            .cmp(&browse_status_priority(left.effective_status))
            .then(right.total_downloads.cmp(&left.total_downloads))
    });

    Ok(cards)
}

pub fn get_package_detail(
    connection: &Connection,
    package_id: &str,
) -> Result<Option<PackageDetailDto>, InternalError> {
    Ok(load_package_records(connection)?
        .into_iter()
        .find(|package| package.id == package_id)
        .map(package_record_to_detail))
}

pub(crate) fn load_package_records(
    connection: &Connection,
) -> Result<Vec<PackageRecord>, InternalError> {
    let mut statement = connection.prepare(
        "SELECT id, full_name, author, summary, categories_json, total_downloads, rating, website_url
         FROM packages",
    )?;
    let rows = statement.query_map([], |row| {
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
    })?;

    let mut packages = Vec::new();

    for row in rows {
        let mut package = row?;
        package.versions = load_versions_for_package(connection, &package.id)?;
        packages.push(package);
    }

    Ok(packages)
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

fn load_versions_for_package(
    connection: &Connection,
    package_id: &str,
) -> Result<Vec<PackageVersionDto>, InternalError> {
    let mut statement = connection.prepare(
        "SELECT pv.id,
                pv.version_number,
                pv.published_at,
                pv.downloads,
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
        let base_zone = parse_base_zone(&row.get::<_, String>(4)?);
        let bundled_reference_state = row
            .get::<_, Option<String>>(5)?
            .as_deref()
            .and_then(parse_reference_state);
        let override_reference_state = row
            .get::<_, Option<String>>(7)?
            .as_deref()
            .and_then(parse_reference_state);
        let effective_status =
            resolve_effective_status(base_zone, bundled_reference_state, override_reference_state);
        let reference_source = if row.get::<_, Option<String>>(7)?.is_some() {
            Some("override".to_string())
        } else if row.get::<_, Option<String>>(5)?.is_some() {
            Some("bundled".to_string())
        } else {
            None
        };

        Ok(PackageVersionDto {
            id: row.get(0)?,
            version_number: row.get(1)?,
            published_at: row.get(2)?,
            downloads: row.get(3)?,
            base_zone,
            bundled_reference_state,
            bundled_reference_note: row.get(6)?,
            override_reference_state,
            override_reference_note: row.get(8)?,
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
                    bundled_reference_state, bundled_reference_note, download_url, file_size, dependencies_json, sha256
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
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

fn pick_recommended_version(versions: &[PackageVersionDto]) -> Option<&PackageVersionDto> {
    versions
        .iter()
        .find(|version| version.effective_status == EffectiveStatus::Verified)
        .or_else(|| {
            versions
                .iter()
                .filter(|version| {
                    matches!(
                        version.effective_status,
                        EffectiveStatus::Green | EffectiveStatus::Yellow | EffectiveStatus::Orange
                    )
                })
                .max_by(|left, right| {
                    browse_status_priority(left.effective_status)
                        .cmp(&browse_status_priority(right.effective_status))
                        .then(left.published_at.cmp(&right.published_at))
                })
        })
        .or_else(|| {
            versions
                .iter()
                .find(|version| version.effective_status == EffectiveStatus::Broken)
        })
        .or_else(|| versions.first())
}

fn every_relevant_version_broken(versions: &[PackageVersionDto]) -> bool {
    let relevant: Vec<_> = versions
        .iter()
        .filter(|version| version.base_zone != BaseZone::Red)
        .collect();

    !relevant.is_empty()
        && relevant
            .iter()
            .all(|version| version.effective_status == EffectiveStatus::Broken)
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
