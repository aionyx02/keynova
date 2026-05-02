use crate::models::hotkey::{ConflictInfo, HotkeyConfig};
use std::collections::HashMap;

/// 管理全局快捷鍵的註冊、取消與衝突偵測。
pub struct HotkeyManager {
    registered: HashMap<String, HotkeyConfig>,
}

impl HotkeyManager {
    pub fn new() -> Self {
        Self {
            registered: HashMap::new(),
        }
    }

    /// 向作業系統註冊全局快捷鍵。
    pub fn register_global_hotkey(&mut self, config: &HotkeyConfig) -> Result<(), String> {
        if self.check_conflict(config).is_some() {
            return Err(format!(
                "hotkey '{}+{}' conflicts with an existing binding",
                config.modifiers.join("+"),
                config.key
            ));
        }
        #[cfg(target_os = "windows")]
        {
            crate::platform::windows::register_hotkey(config)?;
        }
        self.registered.insert(config.id.clone(), config.clone());
        Ok(())
    }

    /// 取消已註冊的全局快捷鍵。
    pub fn unregister_hotkey(&mut self, id: &str) -> Result<(), String> {
        if self.registered.remove(id).is_none() {
            return Err(format!("hotkey '{id}' not found"));
        }
        #[cfg(target_os = "windows")]
        {
            crate::platform::windows::unregister_hotkey(id)?;
        }
        Ok(())
    }

    /// 檢查快捷鍵是否與現有設定衝突。
    pub fn check_conflict(&self, config: &HotkeyConfig) -> Option<ConflictInfo> {
        self.registered.values().find(|existing| {
            existing.id != config.id
                && existing.key == config.key
                && existing.modifiers == config.modifiers
        }).map(|existing| ConflictInfo {
            conflicting_id: existing.id.clone(),
            description: format!(
                "'{}+{}' is already bound to '{}'",
                existing.modifiers.join("+"),
                existing.key,
                existing.action
            ),
        })
    }

    pub fn list_hotkeys(&self) -> Vec<HotkeyConfig> {
        self.registered.values().cloned().collect()
    }
}
