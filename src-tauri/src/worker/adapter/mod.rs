pub mod claude;
pub mod codex;
pub mod glm;
pub mod openai_api;
pub mod anthropic_api;

pub use claude::ClaudeAdapter;
pub use codex::CodexAdapter;
pub use glm::GlmAdapter;
pub use openai_api::OpenAiApiAdapter;
pub use anthropic_api::AnthropicApiAdapter;

use crate::config::{WorkerConfig, WorkerMode};
use super::CliAdapter;

pub fn create_adapter(config: &WorkerConfig) -> Box<dyn CliAdapter> {
    match config.mode {
        WorkerMode::Api => match config.cli_type.as_str() {
            "anthropic" => Box::new(AnthropicApiAdapter::new(config)),
            _ => Box::new(OpenAiApiAdapter::new(config)), // openai and any other API types
        },
        WorkerMode::Cli => match config.cli_type.as_str() {
            "claude_cli" | "claude" => Box::new(ClaudeAdapter::new(config)),
            "glm_cli" | "glm" => Box::new(GlmAdapter::new(config)),
            _ => Box::new(CodexAdapter::new(config)), // codex_cli, codex, and fallback
        },
    }
}
