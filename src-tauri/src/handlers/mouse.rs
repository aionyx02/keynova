use crate::core::{CommandHandler, CommandResult};
use crate::managers::mouse_manager::MouseManager;
use serde_json::Value;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

/// 鍵盤滑鼠控制的 IPC 處理器。
pub struct MouseHandler {
    manager: Arc<Mutex<MouseManager>>,
    active: Arc<AtomicBool>,
}

impl MouseHandler {
    pub fn new(manager: Arc<Mutex<MouseManager>>, active: Arc<AtomicBool>) -> Self {
        Self { manager, active }
    }

    fn ensure_active(&self) -> Result<(), String> {
        if self.active.load(Ordering::Relaxed) {
            Ok(())
        } else {
            Err("mouse control mode is not active".into())
        }
    }
}

impl CommandHandler for MouseHandler {
    fn namespace(&self) -> &'static str {
        "mouse"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "move_relative" => {
                self.ensure_active()?;
                let dx = payload["dx"].as_i64().ok_or("missing field: dx")? as i32;
                let dy = payload["dy"].as_i64().ok_or("missing field: dy")? as i32;
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                mgr.move_cursor_relative(dx, dy)?;
                Ok(Value::Null)
            }
            "move_absolute" => {
                self.ensure_active()?;
                let x = payload["x"].as_i64().ok_or("missing field: x")? as i32;
                let y = payload["y"].as_i64().ok_or("missing field: y")? as i32;
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                mgr.move_cursor_absolute(x, y)?;
                Ok(Value::Null)
            }
            "click" => {
                self.ensure_active()?;
                let button = payload["button"]
                    .as_str()
                    .ok_or("missing field: button")?
                    .to_string();
                let count = payload["count"].as_u64().unwrap_or(1) as u32;
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                mgr.simulate_click(&button, count)?;
                Ok(Value::Null)
            }
            "press_key" => {
                self.ensure_active()?;
                let key = payload["key"]
                    .as_str()
                    .ok_or("missing field: key")?
                    .to_string();
                let modifiers: Vec<String> = payload["modifiers"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                mgr.simulate_key(&key, &modifiers)?;
                Ok(Value::Null)
            }
            "type" => {
                self.ensure_active()?;
                let text = payload["text"]
                    .as_str()
                    .ok_or("missing field: text")?
                    .to_string();
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                mgr.type_text(&text)?;
                Ok(Value::Null)
            }
            _ => Err(format!("mouse: unknown command '{command}'")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn mouse_actions_require_active_mode() {
        let handler = MouseHandler::new(
            Arc::new(Mutex::new(MouseManager::new())),
            Arc::new(AtomicBool::new(false)),
        );

        let error = handler
            .execute("move_relative", json!({ "dx": 1, "dy": 1 }))
            .expect_err("inactive mouse mode must reject HID actions");

        assert!(error.contains("mouse control mode is not active"));
    }
}
