use std::collections::HashMap;
use crate::dag::{SubTask, TaskStatus, TaskUpdate};

pub struct Aggregator {
    results: HashMap<String, String>,
    statuses: HashMap<String, String>,
}

impl Aggregator {
    pub fn new() -> Self {
        Self {
            results: HashMap::new(),
            statuses: HashMap::new(),
        }
    }

    pub fn record(&mut self, update: &TaskUpdate) {
        let status_str = match update.status {
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            _ => "unknown",
        };
        self.statuses.insert(update.task_id.clone(), status_str.to_string());
        if update.status == TaskStatus::Completed {
            if let Some(output) = &update.output {
                self.results.insert(update.task_id.clone(), output.clone());
            }
        }
    }

    pub fn context_for(&self, task: &SubTask) -> Option<String> {
        let parts: Vec<&str> = task
            .depends_on
            .iter()
            .filter_map(|dep_id| self.results.get(dep_id).map(|s| s.as_str()))
            .collect();
        if parts.is_empty() { None } else { Some(parts.join("\n---\n")) }
    }

    pub fn get_status(&self, task_id: &str) -> Option<String> {
        self.statuses.get(task_id).cloned()
    }

    pub fn get_output(&self, task_id: &str) -> Option<String> {
        self.results.get(task_id).cloned()
    }

    pub fn results(&self) -> &HashMap<String, String> {
        &self.results
    }
}
