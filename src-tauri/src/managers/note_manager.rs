use std::path::PathBuf;

use serde::Serialize;

/// 快速筆記管理器：讀寫 AppData/Keynova/notes/ 目錄下的 Markdown 檔案。
pub struct NoteManager {
    storage_dir: PathBuf,
    extension: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NoteMeta {
    pub name: String,
    pub filename: String,
    pub size_bytes: u64,
    pub modified: String,
}

impl NoteManager {
    pub fn new(storage_dir: Option<String>) -> Self {
        let dir = match storage_dir.filter(|s| !s.is_empty()) {
            Some(p) => PathBuf::from(p),
            None => {
                let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
                PathBuf::from(base).join("Keynova").join("notes")
            }
        };
        let _ = std::fs::create_dir_all(&dir);
        Self {
            storage_dir: dir,
            extension: "md".into(),
        }
    }

    fn note_path(&self, name: &str) -> PathBuf {
        let safe = sanitize_name(name);
        self.storage_dir.join(format!("{safe}.{}", self.extension))
    }

    /// 列出所有筆記的元資料，依修改時間降序。
    pub fn list(&self) -> Vec<NoteMeta> {
        let Ok(entries) = std::fs::read_dir(&self.storage_dir) else {
            return vec![];
        };
        let ext = format!(".{}", self.extension);
        let mut metas: Vec<NoteMeta> = entries
            .flatten()
            .filter(|e| e.file_name().to_str().is_some_and(|n| n.ends_with(&ext)))
            .filter_map(|e| {
                let meta = e.metadata().ok()?;
                let filename = e.file_name().to_str()?.to_string();
                let name = filename.trim_end_matches(&ext).to_string();
                let size_bytes = meta.len();
                let modified = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| {
                        let secs = d.as_secs();
                        // ISO 8601 approximation (seconds since epoch as string)
                        format!("{secs}")
                    })
                    .unwrap_or_default();
                Some(NoteMeta {
                    name,
                    filename,
                    size_bytes,
                    modified,
                })
            })
            .collect();
        metas.sort_by(|a, b| b.modified.cmp(&a.modified));
        metas
    }

    /// 讀取筆記內容。
    pub fn get(&self, name: &str) -> Result<String, String> {
        let path = self.note_path(name);
        std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// 建立新筆記（名稱已存在時回傳錯誤）。
    pub fn create(&self, name: &str) -> Result<(), String> {
        let path = self.note_path(name);
        if path.exists() {
            return Err(format!("note '{}' already exists", name));
        }
        std::fs::write(&path, "").map_err(|e| e.to_string())
    }

    /// 儲存筆記內容（自動建立）。
    pub fn save(&self, name: &str, content: &str) -> Result<(), String> {
        let path = self.note_path(name);
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, content).map_err(|e| e.to_string())
    }

    /// 刪除筆記。
    pub fn delete(&self, name: &str) -> Result<(), String> {
        let path = self.note_path(name);
        if !path.exists() {
            return Err(format!("note '{}' not found", name));
        }
        std::fs::remove_file(&path).map_err(|e| e.to_string())
    }

    /// 重新命名筆記。
    pub fn rename(&self, old_name: &str, new_name: &str) -> Result<(), String> {
        let old = self.note_path(old_name);
        let new = self.note_path(new_name);
        if !old.exists() {
            return Err(format!("note '{}' not found", old_name));
        }
        if new.exists() {
            return Err(format!("note '{}' already exists", new_name));
        }
        std::fs::rename(&old, &new).map_err(|e| e.to_string())
    }
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim()
        .replace(' ', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize() {
        assert_eq!(sanitize_name("my note"), "my_note");
        assert_eq!(sanitize_name("a/b\\c"), "a_b_c");
        assert_eq!(sanitize_name("hello-world_123"), "hello-world_123");
    }
}
