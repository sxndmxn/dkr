use bollard::Docker;
use bollard::models::{
    ContainerStateStatusEnum, ContainerStatsResponse, ContainerSummaryStateEnum, HealthStatusEnum,
    PortTypeEnum,
};
use bollard::query_parameters::{InspectContainerOptions, ListContainersOptions, StatsOptions};
use futures_util::StreamExt as _;

use crate::docker::models::{ContainerInfo, ContainerStatus, HealthStatus, PortBinding};
use crate::error::Result;

/// Fetch all containers with their status and stats.
///
/// # Errors
///
/// Returns `DkrError::Connection` if Docker communication fails.
pub async fn fetch_all(docker: &Docker, include_stopped: bool) -> Result<Vec<ContainerInfo>> {
    let options = ListContainersOptions {
        all: include_stopped,
        ..Default::default()
    };

    let summaries = docker
        .list_containers(Some(options))
        .await
        .map_err(crate::error::DkrError::Connection)?;

    let mut containers = Vec::with_capacity(summaries.len());

    for summary in &summaries {
        let id = summary.id.as_deref().unwrap_or_default();
        if id.is_empty() {
            continue;
        }

        match build_container_info(docker, id, summary).await {
            Ok(c) => containers.push(c),
            Err(e) => tracing::warn!(id, "Failed to fetch container info: {e}"),
        }
    }

    containers.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(containers)
}

/// Fetch info for a single container by name.
///
/// # Errors
///
/// Returns error if the container is not found or stats are unavailable.
pub async fn fetch_one(docker: &Docker, name: &str) -> Result<ContainerInfo> {
    let inspect = docker
        .inspect_container(name, None::<InspectContainerOptions>)
        .await
        .map_err(|_| crate::error::DkrError::ContainerNotFound(name.to_owned()))?;

    let id = inspect.id.as_deref().unwrap_or_default();
    let container_name = inspect
        .name
        .as_deref()
        .map_or(name, |n| n.strip_prefix('/').unwrap_or(n))
        .to_owned();

    let image = inspect
        .config
        .as_ref()
        .and_then(|c| c.image.clone())
        .unwrap_or_default();

    let state = inspect.state.as_ref();
    let status = parse_status_from_state(state);
    let health = parse_health_from_state(state);

    let started_at = state
        .and_then(|s| s.started_at.as_deref())
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));

    let compose_project = inspect
        .config
        .as_ref()
        .and_then(|c| c.labels.as_ref())
        .and_then(|l| l.get("com.docker.compose.project"))
        .cloned();

    let port_bindings = extract_ports_from_inspect(&inspect);

    let (cpu_percent, memory_usage, memory_limit) = if status.is_running() {
        fetch_stats(docker, id).await.unwrap_or((0.0, 0, 0))
    } else {
        (0.0, 0, 0)
    };

    Ok(ContainerInfo {
        id: id.to_owned(),
        name: container_name,
        image,
        status,
        health,
        cpu_percent,
        memory_usage,
        memory_limit,
        ports: port_bindings,
        started_at,
        compose_project,
    })
}

async fn build_container_info(
    docker: &Docker,
    id: &str,
    summary: &bollard::models::ContainerSummary,
) -> Result<ContainerInfo> {
    let name = summary
        .names
        .as_ref()
        .and_then(|names| names.first())
        .map_or_else(
            || id[..12].to_owned(),
            |n| n.strip_prefix('/').unwrap_or(n).to_owned(),
        );

    let image = summary.image.clone().unwrap_or_default();

    let status = summary.state.map_or_else(
        || ContainerStatus::Unknown("unknown".to_owned()),
        parse_summary_state,
    );

    let compose_project = summary
        .labels
        .as_ref()
        .and_then(|l| l.get("com.docker.compose.project"))
        .cloned();

    let port_bindings = summary
        .ports
        .as_ref()
        .map(|port_list| {
            port_list
                .iter()
                .map(|p| PortBinding {
                    container_port: p.private_port,
                    protocol: p
                        .typ
                        .as_ref()
                        .map_or_else(|| "tcp".to_owned(), |t| format_port_type(*t)),
                    host_port: p.public_port,
                })
                .collect()
        })
        .unwrap_or_default();

    let health = fetch_health(docker, id).await;
    let started_at = fetch_started_at(docker, id).await;

    let (cpu_percent, memory_usage, memory_limit) = if status.is_running() {
        fetch_stats(docker, id).await.unwrap_or((0.0, 0, 0))
    } else {
        (0.0, 0, 0)
    };

    Ok(ContainerInfo {
        id: id.to_owned(),
        name,
        image,
        status,
        health,
        cpu_percent,
        memory_usage,
        memory_limit,
        ports: port_bindings,
        started_at,
        compose_project,
    })
}

async fn fetch_stats(docker: &Docker, id: &str) -> Result<(f64, u64, u64)> {
    let options = StatsOptions {
        stream: false,
        one_shot: true,
    };

    let mut stream = docker.stats(id, Some(options));
    let Some(stats_result) = stream.next().await else {
        return Err(crate::error::DkrError::StatsUnavailable(id.to_owned()));
    };

    let stats = stats_result.map_err(crate::error::DkrError::Connection)?;

    let cpu_percent = calculate_cpu_percent(&stats);
    let memory_usage = stats
        .memory_stats
        .as_ref()
        .and_then(|m| m.usage)
        .unwrap_or(0);
    let memory_limit = stats
        .memory_stats
        .as_ref()
        .and_then(|m| m.limit)
        .unwrap_or(0);

    Ok((cpu_percent, memory_usage, memory_limit))
}

#[allow(clippy::cast_precision_loss, reason = "CPU counters fit in f64")]
fn calculate_cpu_percent(stats: &ContainerStatsResponse) -> f64 {
    let Some(cpu) = stats.cpu_stats.as_ref() else {
        return 0.0;
    };
    let Some(precpu) = stats.precpu_stats.as_ref() else {
        return 0.0;
    };

    let cpu_total = cpu
        .cpu_usage
        .as_ref()
        .and_then(|u| u.total_usage)
        .unwrap_or(0);
    let precpu_total = precpu
        .cpu_usage
        .as_ref()
        .and_then(|u| u.total_usage)
        .unwrap_or(0);

    let cpu_delta = cpu_total as f64 - precpu_total as f64;
    let system_delta =
        cpu.system_cpu_usage.unwrap_or(0) as f64 - precpu.system_cpu_usage.unwrap_or(0) as f64;

    if system_delta <= 0.0 || cpu_delta < 0.0 {
        return 0.0;
    }

    let num_cpus = cpu.online_cpus.unwrap_or(1);

    (cpu_delta / system_delta) * f64::from(num_cpus) * 100.0
}

async fn fetch_health(docker: &Docker, id: &str) -> HealthStatus {
    let Ok(inspect) = docker
        .inspect_container(id, None::<InspectContainerOptions>)
        .await
    else {
        return HealthStatus::None;
    };

    parse_health_from_state(inspect.state.as_ref())
}

async fn fetch_started_at(docker: &Docker, id: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let inspect = docker
        .inspect_container(id, None::<InspectContainerOptions>)
        .await
        .ok()?;

    inspect
        .state
        .as_ref()
        .and_then(|s| s.started_at.as_deref())
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

const fn parse_summary_state(state: ContainerSummaryStateEnum) -> ContainerStatus {
    match state {
        ContainerSummaryStateEnum::RUNNING => ContainerStatus::Running,
        ContainerSummaryStateEnum::PAUSED => ContainerStatus::Paused,
        ContainerSummaryStateEnum::RESTARTING => ContainerStatus::Restarting,
        ContainerSummaryStateEnum::EXITED | ContainerSummaryStateEnum::EMPTY => {
            ContainerStatus::Exited(0)
        }
        ContainerSummaryStateEnum::DEAD => ContainerStatus::Dead,
        ContainerSummaryStateEnum::CREATED => ContainerStatus::Created,
        ContainerSummaryStateEnum::REMOVING => ContainerStatus::Removing,
    }
}

fn parse_status_from_state(state: Option<&bollard::models::ContainerState>) -> ContainerStatus {
    let Some(state) = state else {
        return ContainerStatus::Unknown("unknown".to_owned());
    };

    if state.running.unwrap_or(false) {
        return ContainerStatus::Running;
    }
    if state.paused.unwrap_or(false) {
        return ContainerStatus::Paused;
    }
    if state.restarting.unwrap_or(false) {
        return ContainerStatus::Restarting;
    }
    if state.dead.unwrap_or(false) {
        return ContainerStatus::Dead;
    }

    match &state.status {
        Some(ContainerStateStatusEnum::CREATED) => ContainerStatus::Created,
        Some(ContainerStateStatusEnum::REMOVING) => ContainerStatus::Removing,
        Some(
            ContainerStateStatusEnum::EXITED
            | ContainerStateStatusEnum::RUNNING
            | ContainerStateStatusEnum::PAUSED
            | ContainerStateStatusEnum::RESTARTING
            | ContainerStateStatusEnum::DEAD
            | ContainerStateStatusEnum::EMPTY,
        )
        | None => ContainerStatus::Exited(state.exit_code.unwrap_or(0)),
    }
}

const fn parse_health_from_state(state: Option<&bollard::models::ContainerState>) -> HealthStatus {
    let Some(state) = state else {
        return HealthStatus::None;
    };

    let Some(health) = &state.health else {
        return HealthStatus::None;
    };

    let Some(status) = &health.status else {
        return HealthStatus::None;
    };

    match status {
        HealthStatusEnum::HEALTHY => HealthStatus::Healthy,
        HealthStatusEnum::UNHEALTHY => HealthStatus::Unhealthy,
        HealthStatusEnum::STARTING => HealthStatus::Starting,
        HealthStatusEnum::NONE | HealthStatusEnum::EMPTY => HealthStatus::None,
    }
}

fn extract_ports_from_inspect(
    inspect: &bollard::models::ContainerInspectResponse,
) -> Vec<PortBinding> {
    let Some(network) = &inspect.network_settings else {
        return Vec::new();
    };

    let Some(ports_map) = &network.ports else {
        return Vec::new();
    };

    let mut result = Vec::new();
    for (port_proto, bindings) in ports_map {
        let mut segments = port_proto.split('/');
        let container_port = segments.next().and_then(|p| p.parse::<u16>().ok());
        let protocol = segments.next().unwrap_or("tcp");

        let Some(container_port) = container_port else {
            continue;
        };

        let host_port = bindings
            .as_ref()
            .and_then(|b| b.first())
            .and_then(|b| b.host_port.as_ref())
            .and_then(|p| p.parse::<u16>().ok());

        result.push(PortBinding {
            container_port,
            protocol: protocol.to_owned(),
            host_port,
        });
    }

    result.sort_by_key(|p| p.container_port);
    result
}

fn format_port_type(pt: PortTypeEnum) -> String {
    match pt {
        PortTypeEnum::TCP | PortTypeEnum::EMPTY => "tcp".to_owned(),
        PortTypeEnum::UDP => "udp".to_owned(),
        PortTypeEnum::SCTP => "sctp".to_owned(),
    }
}
