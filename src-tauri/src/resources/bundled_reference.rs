use std::collections::HashMap;

use serde::Deserialize;

use crate::error::InternalError;

#[derive(Debug, Clone, Deserialize)]
pub struct BundledReferenceFile {
    pub schema_version: u32,
    pub entries: Vec<BundledReferenceEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BundledReferenceEntry {
    pub package_full_name: String,
    pub version_number: String,
    pub reference_state: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BundledReferenceLibrary {
    entries: HashMap<(String, String), BundledReferenceEntry>,
}

impl BundledReferenceLibrary {
    pub fn get(
        &self,
        package_full_name: &str,
        version_number: &str,
    ) -> Option<&BundledReferenceEntry> {
        self.entries
            .get(&(package_full_name.to_string(), version_number.to_string()))
    }
}

pub fn load_bundled_reference_library() -> Result<BundledReferenceLibrary, InternalError> {
    let file: BundledReferenceFile =
        serde_json::from_str(include_str!("../../resources/v49-reference.json"))?;

    if file.schema_version != 1 {
        return Err(InternalError::with_detail(
            "RESOURCE_LOAD_FAILED",
            "Unsupported bundled reference schema version",
            file.schema_version.to_string(),
        ));
    }

    Ok(BundledReferenceLibrary {
        entries: file
            .entries
            .into_iter()
            .map(|entry| {
                (
                    (
                        entry.package_full_name.clone(),
                        entry.version_number.clone(),
                    ),
                    entry,
                )
            })
            .collect(),
    })
}
