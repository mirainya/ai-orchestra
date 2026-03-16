use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub workers: Vec<WorkerConfig>,
    #[serde(default)]
    pub execution: ExecutionConfig,
}

/// Worker calling mode: CLI subprocess or HTTP API
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WorkerMode {
    Cli,
    Api,
}

impl Default for WorkerMode {
    fn default() -> Self { Self::Cli }
}

/// Worker role in the orchestration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WorkerRole {
    Planner,
    Executor,
    Both,
}

impl Default for WorkerRole {
    fn default() -> Self { Self::Executor }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    pub name: String,
    /// e.g. "openai", "anthropic", "claude_cli", "codex_cli", "glm_cli"
    pub cli_type: String,
    #[serde(default)]
    pub mode: WorkerMode,
    #[serde(default)]
    pub role: WorkerRole,
    #[serde(default)]
    pub skills: Vec<String>,

    // --- CLI mode fields ---
    #[serde(default)]
    pub cli_path: Option<String>,
    #[serde(default)]
    pub extra_args: Vec<String>,

    // --- API mode fields ---
    #[serde(default)]
    pub api_base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    #[serde(default = "default_task_timeout")]
    pub task_timeout_secs: u64,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_retry_delay")]
    pub retry_delay_secs: u64,
    #[serde(default = "default_planner_timeout")]
    pub planner_timeout_secs: u64,
}

fn default_task_timeout() -> u64 { 300 }
fn default_max_retries() -> u32 { 1 }
fn default_retry_delay() -> u64 { 2 }
fn default_planner_timeout() -> u64 { 120 }

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            task_timeout_secs: default_task_timeout(),
            max_retries: default_max_retries(),
            retry_delay_secs: default_retry_delay(),
            planner_timeout_secs: default_planner_timeout(),
        }
    }
}

impl AppConfig {
    pub fn load(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self, path: &PathBuf) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn default_config() -> Self {
        Self {
            workers: vec![],
            execution: ExecutionConfig::default(),
        }
    }

    /// Find the first worker with role=planner or role=both
    pub fn find_planner(&self) -> Option<&WorkerConfig> {
        self.workers.iter().find(|w| {
            w.role == WorkerRole::Planner || w.role == WorkerRole::Both
        })
    }

    /// Get all workers that can execute tasks (role=executor or role=both)
    pub fn executor_workers(&self) -> Vec<&WorkerConfig> {
        self.workers.iter().filter(|w| {
            w.role == WorkerRole::Executor || w.role == WorkerRole::Both
        }).collect()
    }
}
