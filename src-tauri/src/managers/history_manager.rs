use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// 剪貼簿歷史條目。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub id: String,
    pub content: String,
    pub content_type: String,
    pub timestamp: u64,
    pub pinned: bool,
}

/// 管理剪貼簿歷史：去重、容量上限、搜尋、釘選、持久化。
pub struct HistoryManager {
    entries: Vec<ClipboardEntry>,
    max_items: usize,
    persist_path: PathBuf,
    last_content_hash: u64,
}

impl HistoryManager {
    pub fn new(max_items: usize) -> Self {
        let path = Self::default_path();
        let entries = Self::load(&path);
        Self {
            entries,
            max_items,
            persist_path: path,
            last_content_hash: 0,
        }
    }

    fn default_path() -> PathBuf {
        let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
        PathBuf::from(base)
            .join("Keynova")
            .join("clipboard_history.json")
    }

    fn load(path: &PathBuf) -> Vec<ClipboardEntry> {
        let Ok(content) = std::fs::read_to_string(path) else {
            return vec![];
        };
        serde_json::from_str(&content).unwrap_or_default()
    }

    fn persist(&self) {
        if let Ok(json) = serde_json::to_string_pretty(&self.entries) {
            if let Some(p) = self.persist_path.parent() {
                let _ = std::fs::create_dir_all(p);
            }
            let _ = std::fs::write(&self.persist_path, json);
        }
    }

    fn simple_hash(s: &str) -> u64 {
        // FNV-1a
        let mut h: u64 = 14695981039346656037;
        for b in s.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(1099511628211);
        }
        h
    }

    /// 嘗試新增一條剪貼簿記錄。若內容與上次相同（去重），回傳 false。
    pub fn push_text(&mut self, content: String) -> bool {
        if content.trim().is_empty() {
            return false;
        }
        let hash = Self::simple_hash(&content);
        if hash == self.last_content_hash {
            return false;
        }
        self.last_content_hash = hash;

        // 去重：移除已存在的相同內容
        self.entries.retain(|e| e.content != content);

        let id = uuid::Uuid::new_v4().to_string();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        self.entries.insert(
            0,
            ClipboardEntry {
                id,
                content,
                content_type: "text".into(),
                timestamp,
                pinned: false,
            },
        );

        // 保持容量：釘選的在末尾保留，非釘選的刪除
        while self.entries.iter().filter(|e| !e.pinned).count() > self.max_items {
            // 找最後一個非釘選條目刪除
            if let Some(idx) = self.entries.iter().rposition(|e| !e.pinned) {
                self.entries.remove(idx);
            } else {
                break;
            }
        }

        self.persist();
        true
    }

    pub fn list(&self) -> &[ClipboardEntry] {
        &self.entries
    }

    pub fn search(&self, q: &str) -> Vec<&ClipboardEntry> {
        let q_low = q.to_lowercase();
        self.entries
            .iter()
            .filter(|e| e.content.to_lowercase().contains(&q_low))
            .collect()
    }

    pub fn delete(&mut self, id: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.id != id);
        let changed = self.entries.len() != before;
        if changed {
            self.persist();
        }
        changed
    }

    pub fn pin(&mut self, id: &str, pinned: bool) -> bool {
        if let Some(e) = self.entries.iter_mut().find(|e| e.id == id) {
            e.pinned = pinned;
            self.persist();
            true
        } else {
            false
        }
    }

    pub fn clear_unpinned(&mut self) {
        self.entries.retain(|e| e.pinned);
        self.persist();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mgr() -> HistoryManager {
        HistoryManager {
            entries: vec![],
            max_items: 5,
            persist_path: PathBuf::from("/dev/null"),
            last_content_hash: 0,
        }
    }

    #[test]
    fn push_and_dedup() {
        let mut m = make_mgr();
        assert!(m.push_text("hello".into()));
        assert!(!m.push_text("hello".into()));
        assert_eq!(m.list().len(), 1);
    }

    #[test]
    fn search() {
        let mut m = make_mgr();
        m.push_text("hello world".into());
        m.push_text("foo bar".into());
        assert_eq!(m.search("hello").len(), 1);
    }

    #[test]
    fn delete() {
        let mut m = make_mgr();
        m.push_text("test".into());
        let id = m.list()[0].id.clone();
        assert!(m.delete(&id));
        assert!(m.list().is_empty());
    }
}
