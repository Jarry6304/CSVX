use anyhow::Context;
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub source: SourceCfg,
    pub database: DatabaseCfg,
    pub log: LogCfg,
    pub profile: ProfileCfg,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SourceCfg {
    pub input_dir: PathBuf,
    pub backup_dir: PathBuf,
    pub error_dir: PathBuf,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseCfg {
    pub target: String,
    pub conn: String,
    pub table: String,
    #[serde(default)]
    pub enabled: HashMap<String, bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LogCfg {
    pub dir: PathBuf,
    pub level: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProfileCfg {
    pub dir: PathBuf,
}

const PASSWORD_PLACEHOLDER: &str = "<REPLACED_BY_ENV>";

pub fn load(path: &Path) -> anyhow::Result<Config> {
    let _ = dotenvy::dotenv();

    let mut cfg: Config = Figment::new()
        .merge(Toml::file(path))
        .merge(Env::prefixed("SMI_").split("__"))
        .extract()
        .with_context(|| format!("loading config from {}", path.display()))?;

    if let Ok(pw) = std::env::var("SMI_DB_PASSWORD") {
        if cfg.database.conn.contains(PASSWORD_PLACEHOLDER) {
            cfg.database.conn = cfg.database.conn.replace(PASSWORD_PLACEHOLDER, &pw);
        }
    }

    Ok(cfg)
}

impl DatabaseCfg {
    pub fn is_enabled(&self, target: &str) -> bool {
        self.enabled.get(target).copied().unwrap_or(false)
    }
}
