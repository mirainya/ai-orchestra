use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use crate::config::{AppConfig, WorkerConfig, WorkerMode};
use crate::dag::{scheduler, TaskStatus, TaskUpdate, ExecMode, SubTask};
use crate::planner;
use crate::aggregator::Aggregator;
use crate::session::SessionManager;
use crate::worker::pool::{WorkerPool, WorkerStatus};
use crate::worker::OutputLine;
use crate::history::{self, HistoryEntry, HistoryStatus, HistorySummary, TaskResult};

pub struct AppState {
    pub config: AppConfig,
    pub pool: WorkerPool,
    pub sessions: SessionManager,
}

#[tauri::command]
pub async fn submit_task(
    goal: String,
    working_dir: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
    app: AppHandle,
) -> Result<(), String> {
    let config = {
        let s = state.lock().await;
        s.config.clone()
    };

    // Emit planning start
    app.emit("task-update", serde_json::json!({
        "type": "planning_start",
        "goal": &goal,
    })).ok();

    // Find planner worker
    let planner_config = config.find_planner()
        .ok_or_else(|| "No planner worker configured (need a worker with role=planner or role=both)".to_string())?
        .clone();

    let executors = config.executor_workers();

    // Step 1: Call planner (with timeout)
    let planner_timeout = Duration::from_secs(config.execution.planner_timeout_secs);
    let plan = tokio::time::timeout(
        planner_timeout,
        planner::generate_plan(&planner_config, &goal, working_dir.as_deref(), &executors),
    )
    .await
    .map_err(|_| format!("Planning timed out after {}s", config.execution.planner_timeout_secs))?
    .map_err(|e| format!("Planning failed: {e}"))?;

    // Validate plan against available executors
    planner::validate_plan(&plan, &executors)
        .map_err(|e| format!("Plan validation failed: {e}"))?;

    // Emit plan ready
    app.emit("task-update", serde_json::json!({
        "type": "plan_ready",
        "plan": &plan,
    })).ok();

    // Create history entry
    let run_id = history::generate_id();
    let mut hist_entry = HistoryEntry {
        id: run_id.clone(),
        goal: goal.clone(),
        plan: plan.clone(),
        started_at: history::now_iso(),
        finished_at: None,
        status: HistoryStatus::Running,
        task_results: Vec::new(),
    };
    history::save(&hist_entry).ok();

    // Step 2: Schedule
    let levels = scheduler::schedule(&plan).map_err(|e| format!("Scheduling failed: {e}"))?;

    // Step 3: Execute level by level
    let aggregator = Arc::new(Mutex::new(Aggregator::new()));
    let exec_config = config.execution.clone();

    for level in &levels {
        let mut handles = Vec::new();

        for task_id in level {
            let task = plan.tasks.iter().find(|t| &t.id == task_id).unwrap().clone();
            let state_clone = Arc::clone(&*state);
            let app_clone = app.clone();
            let agg_clone = Arc::clone(&aggregator);
            let exec_cfg = exec_config.clone();
            let work_dir = working_dir.clone();

            let handle = tokio::spawn(async move {
                // Create a session for this task
                let task_session_id = {
                    let mut s = state_clone.lock().await;
                    let sid = s.sessions.create_session(work_dir.clone());
                    // Store the task prompt as the first user message
                    s.sessions.add_message(&sid, "user", &task.prompt).ok();
                    sid
                };

                // Emit task running
                app_clone.emit("task-update", serde_json::json!({
                    "type": "task_status",
                    "task_id": &task.id,
                    "status": "running",
                    "session_id": &task_session_id,
                })).ok();

                // Get context from dependencies (pipeline mode)
                let context = {
                    let agg = agg_clone.lock().await;
                    if task.execution_mode == ExecMode::Pipeline {
                        agg.context_for(&task)
                    } else {
                        None
                    }
                };

                // Acquire a worker
                let worker = loop {
                    let pool = &state_clone.lock().await.pool;
                    if let Some(w) = pool.acquire(&task.cli_type.to_string()).await {
                        // Emit worker busy
                        app_clone.emit("worker-update", serde_json::json!({
                            "name": w.adapter.name(),
                            "status": "busy",
                        })).ok();
                        break w;
                    }
                    drop(pool);
                    tokio::time::sleep(Duration::from_millis(500)).await;
                };

                // Execute with retry
                let result = execute_task_with_retry(
                    &task,
                    &worker,
                    context.as_deref(),
                    work_dir.as_deref(),
                    &app_clone,
                    &exec_cfg,
                ).await;

                // Release worker
                let worker_name = worker.adapter.name().to_string();
                {
                    let pool = &state_clone.lock().await.pool;
                    pool.release(&worker_name).await;
                }
                app_clone.emit("worker-update", serde_json::json!({
                    "name": &worker_name,
                    "status": "idle",
                })).ok();

                // Record result
                let update = match &result {
                    Ok(output) => TaskUpdate {
                        task_id: task.id.clone(),
                        status: TaskStatus::Completed,
                        output: Some(output.stdout.clone()),
                    },
                    Err(err) => TaskUpdate {
                        task_id: task.id.clone(),
                        status: TaskStatus::Failed,
                        output: Some(err.clone()),
                    },
                };

                {
                    let mut agg = agg_clone.lock().await;
                    agg.record(&update);
                }

                // Emit final status
                let (status_str, output_str) = match &result {
                    Ok(o) => ("completed", o.stdout.clone()),
                    Err(e) => ("failed", e.clone()),
                };

                // Save task output to its session
                {
                    let mut s = state_clone.lock().await;
                    s.sessions.add_message(&task_session_id, "assistant", &output_str).ok();
                }

                app_clone.emit("task-update", serde_json::json!({
                    "type": "task_status",
                    "task_id": &task.id,
                    "status": status_str,
                    "output": &output_str,
                    "session_id": &task_session_id,
                })).ok();

                TaskResult {
                    task_id: task.id.clone(),
                    status: status_str.to_string(),
                    output: Some(output_str),
                }
            });

            handles.push(handle);
        }

        // Wait for all tasks in this level
        for handle in handles {
            if let Ok(result) = handle.await {
                hist_entry.task_results.push(result);
            }
        }
    }

    // Finalize
    let has_failures = hist_entry.task_results.iter().any(|r| r.status == "failed");
    hist_entry.status = if has_failures { HistoryStatus::Failed } else { HistoryStatus::Completed };
    hist_entry.finished_at = Some(history::now_iso());
    history::save(&hist_entry).ok();

    app.emit("task-update", serde_json::json!({
        "type": "all_done",
        "has_failures": has_failures,
    })).ok();

    Ok(())
}

async fn execute_task_with_retry(
    task: &SubTask,
    worker: &crate::worker::pool::WorkerSlot,
    context: Option<&str>,
    working_dir: Option<&str>,
    app: &AppHandle,
    exec_config: &crate::config::ExecutionConfig,
) -> Result<crate::worker::TaskOutput, String> {
    let max_attempts = exec_config.max_retries + 1;
    let timeout_dur = Duration::from_secs(exec_config.task_timeout_secs);
    let mut last_error = String::new();

    for attempt in 1..=max_attempts {
        if attempt > 1 {
            app.emit("output-line", serde_json::json!({
                "source": &task.id,
                "text": format!("Retry attempt {attempt}/{max_attempts}..."),
                "level": "warn",
            })).ok();
            tokio::time::sleep(Duration::from_secs(exec_config.retry_delay_secs)).await;
        }

        let (line_tx, mut line_rx) = tokio::sync::mpsc::unbounded_channel::<crate::worker::OutputLine>();

        // Forward output lines to frontend
        let app_fwd = app.clone();
        let task_id = task.id.clone();
        let fwd_handle = tokio::spawn(async move {
            while let Some(line) = line_rx.recv().await {
                app_fwd.emit("output-line", serde_json::json!({
                    "source": &task_id,
                    "text": &line.text,
                    "level": if line.is_stderr { "warn" } else { "info" },
                })).ok();
            }
        });

        let result = tokio::time::timeout(
            timeout_dur,
            worker.adapter.execute_streaming(&task.prompt, context, working_dir, line_tx),
        ).await;

        let _ = fwd_handle.await;

        match result {
            Ok(Ok(output)) if output.success => return Ok(output),
            Ok(Ok(output)) => {
                last_error = format!("Task exited with error: {}", output.stderr.trim());
            }
            Ok(Err(e)) => {
                last_error = format!("Execution error: {e}");
            }
            Err(_) => {
                last_error = format!("Task timed out after {}s", exec_config.task_timeout_secs);
            }
        }
    }

    // All attempts failed
    app.emit("task-update", serde_json::json!({
        "type": "task_status",
        "task_id": &task.id,
        "status": "failed",
        "output": &last_error,
    })).ok();
    app.emit("output-line", serde_json::json!({
        "source": &task.id,
        "text": format!("Failed after {max_attempts} attempt(s): {last_error}"),
        "level": "error",
    })).ok();

    Err(last_error)
}

#[tauri::command]
pub async fn get_config(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<AppConfig, String> {
    let s = state.lock().await;
    Ok(s.config.clone())
}

#[tauri::command]
pub async fn get_workers(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<WorkerStatus>, String> {
    let s = state.lock().await;
    Ok(s.pool.status().await)
}

#[tauri::command]
pub async fn save_config(
    config: AppConfig,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let config_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("config.toml");
    config.save(&config_path).map_err(|e| format!("Failed to save config: {e}"))?;

    // Rebuild pool with new config
    let new_pool = WorkerPool::new(&config.executor_workers()
        .into_iter().cloned().collect::<Vec<_>>());
    let mut s = state.lock().await;
    s.config = config;
    s.pool = new_pool;
    Ok(())
}

#[tauri::command]
pub async fn get_history_list() -> Result<Vec<HistorySummary>, String> {
    Ok(history::list_all())
}

#[tauri::command]
pub async fn get_history_entry(id: String) -> Result<HistoryEntry, String> {
    history::load(&id).map_err(|e| format!("Failed to load history: {e}"))
}

#[tauri::command]
pub async fn delete_history_entry(id: String) -> Result<(), String> {
    history::delete(&id).map_err(|e| format!("Failed to delete: {e}"))
}

#[derive(serde::Serialize)]
pub struct TestResult {
    pub success: bool,
    pub message: String,
    pub latency_ms: u64,
}

#[tauri::command]
pub async fn test_worker(worker: WorkerConfig) -> Result<TestResult, String> {
    let start = std::time::Instant::now();
    let test_prompt = "Reply with exactly: OK";

    match worker.mode {
        WorkerMode::Api => {
            let base_url = worker.api_base_url.as_deref().unwrap_or("https://api.openai.com");
            let api_key = match worker.api_key.as_deref() {
                Some(k) if !k.is_empty() => k,
                _ => return Ok(TestResult {
                    success: false,
                    message: "API Key 未配置".into(),
                    latency_ms: 0,
                }),
            };
            let model = worker.model.as_deref().unwrap_or("gpt-4o");
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|e| e.to_string())?;

            let is_anthropic = worker.cli_type == "anthropic";

            let resp = if is_anthropic {
                let url = format!("{}/v1/messages", base_url.trim_end_matches('/'));
                client.post(&url)
                    .header("x-api-key", api_key)
                    .header("anthropic-version", "2023-06-01")
                    .header("content-type", "application/json")
                    .json(&serde_json::json!({
                        "model": model,
                        "max_tokens": 32,
                        "messages": [{"role": "user", "content": test_prompt}]
                    }))
                    .send().await
            } else {
                let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
                client.post(&url)
                    .header("Authorization", format!("Bearer {api_key}"))
                    .header("content-type", "application/json")
                    .json(&serde_json::json!({
                        "model": model,
                        "max_tokens": 32,
                        "messages": [{"role": "user", "content": test_prompt}]
                    }))
                    .send().await
            };

            let latency = start.elapsed().as_millis() as u64;

            match resp {
                Ok(r) => {
                    let status = r.status();
                    if status.is_success() {
                        Ok(TestResult {
                            success: true,
                            message: format!("连接成功 (HTTP {})", status.as_u16()),
                            latency_ms: latency,
                        })
                    } else {
                        let body = r.text().await.unwrap_or_default();
                        let short = if body.len() > 200 { &body[..200] } else { &body };
                        Ok(TestResult {
                            success: false,
                            message: format!("HTTP {} - {}", status.as_u16(), short),
                            latency_ms: latency,
                        })
                    }
                }
                Err(e) => {
                    Ok(TestResult {
                        success: false,
                        message: format!("连接失败: {e}"),
                        latency_ms: start.elapsed().as_millis() as u64,
                    })
                }
            }
        }
        WorkerMode::Cli => {
            let cli_path = worker.cli_path.as_deref().unwrap_or("echo");
            #[cfg(windows)]
            let result = tokio::process::Command::new("cmd")
                .args(["/c", cli_path, "--version"])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output()
                .await;
            #[cfg(not(windows))]
            let result = tokio::process::Command::new(cli_path)
                .arg("--version")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output()
                .await;

            let latency = start.elapsed().as_millis() as u64;

            match result {
                Ok(output) => {
                    if output.status.success() {
                        let ver = String::from_utf8_lossy(&output.stdout);
                        let first_line = ver.lines().next().unwrap_or("").trim();
                        Ok(TestResult {
                            success: true,
                            message: if first_line.is_empty() {
                                "CLI 可用".into()
                            } else {
                                first_line.chars().take(100).collect()
                            },
                            latency_ms: latency,
                        })
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let first = stderr.lines().next().unwrap_or("未知错误").trim();
                        Ok(TestResult {
                            success: false,
                            message: format!("退出码 {}: {}", output.status.code().unwrap_or(-1), first),
                            latency_ms: latency,
                        })
                    }
                }
                Err(e) => {
                    Ok(TestResult {
                        success: false,
                        message: format!("无法启动: {e}"),
                        latency_ms: start.elapsed().as_millis() as u64,
                    })
                }
            }
        }
    }
}

// ─── Planner Multi-Round Chat ───────────────────────────────────────────────

#[tauri::command]
pub async fn start_planning(
    goal: String,
    working_dir: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
    app: AppHandle,
) -> Result<String, String> {
    let (planner_config, session_id) = {
        let mut s = state.lock().await;
        let planner_config = s.config.find_planner()
            .ok_or_else(|| "No planner worker configured".to_string())?
            .clone();
        let session_id = s.sessions.create_session(working_dir);
        // Add the user's goal as the first message
        s.sessions.add_message(&session_id, "user", &goal).map_err(|e| e.to_string())?;
        (planner_config, session_id)
    };

    app.emit("planner-message", serde_json::json!({
        "session_id": &session_id,
        "status": "thinking",
    })).ok();

    // Get working_dir from session
    let working_dir = {
        let s = state.lock().await;
        s.sessions.get_working_dir(&session_id)
            .unwrap_or(None)
            .map(|s| s.to_string())
    };

    // Call planner with conversation history
    let messages = {
        let s = state.lock().await;
        s.sessions.get_history(&session_id)
            .map_err(|e| e.to_string())?
            .to_vec()
    };

    let (planner_timeout, executors) = {
        let s = state.lock().await;
        let timeout = Duration::from_secs(s.config.execution.planner_timeout_secs);
        let executors: Vec<crate::config::WorkerConfig> = s.config.executor_workers().into_iter().cloned().collect();
        (timeout, executors)
    };
    let executor_refs: Vec<&crate::config::WorkerConfig> = executors.iter().collect();

    let response = tokio::time::timeout(
        planner_timeout,
        planner::chat_plan(&planner_config, &messages, working_dir.as_deref(), &executor_refs),
    )
    .await
    .map_err(|_| "Planning timed out".to_string())?
    .map_err(|e| format!("Planning failed: {e}"))?;

    // Save assistant response to session
    {
        let mut s = state.lock().await;
        s.sessions.add_message(&session_id, "assistant", &response)
            .map_err(|e| e.to_string())?;
    }

    app.emit("planner-message", serde_json::json!({
        "session_id": &session_id,
        "status": "awaiting_approval",
        "content": &response,
    })).ok();

    Ok(session_id)
}

#[tauri::command]
pub async fn send_planner_message(
    session_id: String,
    message: String,
    state: State<'_, Arc<Mutex<AppState>>>,
    app: AppHandle,
) -> Result<(), String> {
    // Add user message to session
    {
        let mut s = state.lock().await;
        s.sessions.add_message(&session_id, "user", &message)
            .map_err(|e| e.to_string())?;
    }

    app.emit("planner-message", serde_json::json!({
        "session_id": &session_id,
        "status": "thinking",
    })).ok();

    let (planner_config, messages, working_dir, planner_timeout, executors) = {
        let s = state.lock().await;
        let planner_config = s.config.find_planner()
            .ok_or_else(|| "No planner worker configured".to_string())?
            .clone();
        let messages = s.sessions.get_history(&session_id)
            .map_err(|e| e.to_string())?
            .to_vec();
        let working_dir = s.sessions.get_working_dir(&session_id)
            .unwrap_or(None)
            .map(|s| s.to_string());
        let timeout = Duration::from_secs(s.config.execution.planner_timeout_secs);
        let executors: Vec<crate::config::WorkerConfig> = s.config.executor_workers().into_iter().cloned().collect();
        (planner_config, messages, working_dir, timeout, executors)
    };
    let executor_refs: Vec<&crate::config::WorkerConfig> = executors.iter().collect();

    let response = tokio::time::timeout(
        planner_timeout,
        planner::chat_plan(&planner_config, &messages, working_dir.as_deref(), &executor_refs),
    )
    .await
    .map_err(|_| "Planning timed out".to_string())?
    .map_err(|e| format!("Planning failed: {e}"))?;

    // Save assistant response
    {
        let mut s = state.lock().await;
        s.sessions.add_message(&session_id, "assistant", &response)
            .map_err(|e| e.to_string())?;
    }

    app.emit("planner-message", serde_json::json!({
        "session_id": &session_id,
        "status": "awaiting_approval",
        "content": &response,
    })).ok();

    Ok(())
}

#[tauri::command]
pub async fn approve_plan(
    session_id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
    app: AppHandle,
) -> Result<(), String> {
    // Get the last assistant message and try to parse it as a plan
    let (last_response, working_dir, config) = {
        let s = state.lock().await;
        let messages = s.sessions.get_history(&session_id)
            .map_err(|e| e.to_string())?;
        let last = messages.iter().rev()
            .find(|m| m.role == "assistant")
            .ok_or_else(|| "No plan to approve".to_string())?;
        let wd = s.sessions.get_working_dir(&session_id)
            .unwrap_or(None)
            .map(|s| s.to_string());
        (last.content.clone(), wd, s.config.clone())
    };

    // Parse plan from the last assistant response
    let plan = planner::parser::parse_plan(&last_response)
        .map_err(|e| format!("Failed to parse plan: {e}"))?;

    // Validate plan against available executors
    {
        let executors: Vec<crate::config::WorkerConfig> = config.executor_workers().into_iter().cloned().collect();
        let executor_refs: Vec<&crate::config::WorkerConfig> = executors.iter().collect();
        planner::validate_plan(&plan, &executor_refs)
            .map_err(|e| format!("Plan validation failed: {e}"))?;
    }

    // Emit plan ready
    app.emit("task-update", serde_json::json!({
        "type": "plan_ready",
        "plan": &plan,
    })).ok();

    // Create history entry
    let run_id = history::generate_id();
    let mut hist_entry = HistoryEntry {
        id: run_id.clone(),
        goal: plan.goal.clone(),
        plan: plan.clone(),
        started_at: history::now_iso(),
        finished_at: None,
        status: HistoryStatus::Running,
        task_results: Vec::new(),
    };
    history::save(&hist_entry).ok();

    // Schedule
    let levels = scheduler::schedule(&plan).map_err(|e| format!("Scheduling failed: {e}"))?;

    // Execute (reuse the same logic as submit_task)
    let aggregator = Arc::new(Mutex::new(Aggregator::new()));
    let exec_config = config.execution.clone();

    app.emit("task-update", serde_json::json!({
        "type": "planning_start",
        "goal": &plan.goal,
    })).ok();

    for level in &levels {
        let mut handles = Vec::new();

        for task_id in level {
            let task = plan.tasks.iter().find(|t| &t.id == task_id).unwrap().clone();
            let state_clone = Arc::clone(&*state);
            let app_clone = app.clone();
            let agg_clone = Arc::clone(&aggregator);
            let exec_cfg = exec_config.clone();
            let work_dir = working_dir.clone();

            let handle = tokio::spawn(async move {
                // Create a session for this task
                let task_session_id = {
                    let mut s = state_clone.lock().await;
                    let sid = s.sessions.create_session(work_dir.clone());
                    s.sessions.add_message(&sid, "user", &task.prompt).ok();
                    sid
                };

                app_clone.emit("task-update", serde_json::json!({
                    "type": "task_status",
                    "task_id": &task.id,
                    "status": "running",
                    "session_id": &task_session_id,
                })).ok();

                let context = {
                    let agg = agg_clone.lock().await;
                    if task.execution_mode == ExecMode::Pipeline {
                        agg.context_for(&task)
                    } else {
                        None
                    }
                };

                let worker = loop {
                    let pool = &state_clone.lock().await.pool;
                    if let Some(w) = pool.acquire(&task.cli_type.to_string()).await {
                        app_clone.emit("worker-update", serde_json::json!({
                            "name": w.adapter.name(),
                            "status": "busy",
                        })).ok();
                        break w;
                    }
                    drop(pool);
                    tokio::time::sleep(Duration::from_millis(500)).await;
                };

                let result = execute_task_with_retry(
                    &task,
                    &worker,
                    context.as_deref(),
                    work_dir.as_deref(),
                    &app_clone,
                    &exec_cfg,
                ).await;

                let worker_name = worker.adapter.name().to_string();
                {
                    let pool = &state_clone.lock().await.pool;
                    pool.release(&worker_name).await;
                }
                app_clone.emit("worker-update", serde_json::json!({
                    "name": &worker_name,
                    "status": "idle",
                })).ok();

                let update = match &result {
                    Ok(output) => TaskUpdate {
                        task_id: task.id.clone(),
                        status: TaskStatus::Completed,
                        output: Some(output.stdout.clone()),
                    },
                    Err(err) => TaskUpdate {
                        task_id: task.id.clone(),
                        status: TaskStatus::Failed,
                        output: Some(err.clone()),
                    },
                };

                {
                    let mut agg = agg_clone.lock().await;
                    agg.record(&update);
                }

                let (status_str, output_str) = match &result {
                    Ok(o) => ("completed", o.stdout.clone()),
                    Err(e) => ("failed", e.clone()),
                };

                // Save task output to its session
                {
                    let mut s = state_clone.lock().await;
                    s.sessions.add_message(&task_session_id, "assistant", &output_str).ok();
                }

                app_clone.emit("task-update", serde_json::json!({
                    "type": "task_status",
                    "task_id": &task.id,
                    "status": status_str,
                    "output": &output_str,
                    "session_id": &task_session_id,
                })).ok();

                TaskResult {
                    task_id: task.id.clone(),
                    status: status_str.to_string(),
                    output: Some(output_str),
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            if let Ok(result) = handle.await {
                hist_entry.task_results.push(result);
            }
        }
    }

    let has_failures = hist_entry.task_results.iter().any(|r| r.status == "failed");
    hist_entry.status = if has_failures { HistoryStatus::Failed } else { HistoryStatus::Completed };
    hist_entry.finished_at = Some(history::now_iso());
    history::save(&hist_entry).ok();

    app.emit("task-update", serde_json::json!({
        "type": "all_done",
        "has_failures": has_failures,
    })).ok();

    Ok(())
}

// ─── Task Executor Multi-Round Chat ─────────────────────────────────────────

#[tauri::command]
pub async fn send_task_message(
    session_id: String,
    task_id: String,
    message: String,
    state: State<'_, Arc<Mutex<AppState>>>,
    app: AppHandle,
) -> Result<(), String> {
    // Add user message to session
    {
        let mut s = state.lock().await;
        s.sessions.add_message(&session_id, "user", &message)
            .map_err(|e| e.to_string())?;
    }

    app.emit("task-message", serde_json::json!({
        "task_id": &task_id,
        "role": "user",
        "content": &message,
        "status": "thinking",
    })).ok();

    // Get session context
    let (config, messages, working_dir) = {
        let s = state.lock().await;
        let config = s.config.clone();
        let messages = s.sessions.get_history(&session_id)
            .map_err(|e| e.to_string())?
            .to_vec();
        let working_dir = s.sessions.get_working_dir(&session_id)
            .unwrap_or(None)
            .map(|s| s.to_string());
        (config, messages, working_dir)
    };

    let exec_config = config.execution.clone();
    let timeout_dur = Duration::from_secs(exec_config.task_timeout_secs);

    // Build the full prompt with history
    let mut full_prompt = String::new();
    for msg in &messages {
        let label = if msg.role == "user" { "User" } else { "Assistant" };
        full_prompt.push_str(&format!("{label}: {}\n\n", msg.content));
    }

    // Acquire a worker
    let worker = loop {
        let pool = &state.lock().await.pool;
        if let Some(w) = pool.acquire("claude_cli").await {
            break w;
        }
        drop(pool);
        tokio::time::sleep(Duration::from_millis(500)).await;
    };

    let worker_name = worker.adapter.name().to_string();
    app.emit("worker-update", serde_json::json!({
        "name": &worker_name,
        "status": "busy",
    })).ok();

    // Execute with streaming
    let (line_tx, mut line_rx) = tokio::sync::mpsc::unbounded_channel::<OutputLine>();

    let app_fwd = app.clone();
    let tid = task_id.clone();
    let fwd_handle = tokio::spawn(async move {
        while let Some(line) = line_rx.recv().await {
            app_fwd.emit("output-line", serde_json::json!({
                "source": &tid,
                "text": &line.text,
                "level": if line.is_stderr { "warn" } else { "info" },
            })).ok();
        }
    });

    let result = tokio::time::timeout(
        timeout_dur,
        worker.adapter.execute_streaming(
            &full_prompt,
            None,
            working_dir.as_deref(),
            line_tx,
        ),
    ).await;

    let _ = fwd_handle.await;

    // Release worker
    {
        let pool = &state.lock().await.pool;
        pool.release(&worker_name).await;
    }
    app.emit("worker-update", serde_json::json!({
        "name": &worker_name,
        "status": "idle",
    })).ok();

    let response = match result {
        Ok(Ok(output)) => output.stdout,
        Ok(Err(e)) => return Err(format!("Execution error: {e}")),
        Err(_) => return Err(format!("Task timed out after {}s", exec_config.task_timeout_secs)),
    };

    // Save assistant response to session
    {
        let mut s = state.lock().await;
        s.sessions.add_message(&session_id, "assistant", &response)
            .map_err(|e| e.to_string())?;
    }

    app.emit("task-message", serde_json::json!({
        "task_id": &task_id,
        "role": "assistant",
        "content": &response,
        "status": "done",
    })).ok();

    Ok(())
}
