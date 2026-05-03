use crate::core::{CommandHandler, CommandResult};
use crate::managers::terminal_manager::{start_prewarm, TerminalManager};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

/// 終端浮窗的 IPC 處理器。
pub struct TerminalHandler {
    manager: Arc<Mutex<TerminalManager>>,
}

impl TerminalHandler {
    pub fn new(manager: Arc<Mutex<TerminalManager>>) -> Self {
        Self { manager }
    }
}

impl CommandHandler for TerminalHandler {
    fn namespace(&self) -> &'static str {
        "terminal"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "open" => {
                let rows = payload.get("rows").and_then(Value::as_u64).unwrap_or(24) as u16;
                let cols = payload.get("cols").and_then(Value::as_u64).unwrap_or(80) as u16;
                let (id, initial_output) = {
                    let mut mgr = self.manager.lock().map_err(|e| e.to_string())?;
                    mgr.create_pty(rows, cols)?
                }; // MutexGuard dropped here — start_prewarm can lock without deadlock
                start_prewarm(Arc::clone(&self.manager));
                Ok(json!({ "id": id, "initial_output": initial_output }))
            }
            "send" => {
                let id = payload["id"]
                    .as_str()
                    .ok_or("missing field: id")?
                    .to_string();
                let input = payload["input"]
                    .as_str()
                    .ok_or("missing field: input")?
                    .to_string();
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                mgr.write_to_pty(&id, &input)?;
                Ok(Value::Null)
            }
            "close" => {
                let id = payload["id"]
                    .as_str()
                    .ok_or("missing field: id")?
                    .to_string();
                let mut mgr = self.manager.lock().map_err(|e| e.to_string())?;
                mgr.close_pty(&id)?;
                Ok(Value::Null)
            }
            "resize" => {
                let id = payload["id"]
                    .as_str()
                    .ok_or("missing field: id")?
                    .to_string();
                let rows = payload["rows"].as_u64().ok_or("missing field: rows")? as u16;
                let cols = payload["cols"].as_u64().ok_or("missing field: cols")? as u16;
                let mut mgr = self.manager.lock().map_err(|e| e.to_string())?;
                mgr.resize_pty(&id, rows, cols)?;
                Ok(Value::Null)
            }
            _ => Err(format!("terminal: unknown command '{command}'")),
        }
    }
}
