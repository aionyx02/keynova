use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const SLOT_COUNT: usize = 3;

/// 單一工作區的狀態快照。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub id: usize,
    pub query: String,
    pub mode: String,
}

impl WorkspaceState {
    fn empty(id: usize) -> Self {
        Self { id, query: String::new(), mode: "search".into() }
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
    current: usize,
    slots: Vec<WorkspaceState>,
}

impl WorkspaceManager {
    pub fn new() -> Self {
        let persist_path = Self::default_path();
        let (current, slots) = Self::load(&persist_path);
        Self { slots, current, persist_path }
    }

    fn default_path() -> PathBuf {
        let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
        PathBuf::from(base).join("Keynova").join("workspaces.json")
    }

    fn load(path: &PathBuf) -> (usize, [WorkspaceState; SLOT_COUNT]) {
        let default = (0usize, [
            WorkspaceState::empty(0),
            WorkspaceState::empty(1),
            WorkspaceState::empty(2),
        ]);
        let Ok(content) = std::fs::read_to_string(path) else { return default; };
        let Ok(data) = serde_json::from_str::<PersistData>(&content) else { return default; };
        let current = data.current.min(SLOT_COUNT - 1);
        let mut slots = [WorkspaceState::empty(0), WorkspaceState::empty(1), WorkspaceState::empty(2)];
        for s in data.slots {
            if s.id < SLOT_COUNT {
                let id = s.id;
                slots[id] = s;
            }
        }
        (current, slots)
    }

    fn persist(&self) {
        let data = PersistData {
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
            return Err(format!("invalid workspace slot {slot} (max {})", SLOT_COUNT - 1));
        }
        self.current = slot;
        self.persist();
        Ok(&self.slots[self.current])
    }

    /// 儲存當前工作區的查詢與模式（由前端在切換前呼叫）。
    pub fn save_current(&mut self, query: String, mode: String) {
        self.slots[self.current].query = query;
        self.slots[self.current].mode = mode;
        self.persist();
    }

    pub fn current(&self) -> &WorkspaceState {
        &self.slots[self.current]
    }

    pub fn all(&self) -> &[WorkspaceState; SLOT_COUNT] {
        &self.slots
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mgr() -> WorkspaceManager {
        WorkspaceManager {
            slots: [WorkspaceState::empty(0), WorkspaceState::empty(1), WorkspaceState::empty(2)],
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
        mgr.save_current("hello".into(), "search".into());
        assert_eq!(mgr.current().query, "hello");
    }
}