use chrono::{DateTime, Utc};
use failure::err_msg;
use log::info;
use reqwest::header::{HeaderValue, ACCEPT, USER_AGENT};
use semver::Version;
use serde::Deserialize;
use url::Url;

use crate::error::Result;

const APP_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    " ",
    include_str!(concat!(env!("OUT_DIR"), "/git_version"))
);

pub(crate) struct Api {
    api_base: Option<Url>,
    client: reqwest::blocking::Client,
}

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

impl Api {
    pub(super) fn new(api_base: Option<Url>) -> Result<Self> {
        let headers = vec![
            (USER_AGENT, HeaderValue::from_static(APP_USER_AGENT)),
            (ACCEPT, HeaderValue::from_static("application/json")),
        ]
        .into_iter()
        .collect();

        let client = reqwest::blocking::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self { api_base, client })
    }

    fn api_base(&self) -> Result<Url> {
        self.api_base
            .clone()
            .ok_or_else(|| err_msg("index is missing an api base url"))
    }

    pub(crate) fn get_crate_data(&self, name: &str, version: &str) -> RegistryCrateData {
        let (release_time, yanked, downloads) = self
            .get_release_time_yanked_downloads(name, version)
            .unwrap_or_else(|err| {
                info!("Failed to get crate data for {}-{}: {}", name, version, err);
                (Utc::now(), false, 0)
            });

        let owners = self.get_owners(name).unwrap_or_else(|err| {
            info!("Failed to get owners for {}-{}: {}", name, version, err);
            Vec::new()
        });

        RegistryCrateData {
            release_time,
            yanked,
            downloads,
            owners,
        }
    }

    /// Get release_time, yanked and downloads from the registry's API
    fn get_release_time_yanked_downloads(
        &self,
        name: &str,
        version: &str,
    ) -> Result<(DateTime<Utc>, bool, i32)> {
        let url = {
            let mut url = self.api_base()?;
            url.path_segments_mut()
                .map_err(|()| err_msg("Invalid API url"))?
                .extend(&["api", "v1", "crates", name, "versions"]);
            url
        };

        #[derive(Deserialize)]
        struct Response {
            versions: Vec<VersionData>,
        }

        #[derive(Deserialize)]
        struct VersionData {
            num: Version,
            #[serde(default = "Utc::now")]
            created_at: DateTime<Utc>,
            #[serde(default)]
            yanked: bool,
            #[serde(default)]
            downloads: i32,
        }

        let response: Response = self.client.get(url).send()?.error_for_status()?.json()?;

        let version = Version::parse(version)?;
        let version = response
            .versions
            .into_iter()
            .find(|data| data.num == version)
            .ok_or_else(|| err_msg("Could not find version in response"))?;

        Ok((version.created_at, version.yanked, version.downloads))
    }

    /// Fetch owners from the registry's API
    fn get_owners(&self, name: &str) -> Result<Vec<CrateOwner>> {
        let url = {
            let mut url = self.api_base()?;
            url.path_segments_mut()
                .map_err(|()| err_msg("Invalid API url"))?
                .extend(&["api", "v1", "crates", name, "owners"]);
            url
        };

        #[derive(Deserialize)]
        struct Response {
            users: Vec<OwnerData>,
        }

        #[derive(Deserialize)]
        struct OwnerData {
            #[serde(default)]
            avatar: String,
            #[serde(default)]
            email: String,
            #[serde(default)]
            login: String,
            #[serde(default)]
            name: String,
        }

        let response: Response = self.client.get(url).send()?.error_for_status()?.json()?;

        let result = response
            .users
            .into_iter()
            .filter(|data| !data.login.is_empty())
            .map(|data| CrateOwner {
                avatar: data.avatar,
                email: data.email,
                login: data.login,
                name: data.name,
            })
            .collect();

        Ok(result)
    }
}
