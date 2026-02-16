use crate::config::Config;
use crate::docker::models::{ComposeGroup, ContainerInfo, group_containers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Filter,
}

pub struct App {
    pub running: bool,
    pub containers: Vec<ContainerInfo>,
    pub groups: Vec<ComposeGroup>,
    pub selected_index: usize,
    pub filter: String,
    pub input_mode: InputMode,
    pub host: String,
    pub config: Config,
    pub show_help: bool,
    pub last_error: Option<String>,
}

impl App {
    #[must_use]
    pub const fn new(config: Config, host: String) -> Self {
        Self {
            running: true,
            containers: Vec::new(),
            groups: Vec::new(),
            selected_index: 0,
            filter: String::new(),
            input_mode: InputMode::Normal,
            host,
            config,
            show_help: false,
            last_error: None,
        }
    }

    pub fn update_containers(&mut self, containers: Vec<ContainerInfo>) {
        self.containers = containers;
        self.rebuild_groups();
    }

    fn rebuild_groups(&mut self) {
        let filtered = self.filtered_containers();
        self.groups = if self.config.display.group_by_project {
            group_containers(&filtered)
        } else {
            vec![ComposeGroup {
                name: String::new(),
                containers: filtered,
            }]
        };

        // Clamp selection
        let total = self.total_items();
        if total == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= total {
            self.selected_index = total - 1;
        }
    }

    fn filtered_containers(&self) -> Vec<ContainerInfo> {
        let filter_lower = self.filter.to_lowercase();
        self.containers
            .iter()
            .filter(|c| {
                if !self.config.display.show_stopped && !c.status.is_running() {
                    return false;
                }
                if self.filter.is_empty() {
                    return true;
                }
                let name_match = c.name.to_lowercase().contains(&filter_lower);
                let image_match = c.image.to_lowercase().contains(&filter_lower);
                let project_match = c
                    .compose_project
                    .as_ref()
                    .is_some_and(|p| p.to_lowercase().contains(&filter_lower));
                name_match || image_match || project_match
            })
            .cloned()
            .collect()
    }

    #[must_use]
    pub fn total_items(&self) -> usize {
        self.groups.iter().map(|g| g.containers.len()).sum()
    }

    #[must_use]
    pub fn selected_container(&self) -> Option<&ContainerInfo> {
        let mut idx = 0;
        for group in &self.groups {
            for c in &group.containers {
                if idx == self.selected_index {
                    return Some(c);
                }
                idx += 1;
            }
        }
        None
    }

    pub fn select_next(&mut self) {
        let total = self.total_items();
        if total > 0 {
            self.selected_index = (self.selected_index + 1) % total;
        }
    }

    pub fn select_prev(&mut self) {
        let total = self.total_items();
        if total > 0 {
            self.selected_index = if self.selected_index == 0 {
                total - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    pub const fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub const fn start_filter(&mut self) {
        self.input_mode = InputMode::Filter;
    }

    pub fn cancel_filter(&mut self) {
        self.input_mode = InputMode::Normal;
        self.filter.clear();
        self.rebuild_groups();
    }

    pub fn confirm_filter(&mut self) {
        self.input_mode = InputMode::Normal;
        self.rebuild_groups();
    }

    pub fn filter_push(&mut self, ch: char) {
        self.filter.push(ch);
        self.rebuild_groups();
    }

    pub fn filter_pop(&mut self) {
        self.filter.pop();
        self.rebuild_groups();
    }

    #[must_use]
    pub const fn container_count(&self) -> usize {
        self.containers.len()
    }
}
