use std::io::Read;

use chrono::{DateTime, Utc};
use failure::err_msg;
use reqwest::{header::ACCEPT, Client};
use semver::Version;
use serde_json::Value;

use crate::error::Result;

pub(crate) struct RegistryCrateData {
    pub(crate) release_time: DateTime<Utc>,
    pub(crate) yanked: bool,
    pub(crate) downloads: i32,
    pub(crate) owners: Vec<CrateOwner>,
}

pub(crate) struct CrateOwner {
    pub(crate) avatar: String,
    pub(crate) email: String,
    pub(crate) login: String,
    pub(crate) name: String,
}

impl RegistryCrateData {
    pub(crate) fn get_from_network(name: &str, version: &str) -> Result<Self> {
        let (release_time, yanked, downloads) = get_release_time_yanked_downloads(name, version)?;
        let owners = get_owners(name)?;

        Ok(Self {
            release_time,
            yanked,
            downloads,
            owners,
        })
    }
}

/// Get release_time, yanked and downloads from the registry's API
fn get_release_time_yanked_downloads(
    name: &str,
    version: &str,
) -> Result<(DateTime<Utc>, bool, i32)> {
    let url = format!("https://crates.io/api/v1/crates/{}/versions", name);
    // FIXME: There is probably better way to do this
    //        and so many unwraps...
    let client = Client::new();
    let mut res = client.get(&url).header(ACCEPT, "application/json").send()?;

    let mut body = String::new();
    res.read_to_string(&mut body)?;

    let json: Value = serde_json::from_str(&body)?;
    let versions = json
        .as_object()
        .and_then(|o| o.get("versions"))
        .and_then(|v| v.as_array())
        .ok_or_else(|| err_msg("Not a JSON object"))?;

    let (mut release_time, mut yanked, mut downloads) = (None, None, None);

    for version_data in versions {
        let version_data = version_data
            .as_object()
            .ok_or_else(|| err_msg("Not a JSON object"))?;
        let version_num = version_data
            .get("num")
            .and_then(|v| v.as_str())
            .ok_or_else(|| err_msg("Not a JSON object"))?;

        if Version::parse(version_num)?.to_string() == version {
            let release_time_raw = version_data
                .get("created_at")
                .and_then(|c| c.as_str())
                .ok_or_else(|| err_msg("Not a JSON object"))?;

            release_time = Some(
                DateTime::parse_from_str(release_time_raw, "%Y-%m-%dT%H:%M:%S%.f%:z")?
                    .with_timezone(&Utc),
            );

            yanked = Some(
                version_data
                    .get("yanked")
                    .and_then(|c| c.as_bool())
                    .ok_or_else(|| err_msg("Not a JSON object"))?,
            );

            downloads = Some(
                version_data
                    .get("downloads")
                    .and_then(|c| c.as_i64())
                    .ok_or_else(|| err_msg("Not a JSON object"))? as i32,
            );

            break;
        }
    }

    Ok((
        release_time.unwrap_or_else(Utc::now),
        yanked.unwrap_or(false),
        downloads.unwrap_or(0),
    ))
}

/// Fetch owners from the registry's API
fn get_owners(name: &str) -> Result<Vec<CrateOwner>> {
    // owners available in: https://crates.io/api/v1/crates/rand/owners
    let owners_url = format!("https://crates.io/api/v1/crates/{}/owners", name);
    let client = Client::new();
    let mut res = client
        .get(&owners_url[..])
        .header(ACCEPT, "application/json")
        .send()?;
    // FIXME: There is probably better way to do this
    //        and so many unwraps...
    let mut body = String::new();
    res.read_to_string(&mut body).unwrap();
    let json: Value = serde_json::from_str(&body[..])?;

    let owners = json
        .as_object()
        .and_then(|j| j.get("users"))
        .and_then(|j| j.as_array());

    let result = if let Some(owners) = owners {
        owners
            .iter()
            .filter_map(|owner| {
                fn extract<'a>(owner: &'a Value, field: &str) -> &'a str {
                    owner
                        .as_object()
                        .and_then(|o| o.get(field))
                        .and_then(|o| o.as_str())
                        .unwrap_or_default()
                }

                let avatar = extract(owner, "avatar");
                let email = extract(owner, "email");
                let login = extract(owner, "login");
                let name = extract(owner, "name");

                if login.is_empty() {
                    return None;
                }

                Some(CrateOwner {
                    avatar: avatar.to_string(),
                    email: email.to_string(),
                    login: login.to_string(),
                    name: name.to_string(),
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    Ok(result)
}
