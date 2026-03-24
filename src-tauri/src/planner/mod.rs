use std::collections::HashSet;
use anyhow::Result;
use crate::config::WorkerConfig;
use crate::dag::Plan;
use crate::session::ChatMessage;
use crate::worker::spawn_cli;

pub mod parser;
#[cfg(test)]
mod integration_test;

/// Extract the actual text result from Claude CLI JSON output.
/// Claude CLI with `--output-format json` outputs JSON like:
///   {"type":"result","result":"actual text here",...}
/// Sometimes it outputs multiple lines (duplicates). We take the first valid one.
fn extract_cli_result(raw: &str) -> String {
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(result) = v.get("result").and_then(|r| r.as_str()) {
                return result.to_string();
            }
        }
    }
    // Fallback: return raw output
    raw.to_string()
}

/// Build the planner system prompt dynamically based on available executor workers.
pub(crate) fn build_planner_system_prompt(executors: &[&WorkerConfig]) -> String {
    // Build worker description lines
    let mut worker_lines = String::new();
    for w in executors {
        let mode_label = match w.mode {
            crate::config::WorkerMode::Cli => "CLI",
            crate::config::WorkerMode::Api => "API",
        };
        let skills_str = if w.skills.is_empty() {
            "通用".to_string()
        } else {
            w.skills.join(", ")
        };
        worker_lines.push_str(&format!(
            "- **{}** (类型: `{}`, 模式: {}, 技能: {})\n",
            w.name, w.cli_type, mode_label, skills_str
        ));
    }

    // Collect unique cli_types
    let cli_types: Vec<String> = {
        let mut seen = HashSet::new();
        executors.iter()
            .filter(|w| seen.insert(w.cli_type.clone()))
            .map(|w| format!("\"{}\"", w.cli_type))
            .collect()
    };
    let cli_type_enum = cli_types.join("、");
    let example_cli_type = executors.first().map(|w| w.cli_type.as_str()).unwrap_or("claude_cli");

    format!(
        r#"你现在接到了一个任务编排的工作！请将用户的目标拆解为可以由不同 AI CLI 工具执行的子任务。

## 可用的执行者（Workers）

{worker_lines}
## 输出格式

请直接返回 JSON（不要用 markdown 代码块包裹），格式如下：

{{
  "goal": "用户的原始目标",
  "tasks": [
    {{
      "id": "task-1",
      "description": "这个子任务做什么的简短描述",
      "cli_type": "{example_cli_type}",
      "depends_on": [],
      "prompt": "发送给 CLI 工具的详细提示词",
      "execution_mode": "independent"
    }}
  ]
}}

## 字段规则

- **id**: 唯一标识符，使用 "task-1"、"task-2" 等
- **description**: 简短描述（<80字符），用于 DAG 可视化展示
- **cli_type**: 必须是以下之一：{cli_type_enum}，根据任务性质和工作者技能选择最合适的
- **depends_on**: 依赖的任务 ID 数组，无依赖则为空数组，无依赖的任务可以并行执行
- **prompt**: 完整、独立的提示词，要足够详细让 CLI 工具能无歧义地执行
- **execution_mode**: "independent"（独立执行）或 "pipeline"（接收依赖任务的输出作为上下文）

## 注意事项

1. 将目标拆解为 2-8 个子任务，简单目标不要过度拆解
2. 尽量最大化并行度，只在真正需要时才添加依赖
3. 每个 prompt 要自包含，足够详细
4. 编码任务要指定语言、框架、文件路径和预期行为
5. 请只输出 JSON 对象本身，不要附加其他文字说明
6. 只能使用上面列出的工作者类型，不要分配给不存在的工作者
"#)
}

/// Generate a plan using the designated planner worker.
/// Supports both CLI mode and API mode.
pub async fn generate_plan(planner: &WorkerConfig, goal: &str, working_dir: Option<&str>, executors: &[&WorkerConfig]) -> Result<Plan> {
    let system_prompt = build_planner_system_prompt(executors);
    let prompt = format!(
        "{system_prompt}\n\n## User Goal\n\n{goal}"
    );

    match planner.mode {
        crate::config::WorkerMode::Cli => generate_plan_cli(planner, &prompt, working_dir).await,
        crate::config::WorkerMode::Api => generate_plan_api(planner, &prompt).await,
    }
}

async fn generate_plan_cli(planner: &WorkerConfig, prompt: &str, working_dir: Option<&str>) -> Result<Plan> {
    let cli_path = planner.cli_path.as_deref().unwrap_or("claude");

    let mut cmd = spawn_cli(cli_path, working_dir);
    cmd.arg("-p").arg(prompt);
    for arg in &planner.extra_args {
        cmd.arg(arg);
    }
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let output = cmd.output().await
        .map_err(|e| anyhow::anyhow!("Failed to spawn planner CLI '{}': {}", cli_path, e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        let detail = if !stderr.is_empty() { stderr.to_string() }
            else if !stdout.is_empty() { format!("(stdout) {}", &stdout[..stdout.len().min(500)]) }
            else { format!("exit code: {:?}", output.status.code()) };
        anyhow::bail!("Planner CLI failed: {}", detail);
    }

    parser::parse_plan(&stdout)
}

async fn generate_plan_api(planner: &WorkerConfig, prompt: &str) -> Result<Plan> {
    let base_url = planner.api_base_url.as_deref()
        .unwrap_or("https://api.openai.com");
    let api_key = planner.api_key.as_deref()
        .ok_or_else(|| anyhow::anyhow!("Planner API key not configured"))?;
    let model = planner.model.as_deref().unwrap_or("gpt-4o");

    let client = reqwest::Client::new();

    let is_anthropic = planner.cli_type == "anthropic";

    let response = if is_anthropic {
        let url = format!("{}/v1/messages", base_url.trim_end_matches('/'));
        client.post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": model,
                "max_tokens": 4096,
                "messages": [{"role": "user", "content": prompt}]
            }))
            .send().await?
    } else {
        let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
        client.post(&url)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": model,
                "messages": [{"role": "user", "content": prompt}]
            }))
            .send().await?
    };

    let status = response.status();
    let body = response.text().await?;

    if !status.is_success() {
        anyhow::bail!("Planner API returned {}: {}", status, &body[..body.len().min(500)]);
    }

    let json: serde_json::Value = serde_json::from_str(&body)?;

    let content = if is_anthropic {
        json["content"][0]["text"].as_str()
            .unwrap_or("")
            .to_string()
    } else {
        json["choices"][0]["message"]["content"].as_str()
            .unwrap_or("")
            .to_string()
    };

    parser::parse_plan(&content)
}

/// Chat-based plan generation with conversation history.
/// Used for multi-round planner interaction.
pub async fn chat_plan(
    planner: &WorkerConfig,
    messages: &[ChatMessage],
    working_dir: Option<&str>,
    executors: &[&WorkerConfig],
) -> Result<String> {
    let system_prompt = build_planner_system_prompt(executors);
    match planner.mode {
        crate::config::WorkerMode::Cli => chat_plan_cli(planner, &system_prompt, messages, working_dir).await,
        crate::config::WorkerMode::Api => chat_plan_api(planner, &system_prompt, messages).await,
    }
}

/// Validate that all tasks in a plan use cli_types that exist in the executor list.
pub fn validate_plan(plan: &Plan, executors: &[&WorkerConfig]) -> Result<()> {
    let valid_types: HashSet<&str> = executors.iter().map(|w| w.cli_type.as_str()).collect();
    for task in &plan.tasks {
        let task_type = serde_json::to_value(&task.cli_type)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        if !valid_types.contains(task_type.as_str()) {
            let available: Vec<&str> = valid_types.iter().copied().collect();
            anyhow::bail!(
                "Task '{}' uses cli_type '{}' which is not available. Available types: {:?}",
                task.id, task_type, available
            );
        }
    }
    Ok(())
}

/// CLI mode: concatenate history into a single prompt and send via `-p`
async fn chat_plan_cli(
    planner: &WorkerConfig,
    system_prompt: &str,
    messages: &[ChatMessage],
    working_dir: Option<&str>,
) -> Result<String> {
    let cli_path = planner.cli_path.as_deref().unwrap_or("claude");

    // Build a single prompt containing system instructions + conversation history
    let mut prompt = format!("{system_prompt}\n\n");
    for msg in messages {
        let label = if msg.role == "user" { "User" } else { "Assistant" };
        prompt.push_str(&format!("{label}: {}\n\n", msg.content));
    }

    let mut cmd = spawn_cli(cli_path, working_dir);
    cmd.arg("-p").arg(&prompt);
    for arg in &planner.extra_args {
        cmd.arg(arg);
    }
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let output = cmd.output().await
        .map_err(|e| anyhow::anyhow!("Failed to spawn planner CLI '{}': {}", cli_path, e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        let detail = if !stderr.is_empty() { stderr.to_string() }
            else if !stdout.is_empty() { format!("(stdout) {}", &stdout[..stdout.len().min(500)]) }
            else { format!("exit code: {:?}", output.status.code()) };
        anyhow::bail!("Planner CLI failed: {}", detail);
    }

    Ok(extract_cli_result(&stdout))
}

/// API mode: send messages array directly
async fn chat_plan_api(
    planner: &WorkerConfig,
    system_prompt: &str,
    messages: &[ChatMessage],
) -> Result<String> {
    let base_url = planner.api_base_url.as_deref()
        .unwrap_or("https://api.openai.com");
    let api_key = planner.api_key.as_deref()
        .ok_or_else(|| anyhow::anyhow!("Planner API key not configured"))?;
    let model = planner.model.as_deref().unwrap_or("gpt-4o");

    let client = reqwest::Client::new();
    let is_anthropic = planner.cli_type == "anthropic";

    // Prepend system prompt as first user message if needed
    let api_messages: Vec<serde_json::Value> = {
        let mut msgs = vec![serde_json::json!({
            "role": "user",
            "content": system_prompt,
        })];
        // If the first real message is also "user", we need an assistant ack in between
        if messages.first().map_or(false, |m| m.role == "user") {
            msgs.push(serde_json::json!({
                "role": "assistant",
                "content": "好的，我会按照这个格式来拆解任务。请告诉我你的目标。",
            }));
        }
        for msg in messages {
            msgs.push(serde_json::json!({
                "role": msg.role,
                "content": msg.content,
            }));
        }
        msgs
    };

    let response = if is_anthropic {
        let url = format!("{}/v1/messages", base_url.trim_end_matches('/'));
        client.post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": model,
                "max_tokens": 4096,
                "messages": api_messages,
            }))
            .send().await?
    } else {
        let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
        client.post(&url)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": model,
                "messages": api_messages,
            }))
            .send().await?
    };

    let status = response.status();
    let body = response.text().await?;

    if !status.is_success() {
        anyhow::bail!("Planner API returned {}: {}", status, &body[..body.len().min(500)]);
    }

    let json: serde_json::Value = serde_json::from_str(&body)?;

    let content = if is_anthropic {
        json["content"][0]["text"].as_str().unwrap_or("").to_string()
    } else {
        json["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string()
    };

    Ok(content)
}
