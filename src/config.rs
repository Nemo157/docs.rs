use anyhow::{anyhow, Context};
use std::error::Error;
use std::env::VarError;
use std::str::FromStr;

#[derive(Debug)]
pub struct Config {
    // Database connection params
    pub(crate) database_url: String,
    pub(crate) max_pool_size: u32,
    pub(crate) min_pool_idle: u32,

    // Max size of the files served by the docs.rs frontend
    pub(crate) max_file_size: usize,
    pub(crate) max_file_size_html: usize,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: require_env("CRATESFYI_DATABASE_URL")?,
            max_pool_size: env("DOCSRS_MAX_POOL_SIZE", 90)?,
            min_pool_idle: env("DOCSRS_MIN_POOL_IDLE", 10)?,

            max_file_size: env("DOCSRS_MAX_FILE_SIZE", 50 * 1024 * 1024)?,
            max_file_size_html: env("DOCSRS_MAX_FILE_SIZE_HTML", 5 * 1024 * 1024)?,
        })
    }
}

fn env<T>(var: &str, default: T) -> anyhow::Result<T>
where
    T: FromStr,
    T::Err: Error + Send + Sync + 'static,
{
    Ok(maybe_env(var)?.unwrap_or(default))
}

fn require_env<T>(var: &str) -> anyhow::Result<T>
where
    T: FromStr,
    T::Err: Error + Send + Sync + 'static,
{
    maybe_env(var)?.with_context(|| anyhow!("configuration variable {} is missing", var))
}

fn maybe_env<T>(var: &str) -> anyhow::Result<Option<T>>
where
    T: FromStr,
    T::Err: Error + Send + Sync + 'static,
{
    match std::env::var(var) {
        Ok(content) => Ok(content
            .parse::<T>()
            .map(Some)
            .with_context(|| format!("failed to parse configuration variable {}", var))?),
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => Err(anyhow!("configuration variable {} is not UTF-8", var)),
    }
}
