use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;
use crate::config::WorkerConfig;
use crate::worker::{CliAdapter, TaskOutput, OutputLine, build_prompt};

pub struct AnthropicApiAdapter {
    name: String,
    cli_type: String,
    base_url: String,
    api_key: String,
    model: String,
}

impl AnthropicApiAdapter {
    pub fn new(config: &WorkerConfig) -> Self {
        Self {
            name: config.name.clone(),
            cli_type: config.cli_type.clone(),
            base_url: config.api_base_url.clone().unwrap_or_else(|| "https://api.anthropic.com".into()),
            api_key: config.api_key.clone().unwrap_or_default(),
            model: config.model.clone().unwrap_or_else(|| "claude-sonnet-4-20250514".into()),
        }
    }
}

#[async_trait]
impl CliAdapter for AnthropicApiAdapter {
    async fn execute_streaming(
        &self,
        prompt: &str,
        context: Option<&str>,
        _working_dir: Option<&str>,
        line_tx: mpsc::UnboundedSender<OutputLine>,
    ) -> Result<TaskOutput> {
        let full_prompt = build_prompt(prompt, context);
        let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));

        let _ = line_tx.send(OutputLine {
            source: self.name.clone(),
            text: format!("[API] Calling Anthropic model={}", self.model),
            is_stderr: false,
        });

        let client = reqwest::Client::new();
        let response = client.post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": &self.model,
                "max_tokens": 4096,
                "messages": [{"role": "user", "content": &full_prompt}]
            }))
            .send().await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            let err_msg = format!("Anthropic API error {}: {}", status, &body[..body.len().min(300)]);
            let _ = line_tx.send(OutputLine {
                source: self.name.clone(),
                text: err_msg.clone(),
                is_stderr: true,
            });
            return Ok(TaskOutput {
                stdout: String::new(),
                stderr: err_msg,
                success: false,
            });
        }

        let json: serde_json::Value = serde_json::from_str(&body)?;
        let content = json["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        for line in content.lines() {
            let _ = line_tx.send(OutputLine {
                source: self.name.clone(),
                text: line.to_string(),
                is_stderr: false,
            });
        }

        Ok(TaskOutput {
            stdout: content,
            stderr: String::new(),
            success: true,
        })
    }

    fn name(&self) -> &str { &self.name }
    fn cli_type(&self) -> &str { &self.cli_type }
}
