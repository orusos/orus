//! Session audit logger with replay support.
//!
//! Every agent action is logged as a JSON-Lines entry to
//! `~/.orus/data/audit/session-{timestamp}.log`. Sessions can be replayed
//! by reading the log file and re-dispatching actions in order.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
}

/// A single auditable action performed by an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// ISO 8601 timestamp.
    pub timestamp: DateTime<Utc>,
    /// Identifier of the agent that performed the action.
    pub agent_id: String,
    /// The app targeted by this action.
    pub target_app: String,
    /// Category of action (e.g. "click", "navigate", "read_ir", "fill").
    pub action_type: String,
    /// Action-specific payload (serialized as JSON value).
    pub payload: serde_json::Value,
    /// Result of the action ("ok", error message, etc.).
    pub result: ActionResult,
    /// Hash of the app state before the action.
    pub state_hash_before: String,
    /// Hash of the app state after the action.
    pub state_hash_after: String,
}

/// Outcome of an audited action.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum ActionResult {
    #[serde(rename = "ok")]
    Ok {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    #[serde(rename = "denied")]
    Denied { reason: String },
    #[serde(rename = "error")]
    Error { message: String },
}

/// Session audit logger. Writes JSON-Lines to a session log file.
pub struct AuditLogger {
    log_file: File,
    log_path: PathBuf,
    entry_count: u64,
}

impl AuditLogger {
    /// Open a new session log file at `audit_dir/session-{timestamp}.log`.
    ///
    /// Creates the audit directory if it doesn't exist.
    pub fn new_session(audit_dir: &Path) -> Result<Self, AuditError> {
        fs::create_dir_all(audit_dir)?;

        let now = Utc::now();
        let filename = format!("session-{}.log", now.format("%Y-%m-%dT%H-%M-%S"));
        let log_path = audit_dir.join(&filename);

        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        log::info!("audit: session log opened at {}", log_path.display());

        Ok(Self {
            log_file,
            log_path,
            entry_count: 0,
        })
    }

    /// Log an audit entry. Writes a single JSON line and flushes.
    pub fn log(&mut self, entry: &AuditEntry) -> Result<(), AuditError> {
        let json = serde_json::to_string(entry)?;
        writeln!(self.log_file, "{json}")?;
        self.log_file.flush()?;
        self.entry_count += 1;
        Ok(())
    }

    /// Path to the current session log file.
    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    /// Number of entries written so far.
    pub fn entry_count(&self) -> u64 {
        self.entry_count
    }
}

/// Read a session log file and return all entries (for replay).
pub fn read_session_log(path: &Path) -> Result<Vec<AuditEntry>, AuditError> {
    let text = fs::read_to_string(path)?;
    let mut entries = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let entry: AuditEntry = serde_json::from_str(line)?;
        entries.push(entry);
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_entry() -> AuditEntry {
        AuditEntry {
            timestamp: Utc::now(),
            agent_id: "agent-001".into(),
            target_app: "notes".into(),
            action_type: "click".into(),
            payload: serde_json::json!({"element": "save-button", "x": 120, "y": 45}),
            result: ActionResult::Ok { detail: None },
            state_hash_before: "abc123".into(),
            state_hash_after: "def456".into(),
        }
    }

    #[test]
    fn create_session_log() {
        let tmp = TempDir::new().unwrap();
        let audit_dir = tmp.path().join("audit");
        let logger = AuditLogger::new_session(&audit_dir).unwrap();
        assert!(logger.log_path().exists());
        assert_eq!(logger.entry_count(), 0);
    }

    #[test]
    fn write_and_read_entries() {
        let tmp = TempDir::new().unwrap();
        let audit_dir = tmp.path().join("audit");

        let mut logger = AuditLogger::new_session(&audit_dir).unwrap();
        let entry1 = sample_entry();
        let entry2 = AuditEntry {
            action_type: "navigate".into(),
            payload: serde_json::json!({"url": "https://example.com"}),
            result: ActionResult::Denied {
                reason: "no network permission".into(),
            },
            ..sample_entry()
        };

        logger.log(&entry1).unwrap();
        logger.log(&entry2).unwrap();
        assert_eq!(logger.entry_count(), 2);

        // Replay
        let path = logger.log_path().to_path_buf();
        let replayed = read_session_log(&path).unwrap();
        assert_eq!(replayed.len(), 2);
        assert_eq!(replayed[0].action_type, "click");
        assert_eq!(replayed[1].action_type, "navigate");
    }

    #[test]
    fn entry_roundtrip() {
        let entry = sample_entry();
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: AuditEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.agent_id, "agent-001");
        assert_eq!(parsed.target_app, "notes");
    }

    #[test]
    fn action_result_variants() {
        let ok = ActionResult::Ok {
            detail: Some("200 OK".into()),
        };
        let denied = ActionResult::Denied {
            reason: "sandbox violation".into(),
        };
        let error = ActionResult::Error {
            message: "timeout".into(),
        };

        let ok_json = serde_json::to_string(&ok).unwrap();
        assert!(ok_json.contains("\"status\":\"ok\""));
        let denied_json = serde_json::to_string(&denied).unwrap();
        assert!(denied_json.contains("\"status\":\"denied\""));
        let error_json = serde_json::to_string(&error).unwrap();
        assert!(error_json.contains("\"status\":\"error\""));
    }

    #[test]
    fn empty_log_parses_ok() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("empty.log");
        std::fs::write(&path, "").unwrap();
        let entries = read_session_log(&path).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn state_hash_preserved() {
        let entry = sample_entry();
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: AuditEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.state_hash_before, "abc123");
        assert_eq!(parsed.state_hash_after, "def456");
    }
}
