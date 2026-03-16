use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::dag::Plan;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub goal: String,
    pub plan: Plan,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: HistoryStatus,
    pub task_results: Vec<TaskResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HistoryStatus {
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub status: String,
    pub output: Option<String>,
}

/// Summary for listing (without full output data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistorySummary {
    pub id: String,
    pub goal: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: HistoryStatus,
    pub task_count: usize,
}

fn history_dir() -> PathBuf {
    let dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ai-orch")
        .join("history");
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn entry_path(id: &str) -> PathBuf {
    history_dir().join(format!("{id}.json"))
}

pub fn generate_id() -> String {
    let now = chrono_lite_now();
    format!("run-{now}")
}

pub fn save(entry: &HistoryEntry) -> anyhow::Result<()> {
    let path = entry_path(&entry.id);
    let json = serde_json::to_string_pretty(entry)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn load(id: &str) -> anyhow::Result<HistoryEntry> {
    let path = entry_path(id);
    let json = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&json)?)
}

pub fn list_all() -> Vec<HistorySummary> {
    let dir = history_dir();
    let mut entries: Vec<HistorySummary> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
        .filter_map(|e| {
            let json = std::fs::read_to_string(e.path()).ok()?;
            let entry: HistoryEntry = serde_json::from_str(&json).ok()?;
            Some(HistorySummary {
                id: entry.id,
                goal: entry.goal,
                started_at: entry.started_at,
                finished_at: entry.finished_at,
                status: entry.status,
                task_count: entry.plan.tasks.len(),
            })
        })
        .collect();
    entries.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    entries
}

pub fn delete(id: &str) -> anyhow::Result<()> {
    let path = entry_path(id);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

/// Simple timestamp without chrono dependency
fn chrono_lite_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs();
    // Format as compact timestamp
    format!("{secs}")
}

pub fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs();
    // Approximate ISO format (good enough for display)
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Days since epoch to Y-M-D (simplified)
    let mut y = 1970i64;
    let mut remaining = days as i64;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let month_days = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 1u32;
    for &md in &month_days {
        if remaining < md {
            break;
        }
        remaining -= md;
        m += 1;
    }
    let d = remaining + 1;
    format!("{y:04}-{m:02}-{d:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}
