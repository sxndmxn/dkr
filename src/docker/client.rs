use bollard::Docker;

use crate::error::{DkrError, Result};

/// Create a Docker client from a host URI string.
///
/// # Errors
///
/// Returns `DkrError::Connection` if the connection fails.
pub fn connect(host: &str) -> Result<Docker> {
    tracing::debug!(host, "Connecting to Docker");

    let docker = if let Some(path) = host.strip_prefix("unix://") {
        Docker::connect_with_unix(path, 120, bollard::API_DEFAULT_VERSION)
            .map_err(DkrError::Connection)?
    } else if host.starts_with("ssh://")
        || host.starts_with("tcp://")
        || host.starts_with("https://")
    {
        Docker::connect_with_local(host, 120, bollard::API_DEFAULT_VERSION)
            .map_err(DkrError::Connection)?
    } else {
        // Treat as unix socket path
        Docker::connect_with_unix(host, 120, bollard::API_DEFAULT_VERSION)
            .map_err(DkrError::Connection)?
    };

    Ok(docker)
}

/// Verify the connection by pinging Docker.
///
/// # Errors
///
/// Returns `DkrError::Connection` if the ping fails.
pub async fn ping(docker: &Docker) -> Result<()> {
    docker.ping().await.map_err(DkrError::Connection)?;
    tracing::debug!("Docker ping successful");
    Ok(())
}
