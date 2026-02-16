use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{DkrError, Result};

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub connection: ConnectionConfig,
    pub display: DisplayConfig,
    pub alerts: AlertsConfig,
    pub theme: ThemeConfig,
}

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default)]
pub struct ConnectionConfig {
    pub host: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct DisplayConfig {
    pub refresh_interval: u64,
    pub group_by_project: bool,
    pub show_stopped: bool,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct AlertsConfig {
    pub memory_warn_percent: f64,
    pub cpu_warn_percent: f64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct ThemeConfig {
    pub accent: String,
    pub healthy: String,
    pub warning: String,
    pub critical: String,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            refresh_interval: 2000,
            group_by_project: true,
            show_stopped: true,
        }
    }
}

impl Default for AlertsConfig {
    fn default() -> Self {
        Self {
            memory_warn_percent: 80.0,
            cpu_warn_percent: 90.0,
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            accent: "#f1c35f".to_owned(),
            healthy: "#a6e3a1".to_owned(),
            warning: "#f9e2af".to_owned(),
            critical: "#f38ba8".to_owned(),
        }
    }
}

#[must_use]
pub fn default_config_path() -> PathBuf {
    let config_dir = std::env::var("XDG_CONFIG_HOME").map_or_else(
        |_| {
            let mut home = home_dir();
            home.push(".config");
            home
        },
        PathBuf::from,
    );
    config_dir.join("dkr").join("config.toml")
}

/// Load config from disk, falling back to defaults if the file doesn't exist.
///
/// # Errors
///
/// Returns `DkrError::Config` if the file can't be read, or `DkrError::ConfigParse` on parse error.
pub fn load_config(path: Option<&Path>) -> Result<Config> {
    let config_path = path.map_or_else(default_config_path, Path::to_path_buf);

    if !config_path.exists() {
        tracing::debug!(?config_path, "Config file not found, using defaults");
        return Ok(Config::default());
    }

    tracing::debug!(?config_path, "Loading config");
    let contents = std::fs::read_to_string(&config_path).map_err(|e| {
        DkrError::Config(format!(
            "Failed to read config at {}: {e}",
            config_path.display()
        ))
    })?;

    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}

fn home_dir() -> PathBuf {
    std::env::var("HOME").map_or_else(|_| PathBuf::from("/root"), PathBuf::from)
}

/// Resolve the Docker host URI from CLI flag, config, env, or default.
#[must_use]
pub fn resolve_host(cli_host: Option<&str>, config: &Config) -> String {
    if let Some(host) = cli_host {
        return host.to_owned();
    }

    if let Some(host) = &config.connection.host {
        return host.clone();
    }

    if let Ok(host) = std::env::var("DOCKER_HOST") {
        return host;
    }

    "unix:///var/run/docker.sock".to_owned()
}
