use crate::core::{CommandHandler, CommandResult};
use crate::managers::{
    terminal_manager::{start_prewarm, TerminalManager},
    workspace_manager::WorkspaceManager,
};
use crate::models::ipc_requests::{
    TerminalOpenRequest, TerminalResizeRequest, TerminalSendRequest, TerminalSessionRequest,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

pub struct TerminalHandler {
    manager: Arc<Mutex<TerminalManager>>,
    workspace_manager: Arc<Mutex<WorkspaceManager>>,
}

impl TerminalHandler {
    pub fn new(
        manager: Arc<Mutex<TerminalManager>>,
        workspace_manager: Arc<Mutex<WorkspaceManager>>,
    ) -> Self {
        Self {
            manager,
            workspace_manager,
        }
    }
}

impl CommandHandler for TerminalHandler {
    fn namespace(&self) -> &'static str {
        "terminal"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "open" => {
                let req: TerminalOpenRequest = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid terminal.open request: {e}"))?;
                let (id, initial_output) = {
                    let mut mgr = self.manager.lock().map_err(|e| e.to_string())?;
                    match req.launch_spec.as_ref() {
                        Some(spec) => mgr.create_pty_with_command(spec, req.rows, req.cols)?,
                        None => mgr.create_pty(req.rows, req.cols)?,
                    }
                }; // MutexGuard dropped here — start_prewarm can lock without deadlock
                if let Ok(mut workspace) = self.workspace_manager.lock() {
                    workspace.record_terminal_session(id.clone());
                }
                if req.launch_spec.is_none() {
                    start_prewarm(Arc::clone(&self.manager));
                }
                Ok(json!({ "id": id, "initial_output": initial_output }))
            }
            "send" => {
                let req: TerminalSendRequest = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid terminal.send request: {e}"))?;
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                mgr.write_to_pty(&req.id, &req.input)?;
                Ok(Value::Null)
            }
            "close" => {
                let req: TerminalSessionRequest = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid terminal.close request: {e}"))?;
                let mut mgr = self.manager.lock().map_err(|e| e.to_string())?;
                mgr.close_pty(&req.id)?;
                if let Ok(mut workspace) = self.workspace_manager.lock() {
                    workspace.remove_terminal_session(&req.id);
                }
                Ok(Value::Null)
            }
            "resize" => {
                let req: TerminalResizeRequest = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid terminal.resize request: {e}"))?;
                let mut mgr = self.manager.lock().map_err(|e| e.to_string())?;
                mgr.resize_pty(&req.id, req.rows, req.cols)?;
                Ok(Value::Null)
            }
            _ => Err(format!("terminal: unknown command '{command}'")),
        }
    }
}