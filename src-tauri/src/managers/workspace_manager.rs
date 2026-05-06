use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const SLOT_COUNT: usize = 3;
const CURRENT_WORKSPACE_VERSION: u32 = 2;

/// 單一工作區的狀態快照。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceState {
    #[serde(default = "workspace_version")]
    pub version: u32,
    pub id: usize,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub project_root: Option<String>,
    pub query: String,
    pub mode: String,
    #[serde(default)]
    pub panel: Option<String>,
    #[serde(default)]
    pub recent_actions: Vec<String>,
    #[serde(default)]
    pub recent_files: Vec<String>,
    #[serde(default)]
    pub terminal_sessions: Vec<String>,
    #[serde(default)]
    pub note_ids: Vec<String>,
    #[serde(default)]
    pub ai_conversation_ids: Vec<String>,
}

impl WorkspaceState {
    fn empty(id: usize) -> Self {
        Self {
            version: CURRENT_WORKSPACE_VERSION,
            id,
            name: format!("Workspace {}", id + 1),
            project_root: None,
            query: String::new(),
            mode: "search".into(),
            panel: None,
            recent_actions: Vec::new(),
            recent_files: Vec::new(),
            terminal_sessions: Vec::new(),
            note_ids: Vec::new(),
            ai_conversation_ids: Vec::new(),
        }
    }
}

/// 管理最多 3 個虛擬工作區的切換與持久化。
pub struct WorkspaceManager {
    slots: [WorkspaceState; SLOT_COUNT],
    current: usize,
    persist_path: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct PersistData {
    #[serde(default = "workspace_version")]
    version: u32,
    current: usize,
    slots: Vec<WorkspaceState>,
}

impl WorkspaceManager {
    pub fn new() -> Self {
        let persist_path = Self::default_path();
        let (current, slots) = Self::load(&persist_path);
        Self {
            slots,
            current,
            persist_path,
        }
    }

    fn default_path() -> PathBuf {
        let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
        PathBuf::from(base).join("Keynova").join("workspaces.json")
    }

    fn load(path: &PathBuf) -> (usize, [WorkspaceState; SLOT_COUNT]) {
        let default = (
            0usize,
            [
                WorkspaceState::empty(0),
                WorkspaceState::empty(1),
                WorkspaceState::empty(2),
            ],
        );
        let Ok(content) = std::fs::read_to_string(path) else {
            return default;
        };
        let Ok(data) = serde_json::from_str::<PersistData>(&content) else {
            return default;
        };
        let current = data.current.min(SLOT_COUNT - 1);
        let mut slots = [
            WorkspaceState::empty(0),
            WorkspaceState::empty(1),
            WorkspaceState::empty(2),
        ];
        for mut s in data.slots {
            if s.id < SLOT_COUNT {
                migrate_workspace_state(&mut s, data.version);
                let id = s.id;
                slots[id] = s;
            }
        }
        (current, slots)
    }

    fn persist(&self) {
        let data = PersistData {
            version: CURRENT_WORKSPACE_VERSION,
            current: self.current,
            slots: self.slots.to_vec(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&data) {
            if let Some(parent) = self.persist_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&self.persist_path, json);
        }
    }

    /// 切換到指定槽位（0-based），回傳新的工作區狀態。
    pub fn switch_to(&mut self, slot: usize) -> Result<&WorkspaceState, String> {
        if slot >= SLOT_COUNT {
            return Err(format!(
                "invalid workspace slot {slot} (max {})",
                SLOT_COUNT - 1
            ));
        }
        self.current = slot;
        self.persist();
        Ok(&self.slots[self.current])
    }

    /// 儲存當前工作區的查詢與模式（由前端在切換前呼叫）。
    pub fn save_current_restore_state(
        &mut self,
        query: String,
        mode: String,
        panel: Option<String>,
        project_root: Option<String>,
    ) {
        let slot = &mut self.slots[self.current];
        slot.query = query;
        slot.mode = mode;
        slot.panel = panel;
        if project_root.as_deref().is_some_and(|root| !root.is_empty()) {
            slot.project_root = project_root;
        }
        self.persist();
    }

    pub fn record_action(&mut self, action_id: String) {
        push_recent(&mut self.slots[self.current].recent_actions, action_id, 25);
        self.persist();
    }

    pub fn record_file(&mut self, path: String) {
        push_recent(&mut self.slots[self.current].recent_files, path, 25);
        self.persist();
    }

    pub fn record_terminal_session(&mut self, session_id: String) {
        push_recent(
            &mut self.slots[self.current].terminal_sessions,
            session_id,
            10,
        );
        self.persist();
    }

    pub fn remove_terminal_session(&mut self, session_id: &str) {
        self.slots[self.current]
            .terminal_sessions
            .retain(|id| id != session_id);
        self.persist();
    }

    pub fn record_note(&mut self, note_id: String) {
        push_recent(&mut self.slots[self.current].note_ids, note_id, 50);
        self.persist();
    }

    pub fn remove_note(&mut self, note_id: &str) {
        self.slots[self.current].note_ids.retain(|id| id != note_id);
        self.persist();
    }

    pub fn record_ai_conversation(&mut self, conversation_id: String) {
        push_recent(
            &mut self.slots[self.current].ai_conversation_ids,
            conversation_id,
            50,
        );
        self.persist();
    }

    pub fn current(&self) -> &WorkspaceState {
        &self.slots[self.current]
    }

    pub fn all(&self) -> &[WorkspaceState; SLOT_COUNT] {
        &self.slots
    }
}

fn workspace_version() -> u32 {
    CURRENT_WORKSPACE_VERSION
}

fn migrate_workspace_state(state: &mut WorkspaceState, persisted_version: u32) {
    if state.name.trim().is_empty() {
        state.name = format!("Workspace {}", state.id + 1);
    }
    if persisted_version < CURRENT_WORKSPACE_VERSION {
        state.version = CURRENT_WORKSPACE_VERSION;
    }
}

fn push_recent(list: &mut Vec<String>, value: String, limit: usize) {
    if value.trim().is_empty() {
        return;
    }
    list.retain(|item| item != &value);
    list.insert(0, value);
    list.truncate(limit);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mgr() -> WorkspaceManager {
        WorkspaceManager {
            slots: [
                WorkspaceState::empty(0),
                WorkspaceState::empty(1),
                WorkspaceState::empty(2),
            ],
            current: 0,
            persist_path: PathBuf::from("/dev/null"),
        }
    }

    #[test]
    fn switch_workspace() {
        let mut mgr = make_mgr();
        mgr.switch_to(1).unwrap();
        assert_eq!(mgr.current().id, 1);
    }

    #[test]
    fn switch_out_of_bounds() {
        let mut mgr = make_mgr();
        assert!(mgr.switch_to(5).is_err());
    }

    #[test]
    fn save_and_restore() {
        let mut mgr = make_mgr();
        mgr.save_current_restore_state("hello".into(), "search".into(), None, None);
        assert_eq!(mgr.current().query, "hello");
    }

    #[test]
    fn records_workspace_bindings() {
        let mut mgr = make_mgr();
        mgr.record_terminal_session("pty-1".into());
        mgr.record_note("daily".into());
        mgr.record_ai_conversation("chat-1".into());
        assert_eq!(mgr.current().terminal_sessions, vec!["pty-1"]);
        assert_eq!(mgr.current().note_ids, vec!["daily"]);
        assert_eq!(mgr.current().ai_conversation_ids, vec!["chat-1"]);

        mgr.remove_terminal_session("pty-1");
        mgr.remove_note("daily");
        assert!(mgr.current().terminal_sessions.is_empty());
        assert!(mgr.current().note_ids.is_empty());
    }
}
