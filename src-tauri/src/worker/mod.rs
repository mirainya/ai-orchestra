use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

pub mod adapter;
pub mod pool;

#[derive(Debug, Clone)]
pub struct TaskOutput {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

/// A line emitted during CLI execution
#[derive(Debug, Clone)]
pub struct OutputLine {
    pub source: String,
    pub text: String,
    pub is_stderr: bool,
}

#[async_trait]
pub trait CliAdapter: Send + Sync {
    /// Execute with streaming: sends lines through the channel as they arrive.
    /// Returns the full TaskOutput when done.
    async fn execute_streaming(
        &self,
        prompt: &str,
        context: Option<&str>,
        working_dir: Option<&str>,
        line_tx: mpsc::UnboundedSender<OutputLine>,
    ) -> Result<TaskOutput>;

    /// Execute without streaming (convenience wrapper).
    async fn execute(&self, prompt: &str, context: Option<&str>, working_dir: Option<&str>) -> Result<TaskOutput> {
        let (tx, _rx) = mpsc::unbounded_channel();
        self.execute_streaming(prompt, context, working_dir, tx).await
    }

    fn name(&self) -> &str;
    fn cli_type(&self) -> &str;
}

/// Helper: create a Command for a CLI path, handling Windows .cmd wrappers
pub fn spawn_cli(path: &str, working_dir: Option<&str>) -> tokio::process::Command {
    #[cfg(windows)]
    let mut cmd = {
        let mut cmd = tokio::process::Command::new("cmd");
        cmd.args(["/c", path]);
        cmd
    };
    #[cfg(not(windows))]
    let mut cmd = tokio::process::Command::new(path);

    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }
    cmd
}

/// Helper: build the full prompt with optional context
pub fn build_prompt(prompt: &str, context: Option<&str>) -> String {
    match context {
        Some(ctx) => format!("Context from previous task:\n{ctx}\n\nTask:\n{prompt}"),
        None => prompt.to_string(),
    }
}

/// Helper: spawn a command and stream stdout/stderr line by line
pub async fn run_streaming(
    mut child: tokio::process::Child,
    source_name: &str,
    line_tx: mpsc::UnboundedSender<OutputLine>,
) -> Result<TaskOutput> {
    use tokio::io::{AsyncBufReadExt, BufReader};

    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

    let source = source_name.to_string();
    let source2 = source.clone();
    let tx1 = line_tx.clone();
    let tx2 = line_tx;

    let stdout_handle = tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        let mut collected = String::new();
        while let Ok(Some(line)) = lines.next_line().await {
            collected.push_str(&line);
            collected.push('\n');
            let _ = tx1.send(OutputLine {
                source: source.clone(),
                text: line,
                is_stderr: false,
            });
        }
        collected
    });

    let stderr_handle = tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        let mut collected = String::new();
        while let Ok(Some(line)) = lines.next_line().await {
            collected.push_str(&line);
            collected.push('\n');
            let _ = tx2.send(OutputLine {
                source: source2.clone(),
                text: line,
                is_stderr: true,
            });
        }
        collected
    });

    let status = child.wait().await?;
    let stdout_str = stdout_handle.await.unwrap_or_default();
    let stderr_str = stderr_handle.await.unwrap_or_default();

    Ok(TaskOutput {
        stdout: stdout_str,
        stderr: stderr_str,
        success: status.success(),
    })
}
