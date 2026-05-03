use std::collections::HashMap;
use std::path::PathBuf;

/// 讀寫 %APPDATA%\Keynova\config.toml，回退至 default_config.toml。
pub struct ConfigManager {
    data: HashMap<String, String>,
    config_path: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Self {
        let config_path = Self::user_config_path();
        let data = Self::load(&config_path);
        Self { data, config_path }
    }

    fn user_config_path() -> PathBuf {
        let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
        PathBuf::from(base).join("Keynova").join("config.toml")
    }

    fn load(path: &PathBuf) -> HashMap<String, String> {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(table) = content.parse::<toml::Table>() {
                return flatten_table(&table, "");
            }
        }
        let default_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("default_config.toml");
        if let Ok(content) = std::fs::read_to_string(&default_path) {
            if let Ok(table) = content.parse::<toml::Table>() {
                return flatten_table(&table, "");
            }
        }
        HashMap::new()
    }

    /// 取得單一設定值。
    pub fn get(&self, key: &str) -> Option<String> {
        self.data.get(key).cloned()
    }

    /// 更新設定值並寫回磁碟。
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        self.data.insert(key.to_string(), value.to_string());
        self.persist()
    }

    /// 回傳所有設定的 (key, value) 清單，依 key 排序。
    pub fn list_all(&self) -> Vec<(String, String)> {
        let mut pairs: Vec<_> = self.data.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        pairs
    }

    fn persist(&self) -> Result<(), String> {
        let dir = self.config_path.parent()
            .ok_or_else(|| "invalid config path".to_string())?;
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let mut sorted: Vec<_> = self.data.iter().collect();
        sorted.sort_by_key(|(k, _)| k.as_str());
        let content: String = sorted.iter()
            .map(|(k, v)| format!("{} = {:?}\n", k, v))
            .collect();
        std::fs::write(&self.config_path, content).map_err(|e| e.to_string())
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

fn flatten_table(table: &toml::Table, prefix: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (key, value) in table {
        let full_key = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{}.{}", prefix, key)
        };
        match value {
            toml::Value::Table(t) => map.extend(flatten_table(t, &full_key)),
            toml::Value::String(s) => { map.insert(full_key, s.clone()); }
            toml::Value::Integer(i) => { map.insert(full_key, i.to_string()); }
            toml::Value::Float(f) => { map.insert(full_key, f.to_string()); }
            toml::Value::Boolean(b) => { map.insert(full_key, b.to_string()); }
            _ => {}
        }
    }
    map
}