use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: ContainerStatus,
    pub health: HealthStatus,
    pub cpu_percent: f64,
    pub memory_usage: u64,
    pub memory_limit: u64,
    pub ports: Vec<PortBinding>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub compose_project: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerStatus {
    Running,
    Paused,
    Restarting,
    Exited(i64),
    Dead,
    Created,
    Removing,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Starting,
    None,
}

#[derive(Debug, Clone)]
pub struct PortBinding {
    pub container_port: u16,
    pub protocol: String,
    pub host_port: Option<u16>,
}

#[derive(Debug, Clone)]
pub struct ComposeGroup {
    pub name: String,
    pub containers: Vec<ContainerInfo>,
}

impl ContainerInfo {
    #[allow(clippy::cast_precision_loss, reason = "memory values fit in f64")]
    #[must_use]
    pub fn memory_percent(&self) -> f64 {
        if self.memory_limit == 0 {
            return 0.0;
        }
        (self.memory_usage as f64 / self.memory_limit as f64) * 100.0
    }

    #[must_use]
    pub fn uptime_string(&self) -> String {
        let Some(started) = self.started_at else {
            return String::from("\u{2014}");
        };

        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(started);

        if duration.num_seconds() < 0 {
            return String::from("\u{2014}");
        }

        let days = duration.num_days();
        let hours = duration.num_hours() % 24;
        let minutes = duration.num_minutes() % 60;

        if days > 0 {
            format!("{days}d {hours}h")
        } else if hours > 0 {
            format!("{hours}h {minutes}m")
        } else {
            format!("{minutes}m")
        }
    }

    #[must_use]
    pub fn status_display(&self) -> &str {
        match &self.status {
            ContainerStatus::Running => "running",
            ContainerStatus::Paused => "paused",
            ContainerStatus::Restarting => "restarting",
            ContainerStatus::Exited(_) => "exited",
            ContainerStatus::Dead => "dead",
            ContainerStatus::Created => "created",
            ContainerStatus::Removing => "removing",
            ContainerStatus::Unknown(s) => s,
        }
    }

    #[must_use]
    pub const fn health_display(&self) -> &str {
        match &self.health {
            HealthStatus::Healthy => "healthy",
            HealthStatus::Unhealthy => "unhealthy",
            HealthStatus::Starting => "starting",
            HealthStatus::None => "none",
        }
    }

    #[must_use]
    pub fn ports_display(&self) -> String {
        if self.ports.is_empty() {
            return String::from("\u{2014}");
        }
        self.ports
            .iter()
            .map(|p| {
                p.host_port.map_or_else(
                    || format!("{}/{}", p.container_port, p.protocol),
                    |hp| format!("{hp}:{}/{}", p.container_port, p.protocol),
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl ContainerStatus {
    #[must_use]
    pub const fn is_running(&self) -> bool {
        matches!(self, Self::Running)
    }
}

/// Group containers by compose project label.
#[must_use]
pub fn group_containers(containers: &[ContainerInfo]) -> Vec<ComposeGroup> {
    let mut groups: BTreeMap<String, Vec<ContainerInfo>> = BTreeMap::new();

    for c in containers {
        let key = c
            .compose_project
            .clone()
            .unwrap_or_else(|| "(standalone)".to_owned());
        groups.entry(key).or_default().push(c.clone());
    }

    groups
        .into_iter()
        .map(|(name, containers)| ComposeGroup { name, containers })
        .collect()
}

#[allow(clippy::cast_precision_loss, reason = "byte values fit in f64")]
#[must_use]
pub fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;

    if bytes >= GIB {
        format!("{:.1}G", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1}M", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1}K", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes}B")
    }
}
