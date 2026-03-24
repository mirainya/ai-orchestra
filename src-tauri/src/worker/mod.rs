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
    let mut cmd = resolve_and_build_command(path);

    // Clear env vars that prevent nested CLI sessions
    cmd.env_remove("CLAUDECODE");

    // On Windows, ensure Claude CLI can find git-bash
    #[cfg(windows)]
    {
        if std::env::var("CLAUDE_CODE_GIT_BASH_PATH").is_err() {
            if let Some(bash) = find_git_bash() {
                cmd.env("CLAUDE_CODE_GIT_BASH_PATH", bash);
            }
        }
    }

    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }
    cmd
}

/// Find git-bash on Windows by searching PATH for bash.exe
#[cfg(windows)]
fn find_git_bash() -> Option<String> {
    if let Ok(paths) = std::env::var("PATH") {
        for dir in std::env::split_paths(&paths) {
            let candidate = dir.join("bash.exe");
            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }
    // Common install locations
    for path in &[
        r"C:\Program Files\Git\bin\bash.exe",
        r"C:\Program Files (x86)\Git\bin\bash.exe",
    ] {
        if std::path::Path::new(path).exists() {
            return Some(path.to_string());
        }
    }
    None
}

/// On Windows, .cmd files must go through `cmd /c` which mangles special chars
/// ({, }, &, |, etc.) in arguments. Instead, parse the .cmd to find the actual
/// node script and invoke `node <script>` directly.
fn resolve_and_build_command(path: &str) -> tokio::process::Command {
    #[cfg(windows)]
    {
        if let Some((node, script)) = find_cmd_node_script(path) {
            let mut cmd = tokio::process::Command::new(node);
            cmd.arg(script);
            return cmd;
        }
        // Fallback: use cmd /c
        let mut cmd = tokio::process::Command::new("cmd");
        cmd.args(["/c", path]);
        cmd
    }
    #[cfg(not(windows))]
    {
        tokio::process::Command::new(path)
    }
}

/// Parse a .cmd wrapper to extract the node executable and JS script path.
/// Returns Some((node_path, script_path)) if found.
#[cfg(windows)]
fn find_cmd_node_script(name: &str) -> Option<(String, String)> {
    use std::path::Path;

    // Find the .cmd file in PATH
    let cmd_path = {
        let p = Path::new(name);
        if p.extension().is_some() && p.exists() {
            Some(p.to_path_buf())
        } else if let Ok(paths) = std::env::var("PATH") {
            std::env::split_paths(&paths)
                .map(|dir| dir.join(format!("{}.cmd", name)))
                .find(|c| c.exists())
        } else {
            None
        }
    }?;

    let content = std::fs::read_to_string(&cmd_path).ok()?;
    let cmd_dir = cmd_path.parent()?;

    // Look for pattern: "%_prog%" "%dp0%\path\to\script.js" %*
    // or: "%_prog%" "%dp0%/path/to/script.js" %*
    for line in content.lines() {
        let line = line.trim();
        if line.contains("%_prog%") && line.contains(".js") {
            // Extract the .js path: find %dp0%\...\something.js
            if let Some(start) = line.find("%dp0%") {
                let after = &line[start + 5..]; // skip %dp0%
                let after = after.trim_start_matches('\\').trim_start_matches('/');
                // Find end: either " or space before %*
                let end = after.find('"')
                    .or_else(|| after.find(" %*"))
                    .unwrap_or(after.len());
                let rel_script = &after[..end];
                let script_path = cmd_dir.join(rel_script);
                if script_path.exists() {
                    // Determine node path
                    let node_exe = cmd_dir.join("node.exe");
                    let node = if node_exe.exists() {
                        node_exe.to_string_lossy().to_string()
                    } else {
                        "node".to_string()
                    };
                    return Some((node, script_path.to_string_lossy().to_string()));
                }
            }
        }
    }
    None
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
