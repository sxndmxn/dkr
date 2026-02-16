use crossterm::style::{Color, Stylize};

use crate::config::Config;
use crate::docker::models::{
    ContainerInfo, ContainerStatus, HealthStatus, format_bytes, group_containers,
};

pub fn print_table(containers: &[ContainerInfo], config: &Config) {
    if containers.is_empty() {
        println!("No containers found.");
        return;
    }

    let colors = ThemeColors::from_config(config);

    if config.display.group_by_project {
        print_grouped(containers, &colors, config);
    } else {
        print_flat(containers, &colors, config);
    }
}

pub fn print_single(container: &ContainerInfo, config: &Config) {
    let colors = ThemeColors::from_config(config);
    let sc = status_color(&container.status, &container.health, &colors);

    println!(
        "{}",
        format!("Container: {}", container.name)
            .with(Color::White)
            .bold()
    );
    println!("  Image:   {}", container.image);
    println!(
        "  Status:  {}",
        format!(
            "{} ({})",
            container.status_display(),
            container.health_display()
        )
        .with(sc)
    );
    println!("  CPU:     {:.1}%", container.cpu_percent);

    let mem_str = format!(
        "{} / {} ({:.0}%)",
        format_bytes(container.memory_usage),
        format_bytes(container.memory_limit),
        container.memory_percent()
    );
    let mem_color = if container.memory_percent() >= config.alerts.memory_warn_percent {
        colors.warning
    } else {
        Color::White
    };
    println!("  Memory:  {}", mem_str.with(mem_color));
    println!("  Ports:   {}", container.ports_display());
    println!("  Uptime:  {}", container.uptime_string());

    if let Some(project) = &container.compose_project {
        println!("  Project: {project}");
    }
}

struct ThemeColors {
    healthy: Color,
    warning: Color,
    critical: Color,
    accent: Color,
}

impl ThemeColors {
    fn from_config(config: &Config) -> Self {
        Self {
            healthy: parse_hex_color(&config.theme.healthy),
            warning: parse_hex_color(&config.theme.warning),
            critical: parse_hex_color(&config.theme.critical),
            accent: parse_hex_color(&config.theme.accent),
        }
    }
}

fn print_grouped(containers: &[ContainerInfo], colors: &ThemeColors, config: &Config) {
    let groups = group_containers(containers);

    println!(
        "{}",
        format!(
            " {:<20} {:<10} {:<8} {:>6}  {:>14}  {}",
            "NAME", "STATUS", "HEALTH", "CPU", "MEMORY", "UPTIME"
        )
        .with(Color::DarkGrey)
    );

    for group in &groups {
        println!("{}", format!("[{}]", group.name).with(colors.accent).bold());

        for c in &group.containers {
            print_container_row(c, colors, config);
        }
    }
}

fn print_flat(containers: &[ContainerInfo], colors: &ThemeColors, config: &Config) {
    println!(
        "{}",
        format!(
            " {:<20} {:<10} {:<8} {:>6}  {:>14}  {}",
            "NAME", "STATUS", "HEALTH", "CPU", "MEMORY", "UPTIME"
        )
        .with(Color::DarkGrey)
    );

    for c in containers {
        print_container_row(c, colors, config);
    }
}

fn print_container_row(c: &ContainerInfo, colors: &ThemeColors, config: &Config) {
    let sc = status_color(&c.status, &c.health, colors);
    let indicator = status_indicator(&c.status, &c.health);

    let mem_str = if c.memory_limit > 0 {
        format!(
            "{} / {} ({:.0}%)",
            format_bytes(c.memory_usage),
            format_bytes(c.memory_limit),
            c.memory_percent()
        )
    } else if c.status.is_running() {
        format_bytes(c.memory_usage)
    } else {
        String::from("\u{2014}")
    };

    let cpu_str = if c.status.is_running() {
        format!("{:.1}%", c.cpu_percent)
    } else {
        String::from("\u{2014}")
    };

    let mem_color = if c.memory_percent() >= config.alerts.memory_warn_percent {
        colors.warning
    } else {
        Color::White
    };

    let cpu_color = if c.cpu_percent >= config.alerts.cpu_warn_percent {
        colors.warning
    } else {
        Color::White
    };

    let name = truncate_name(&c.name, 18);

    println!(
        "  {:<18} {:<10} {:<8} {:>6}  {:>14}  {}",
        name.with(Color::White),
        format!("{} {indicator}", c.status_display()).with(sc),
        c.health_display().with(sc),
        cpu_str.with(cpu_color),
        mem_str.with(mem_color),
        c.uptime_string().with(Color::DarkGrey),
    );
}

fn truncate_name(name: &str, max_len: usize) -> String {
    if name.len() > max_len {
        format!("{}...", &name[..max_len.saturating_sub(3)])
    } else {
        name.to_owned()
    }
}

const fn status_color(
    status: &ContainerStatus,
    health: &HealthStatus,
    colors: &ThemeColors,
) -> Color {
    match (status, health) {
        (ContainerStatus::Running, HealthStatus::Healthy | HealthStatus::None) => colors.healthy,
        (ContainerStatus::Running, HealthStatus::Starting)
        | (ContainerStatus::Restarting | ContainerStatus::Paused, _) => colors.warning,
        (ContainerStatus::Running, HealthStatus::Unhealthy)
        | (ContainerStatus::Exited(_) | ContainerStatus::Dead, _) => colors.critical,
        (ContainerStatus::Created | ContainerStatus::Removing | ContainerStatus::Unknown(_), _) => {
            Color::DarkGrey
        }
    }
}

const fn status_indicator(status: &ContainerStatus, health: &HealthStatus) -> &'static str {
    match (status, health) {
        (
            ContainerStatus::Running,
            HealthStatus::Healthy | HealthStatus::None | HealthStatus::Unhealthy,
        ) => "\u{25cf}",
        (ContainerStatus::Running, HealthStatus::Starting) => "\u{25d0}",
        (ContainerStatus::Restarting, _) => "\u{21bb}",
        (ContainerStatus::Paused, _) => "\u{23f8}",
        (
            ContainerStatus::Exited(_)
            | ContainerStatus::Dead
            | ContainerStatus::Created
            | ContainerStatus::Removing
            | ContainerStatus::Unknown(_),
            _,
        ) => "\u{25cb}",
    }
}

fn parse_hex_color(hex: &str) -> Color {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 {
        return Color::White;
    }

    let Ok(r) = u8::from_str_radix(&hex[0..2], 16) else {
        return Color::White;
    };
    let Ok(g) = u8::from_str_radix(&hex[2..4], 16) else {
        return Color::White;
    };
    let Ok(b) = u8::from_str_radix(&hex[4..6], 16) else {
        return Color::White;
    };

    Color::Rgb { r, g, b }
}
