use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CliType {
    Claude,
    Codex,
    Glm,
    #[serde(rename = "claude_cli")]
    ClaudeCli,
    #[serde(rename = "codex_cli")]
    CodexCli,
    #[serde(rename = "glm_cli")]
    GlmCli,
    #[serde(rename = "openai")]
    OpenAi,
    #[serde(rename = "anthropic")]
    Anthropic,
}

impl std::fmt::Display for CliType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliType::Claude | CliType::ClaudeCli => write!(f, "claude_cli"),
            CliType::Codex | CliType::CodexCli => write!(f, "codex_cli"),
            CliType::Glm | CliType::GlmCli => write!(f, "glm_cli"),
            CliType::OpenAi => write!(f, "openai"),
            CliType::Anthropic => write!(f, "anthropic"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExecMode {
    Independent,
    Pipeline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub goal: String,
    pub tasks: Vec<SubTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    pub id: String,
    pub description: String,
    pub cli_type: CliType,
    pub depends_on: Vec<String>,
    pub prompt: String,
    #[serde(default = "default_exec_mode")]
    pub execution_mode: ExecMode,
}

fn default_exec_mode() -> ExecMode {
    ExecMode::Independent
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskUpdate {
    pub task_id: String,
    pub status: TaskStatus,
    pub output: Option<String>,
}

pub mod scheduler;
