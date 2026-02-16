use thiserror::Error;

#[derive(Debug, Error)]
pub enum DkrError {
    #[error("Docker connection failed: {0}")]
    Connection(#[from] bollard::errors::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Config file parse error: {0}")]
    ConfigParse(#[from] toml::de::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Container not found: {0}")]
    ContainerNotFound(String),

    #[error("Stats unavailable for container: {0}")]
    StatsUnavailable(String),
}

pub type Result<T> = std::result::Result<T, DkrError>;
