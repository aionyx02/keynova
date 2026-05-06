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

    /// 檢查快捷鍵是否與現有設定衝突。
    pub fn check_conflict(&self, config: &HotkeyConfig) -> Option<ConflictInfo> {
        self.registered
            .values()
            .find(|existing| {
                existing.id != config.id
                    && existing.key == config.key
                    && existing.modifiers == config.modifiers
            })
            .map(|existing| ConflictInfo {
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
