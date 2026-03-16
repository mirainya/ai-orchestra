use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;
use crate::config::WorkerConfig;
use crate::worker::{CliAdapter, TaskOutput, OutputLine, build_prompt, run_streaming, spawn_cli};

pub struct CodexAdapter {
    name: String,
    cli_path: String,
    extra_args: Vec<String>,
}

impl CodexAdapter {
    pub fn new(config: &WorkerConfig) -> Self {
        Self {
            name: config.name.clone(),
            cli_path: config.cli_path.clone().unwrap_or_else(|| "codex".into()),
            extra_args: config.extra_args.clone(),
        }
    }
}

#[async_trait]
impl CliAdapter for CodexAdapter {
    async fn execute_streaming(
        &self,
        prompt: &str,
        context: Option<&str>,
        working_dir: Option<&str>,
        line_tx: mpsc::UnboundedSender<OutputLine>,
    ) -> Result<TaskOutput> {
        let full_prompt = build_prompt(prompt, context);
        let mut cmd = spawn_cli(&self.cli_path, working_dir);
        for arg in &self.extra_args {
            cmd.arg(arg);
        }
        cmd.arg(&full_prompt);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let child = cmd.spawn()?;
        run_streaming(child, &self.name, line_tx).await
    }

    fn name(&self) -> &str { &self.name }
    fn cli_type(&self) -> &str { "codex" }
}
