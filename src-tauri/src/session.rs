use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    pub role: String, // "user" | "assistant"
    pub content: String,
}

#[derive(Debug)]
pub struct Session {
    pub id: String,
    pub messages: Vec<ChatMessage>,
    pub working_dir: Option<String>,
}

#[derive(Debug, Default)]
pub struct SessionManager {
    sessions: HashMap<String, Session>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    /// Create a new session and return its id
    pub fn create_session(&mut self, working_dir: Option<String>) -> String {
        let id = format!("session-{}", uuid_v4());
        let session = Session {
            id: id.clone(),
            messages: Vec::new(),
            working_dir,
        };
        self.sessions.insert(id.clone(), session);
        id
    }

    /// Add a message to a session
    pub fn add_message(&mut self, session_id: &str, role: &str, content: &str) -> Result<(), String> {
        let session = self.sessions.get_mut(session_id)
            .ok_or_else(|| format!("Session not found: {session_id}"))?;
        session.messages.push(ChatMessage {
            role: role.to_string(),
            content: content.to_string(),
        });
        Ok(())
    }

    /// Get the message history for a session
    pub fn get_history(&self, session_id: &str) -> Result<&[ChatMessage], String> {
        let session = self.sessions.get(session_id)
            .ok_or_else(|| format!("Session not found: {session_id}"))?;
        Ok(&session.messages)
    }

    /// Get the working directory for a session
    pub fn get_working_dir(&self, session_id: &str) -> Result<Option<&str>, String> {
        let session = self.sessions.get(session_id)
            .ok_or_else(|| format!("Session not found: {session_id}"))?;
        Ok(session.working_dir.as_deref())
    }

    /// Remove a session
    pub fn remove_session(&mut self, session_id: &str) {
        self.sessions.remove(session_id);
    }
}

/// Simple UUID v4 generator (timestamp + random-ish)
fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:016x}-{:04x}", ts, (ts >> 64) as u16 ^ 0xbeef)
}
