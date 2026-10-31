use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::Result;
use chrono::Utc;
use orus_sandbox::audit::{ActionResult, AuditEntry, AuditLogger};
use orus_sandbox::file_broker::FileBroker;
use orus_sandbox::network_broker::{NetworkBroker, NetworkConfig};
use serde::Serialize;

use crate::paths;

pub const SYSTEM_APP_ID: &str = "system";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    HostFileOpen,
    HostFileSave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogAccess {
    Open,
    Save,
}

impl DialogAccess {
    fn capability(self) -> Capability {
        match self {
            Self::Open => Capability::HostFileOpen,
            Self::Save => Capability::HostFileSave,
        }
    }

    fn action_type(self) -> &'static str {
        match self {
            Self::Open => "host_file_open",
            Self::Save => "host_file_save",
        }
    }
}

pub struct SandboxContext {
    root: PathBuf,
    legacy_root: PathBuf,
    file_broker: FileBroker,
    network_broker: NetworkBroker,
    capabilities: HashMap<String, HashSet<Capability>>,
    audit: Mutex<Option<AuditLogger>>,
}

impl SandboxContext {
    pub fn new(root: PathBuf) -> Result<Self> {
        Self::new_with_legacy_root(root, paths::legacy_orus_data_dir())
    }

    fn new_with_legacy_root(root: PathBuf, legacy_root: PathBuf) -> Result<Self> {
        for dir in [
            root.join("apps"),
            root.join("config"),
            root.join("downloads"),
            root.join("shared"),
            root.join("data"),
            root.join("data").join(SYSTEM_APP_ID),
            root.join("data").join("audit"),
        ] {
            std::fs::create_dir_all(&dir)?;
        }

        let file_broker = FileBroker::new(&root);
        let network_config = NetworkConfig::load(&root.join("config/network.toml"));
        let network_broker = NetworkBroker::with_config(&network_config);
        let audit = AuditLogger::new_session(&root.join("data/audit")).ok();

        let mut capabilities = HashMap::new();
        capabilities.insert(
            "detir-browser".to_string(),
            HashSet::from([Capability::HostFileOpen, Capability::HostFileSave]),
        );
        capabilities.insert(
            "ir-app-workflow-editor".to_string(),
            HashSet::from([Capability::HostFileOpen, Capability::HostFileSave]),
        );

        Ok(Self {
            root,
            legacy_root,
            file_broker,
            network_broker,
            capabilities,
            audit: Mutex::new(audit),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn apps_dir(&self) -> PathBuf {
        self.root.join("apps")
    }

    pub fn shared_dir(&self) -> PathBuf {
        self.root.join("shared")
    }

    pub fn system_dir(&self) -> PathBuf {
        self.root.join("data").join(SYSTEM_APP_ID)
    }

    pub fn default_terminal_cwd(&self) -> PathBuf {
        self.shared_dir()
    }

    pub fn network_broker(&self) -> &NetworkBroker {
        &self.network_broker
    }

    pub fn load_system_json<T: serde::de::DeserializeOwned>(&self, file_name: &str) -> Option<T> {
        self.load_json_at(
            SYSTEM_APP_ID,
            self.system_dir().join(file_name),
            self.legacy_system_path(file_name),
        )
    }

    pub fn save_system_json<T: Serialize>(&self, file_name: &str, value: &T) {
        self.save_json_at(SYSTEM_APP_ID, self.system_dir().join(file_name), value);
    }

    pub fn load_app_json<T: serde::de::DeserializeOwned>(
        &self,
        app_id: &str,
        file_name: &str,
    ) -> Option<T> {
        self.load_json_at(
            app_id,
            self.app_data_dir(app_id).join(file_name),
            self.legacy_app_path(app_id, file_name),
        )
    }

    pub fn save_app_json<T: Serialize>(&self, app_id: &str, file_name: &str, value: &T) {
        self.save_json_at(app_id, self.app_data_dir(app_id).join(file_name), value);
    }

    pub fn sanitize_app_read_path(&self, app_id: &str, path: &Path) -> Option<PathBuf> {
        self.file_broker.check_read(app_id, path).ok()
    }

    pub fn authorize_file_dialog(
        &self,
        app_id: &str,
        access: DialogAccess,
        selected_path: Option<PathBuf>,
    ) -> Option<PathBuf> {
        let Some(path) = selected_path else {
            self.audit(
                app_id,
                access.action_type(),
                serde_json::json!({"status": "cancelled"}),
                ActionResult::Ok {
                    detail: Some("cancelled".into()),
                },
            );
            return None;
        };

        if !self.has_capability(app_id, access.capability()) {
            self.audit(
                app_id,
                access.action_type(),
                serde_json::json!({"path": path.display().to_string()}),
                ActionResult::Denied {
                    reason: format!("missing capability {:?}", access.capability()),
                },
            );
            return None;
        }

        let audit_payload = serde_json::json!({
            "path": path.display().to_string(),
            "inside_sandbox": path.starts_with(&self.root),
        });

        if path.starts_with(&self.root) {
            let result = match access {
                DialogAccess::Open => self.file_broker.check_read(app_id, &path).map(|_| ()),
                DialogAccess::Save => self.file_broker.check_write(app_id, &path).map(|_| ()),
            };
            if let Err(err) = result {
                self.audit(
                    app_id,
                    access.action_type(),
                    audit_payload,
                    ActionResult::Denied {
                        reason: err.to_string(),
                    },
                );
                return None;
            }
        }

        self.audit(
            app_id,
            access.action_type(),
            audit_payload,
            ActionResult::Ok {
                detail: Some("granted".into()),
            },
        );
        Some(path)
    }

    fn has_capability(&self, app_id: &str, capability: Capability) -> bool {
        self.capabilities
            .get(app_id)
            .is_some_and(|caps| caps.contains(&capability))
    }

    fn app_data_dir(&self, app_id: &str) -> PathBuf {
        self.root.join("data").join(app_id)
    }

    fn legacy_system_path(&self, file_name: &str) -> PathBuf {
        self.legacy_root.join(file_name)
    }

    fn legacy_app_path(&self, app_id: &str, file_name: &str) -> PathBuf {
        self.legacy_root.join("apps").join(app_id).join(file_name)
    }

    fn load_json_at<T: serde::de::DeserializeOwned>(
        &self,
        app_id: &str,
        preferred_path: PathBuf,
        legacy_path: PathBuf,
    ) -> Option<T> {
        self.read_json_path(app_id, &preferred_path)
            .or_else(|| self.read_legacy_json_path(&legacy_path))
    }

    fn read_json_path<T: serde::de::DeserializeOwned>(
        &self,
        app_id: &str,
        path: &Path,
    ) -> Option<T> {
        let path = self.file_broker.check_read(app_id, path).ok()?;
        let data = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&data).ok()
    }

    fn read_legacy_json_path<T: serde::de::DeserializeOwned>(&self, path: &Path) -> Option<T> {
        let data = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }

    fn save_json_at<T: Serialize>(&self, app_id: &str, path: PathBuf, value: &T) {
        let Ok(path) = self.file_broker.check_write(app_id, &path) else {
            self.audit(
                app_id,
                "broker_write_denied",
                serde_json::json!({"path": path.display().to_string()}),
                ActionResult::Denied {
                    reason: "write denied by broker".into(),
                },
            );
            return;
        };

        if let Some(parent) = path.parent()
            && let Err(err) = std::fs::create_dir_all(parent)
        {
            log::warn!("Failed to create {}: {err}", parent.display());
            return;
        }

        match serde_json::to_string_pretty(value) {
            Ok(json) => {
                if let Err(err) = std::fs::write(&path, json) {
                    log::warn!("Failed to write {}: {err}", path.display());
                    self.audit(
                        app_id,
                        "broker_write",
                        serde_json::json!({"path": path.display().to_string()}),
                        ActionResult::Error {
                            message: err.to_string(),
                        },
                    );
                } else {
                    self.audit(
                        app_id,
                        "broker_write",
                        serde_json::json!({"path": path.display().to_string()}),
                        ActionResult::Ok { detail: None },
                    );
                }
            }
            Err(err) => log::warn!("Failed to serialize {} state: {err}", app_id),
        }
    }

    fn audit(
        &self,
        app_id: &str,
        action_type: &str,
        payload: serde_json::Value,
        result: ActionResult,
    ) {
        let Ok(mut guard) = self.audit.lock() else {
            return;
        };
        let Some(logger) = guard.as_mut() else {
            return;
        };
        let _ = logger.log(&AuditEntry {
            timestamp: Utc::now(),
            agent_id: "orus-shell".into(),
            target_app: app_id.to_string(),
            action_type: action_type.to_string(),
            payload,
            result,
            state_hash_before: String::new(),
            state_hash_after: String::new(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, SandboxContext) {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join(".orus");
        let legacy = tmp.path().join(".orus");
        std::fs::create_dir_all(legacy.join("apps/terminal")).unwrap();
        std::fs::write(
            legacy.join("apps/terminal/state.json"),
            r#"{"cwd":"/tmp/legacy"}"#,
        )
        .unwrap();
        let sandbox = SandboxContext::new_with_legacy_root(root, legacy).unwrap();
        (tmp, sandbox)
    }

    #[test]
    fn app_state_roundtrip_uses_brokered_data_dir() {
        let (_tmp, sandbox) = setup();
        let value = serde_json::json!({"url":"detir://home"});
        sandbox.save_app_json("detir-browser", "state.json", &value);

        let saved = sandbox
            .load_app_json::<serde_json::Value>("detir-browser", "state.json")
            .unwrap();
        assert_eq!(saved["url"], "detir://home");
        assert!(
            sandbox
                .root()
                .join("data/detir-browser/state.json")
                .exists()
        );
    }

    #[test]
    fn legacy_app_state_is_migrated_on_read() {
        let (tmp, sandbox) = setup();
        let loaded = sandbox
            .load_app_json::<serde_json::Value>("terminal", "state.json")
            .unwrap();
        assert_eq!(loaded["cwd"], "/tmp/legacy");
        assert!(
            tmp.path()
                .join(".orus/apps/terminal/state.json")
                .exists()
        );
    }

    #[test]
    fn file_dialog_requires_capability() {
        let (_tmp, sandbox) = setup();
        let path = sandbox.authorize_file_dialog(
            "terminal",
            DialogAccess::Open,
            Some(PathBuf::from("/tmp/host-file.workflow.json")),
        );
        assert!(path.is_none());
    }
}
