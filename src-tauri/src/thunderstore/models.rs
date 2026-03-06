use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ThunderstorePackage {
    pub full_name: String,
    pub owner: String,
    pub categories: Option<Vec<String>>,
    pub rating_score: Option<f64>,
    pub total_downloads: Option<i64>,
    pub package_url: Option<String>,
    pub versions: Vec<ThunderstoreVersion>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ThunderstoreVersion {
    pub version_number: String,
    pub date_created: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub downloads: Option<i64>,
    #[serde(default)]
    pub download_url: String,
    #[serde(default)]
    pub file_size: Option<i64>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
}
