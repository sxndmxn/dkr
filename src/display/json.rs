use serde::Serialize;

use crate::docker::models::{ContainerInfo, format_bytes};

#[derive(Serialize)]
struct ContainerJson<'a> {
    id: &'a str,
    name: &'a str,
    image: &'a str,
    status: &'a str,
    health: &'a str,
    cpu_percent: f64,
    memory_usage: String,
    memory_limit: String,
    memory_percent: f64,
    ports: String,
    uptime: String,
    compose_project: Option<&'a str>,
}

pub fn print_json(containers: &[ContainerInfo]) {
    let json_containers: Vec<ContainerJson<'_>> = containers.iter().map(to_json).collect();

    match serde_json::to_string_pretty(&json_containers) {
        Ok(json) => println!("{json}"),
        Err(e) => eprintln!("Failed to serialize JSON: {e}"),
    }
}

pub fn print_single_json(container: &ContainerInfo) {
    let json = to_json(container);

    match serde_json::to_string_pretty(&json) {
        Ok(json) => println!("{json}"),
        Err(e) => eprintln!("Failed to serialize JSON: {e}"),
    }
}

fn to_json(c: &ContainerInfo) -> ContainerJson<'_> {
    ContainerJson {
        id: &c.id,
        name: &c.name,
        image: &c.image,
        status: c.status_display(),
        health: c.health_display(),
        cpu_percent: (c.cpu_percent * 10.0).round() / 10.0,
        memory_usage: format_bytes(c.memory_usage),
        memory_limit: format_bytes(c.memory_limit),
        memory_percent: (c.memory_percent() * 10.0).round() / 10.0,
        ports: c.ports_display(),
        uptime: c.uptime_string(),
        compose_project: c.compose_project.as_deref(),
    }
}
