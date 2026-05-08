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
        self.resolve_named_note(name)
    }

    pub fn notes_root(&self) -> PathBuf {
        self.storage_dir.clone()
    }

    pub fn resolve_named_note(&self, name: &str) -> PathBuf {
        let safe = sanitize_name(name);
        self.storage_dir.join(format!("{safe}.{}", self.extension))
    }

    pub fn create_parent_dirs_for_file(&self, path: &std::path::Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        Ok(())
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
        validate_note_name(name)?;
        let path = self.note_path(name);
        std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// 建立新筆記（名稱已存在時回傳錯誤）。
    pub fn create(&self, name: &str) -> Result<(), String> {
        validate_note_name(name)?;
        let path = self.note_path(name);
        if path.exists() {
            return Err(format!("note '{}' already exists", name));
        }
        std::fs::write(&path, "").map_err(|e| e.to_string())
    }

    /// 儲存筆記內容（自動建立）。
    pub fn save(&self, name: &str, content: &str) -> Result<(), String> {
        validate_note_name(name)?;
        let path = self.note_path(name);
        self.create_parent_dirs_for_file(&path)?;
        std::fs::write(&path, content).map_err(|e| e.to_string())
    }

    /// 刪除筆記。
    pub fn delete(&self, name: &str) -> Result<(), String> {
        validate_note_name(name)?;
        let path = self.note_path(name);
        if !path.exists() {
            return Err(format!("note '{}' not found", name));
        }
        std::fs::remove_file(&path).map_err(|e| e.to_string())
    }

    /// 重新命名筆記。
    pub fn rename(&self, old_name: &str, new_name: &str) -> Result<(), String> {
        validate_note_name(old_name)?;
        validate_note_name(new_name)?;
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

fn validate_note_name(name: &str) -> Result<(), String> {
    let safe = sanitize_name(name);
    if safe.is_empty() {
        return Err("note name cannot be empty".into());
    }
    let upper = safe.to_ascii_uppercase();
    const RESERVED: &[&str] = &["CON", "PRN", "AUX", "NUL"];
    if RESERVED.contains(&upper.as_str()) {
        return Err(format!("'{safe}' is a reserved name"));
    }
    for prefix in ["COM", "LPT"] {
        if upper.len() == 4 && upper.starts_with(prefix) && upper.as_bytes()[3].is_ascii_digit() {
            return Err(format!("'{safe}' is a reserved name"));
        }
    }
    Ok(())
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

    #[test]
    fn validate_empty_names_rejected() {
        // These sanitize to "" (empty string → Err)
        assert!(validate_note_name("").is_err());
        assert!(validate_note_name("   ").is_err());
        // Symbols sanitize to underscores ("___"), which is non-empty and accepted
        assert!(validate_note_name("!!!").is_ok());
    }

    #[test]
    fn validate_reserved_names_rejected() {
        for name in &["CON", "con", "Con", "PRN", "AUX", "NUL"] {
            assert!(validate_note_name(name).is_err(), "{name} should be rejected");
        }
        for name in &["COM1", "com9", "LPT1", "lpt3"] {
            assert!(validate_note_name(name).is_err(), "{name} should be rejected");
        }
    }

    #[test]
    fn validate_valid_names_accepted() {
        assert!(validate_note_name("my note").is_ok());
        assert!(validate_note_name("hello-world_123").is_ok());
        assert!(validate_note_name("CONSOLE").is_ok());
        assert!(validate_note_name("COM10").is_ok());
        assert!(validate_note_name("lpt").is_ok());
    }
}
