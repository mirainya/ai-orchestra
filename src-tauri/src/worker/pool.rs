use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::config::WorkerConfig;
use crate::worker::CliAdapter;
use crate::worker::adapter::create_adapter;

pub struct WorkerPool {
    workers: Vec<Arc<WorkerSlot>>,
}

pub struct WorkerSlot {
    pub adapter: Box<dyn CliAdapter>,
    pub busy: Mutex<bool>,
    pub skills: Vec<String>,
    pub role: String,
}

impl WorkerPool {
    pub fn new(configs: &[WorkerConfig]) -> Self {
        let workers = configs
            .iter()
            .map(|c| {
                Arc::new(WorkerSlot {
                    adapter: create_adapter(c),
                    busy: Mutex::new(false),
                    skills: c.skills.clone(),
                    role: serde_json::to_value(&c.role)
                        .ok()
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_else(|| "executor".into()),
                })
            })
            .collect();
        Self { workers }
    }

    /// Find an idle worker matching the given cli_type, mark it busy.
    pub async fn acquire(&self, cli_type: &str) -> Option<Arc<WorkerSlot>> {
        for slot in &self.workers {
            if slot.adapter.cli_type() == cli_type {
                let mut busy = slot.busy.lock().await;
                if !*busy {
                    *busy = true;
                    return Some(Arc::clone(slot));
                }
            }
        }
        None
    }

    /// Find an idle worker that has the required skill, mark it busy.
    pub async fn acquire_by_skill(&self, skill: &str) -> Option<Arc<WorkerSlot>> {
        for slot in &self.workers {
            if slot.skills.iter().any(|s| s == skill) {
                let mut busy = slot.busy.lock().await;
                if !*busy {
                    *busy = true;
                    return Some(Arc::clone(slot));
                }
            }
        }
        None
    }

    /// Release a worker back to idle.
    pub async fn release(&self, name: &str) {
        for slot in &self.workers {
            if slot.adapter.name() == name {
                let mut busy = slot.busy.lock().await;
                *busy = false;
                return;
            }
        }
    }

    /// Get status of all workers for the frontend.
    pub async fn status(&self) -> Vec<WorkerStatus> {
        let mut result = Vec::new();
        for slot in &self.workers {
            let busy = *slot.busy.lock().await;
            result.push(WorkerStatus {
                name: slot.adapter.name().to_string(),
                cli_type: slot.adapter.cli_type().to_string(),
                status: if busy { "busy".into() } else { "idle".into() },
                role: slot.role.clone(),
                skills: slot.skills.clone(),
            });
        }
        result
    }
}

#[derive(serde::Serialize, Clone)]
pub struct WorkerStatus {
    pub name: String,
    pub cli_type: String,
    pub status: String,
    pub role: String,
    pub skills: Vec<String>,
}
