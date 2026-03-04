use reqwest::blocking::Client;

use crate::{error::InternalError, thunderstore::models::ThunderstorePackage};

const LETHAL_COMPANY_PACKAGE_ENDPOINT: &str =
    "https://thunderstore.io/c/lethal-company/api/v1/package/";

pub fn fetch_lethal_company_packages(
    client: &Client,
) -> Result<Vec<ThunderstorePackage>, InternalError> {
    let response = client
        .get(LETHAL_COMPANY_PACKAGE_ENDPOINT)
        .send()?
        .error_for_status()?;

    Ok(response.json::<Vec<ThunderstorePackage>>()?)
}
