use crate::core::{CommandHandler, CommandResult};
use crate::managers::{terminal_manager::TerminalManager, workspace_manager::WorkspaceManager};
use crate::models::ipc_requests::{
    TerminalOpenRequest, TerminalResizeRequest, TerminalSendRequest, TerminalSessionRequest,
};
use crate::models::terminal::TerminalLaunchSpec;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Default interactive shell program for a human-driven terminal (ADR-0045).
/// Honors the user's configured shell where the OS exposes it.
fn default_shell_program() -> String {
    #[cfg(target_os = "windows")]
    {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
    }
}

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
                        Some(spec) => {
                            mgr.consume_registered_launch_spec(spec)?;
                            mgr.create_pty_with_command(spec, req.rows, req.cols)?
                        }
                        None => {
                            return Err("terminal.open requires a backend-issued launch_spec".into())
                        }
                    }
                }; // MutexGuard dropped here — start_prewarm can lock without deadlock
                if let Ok(mut workspace) = self.workspace_manager.lock() {
                    workspace.record_terminal_session(id.clone());
                }
                Ok(json!({ "id": id, "initial_output": initial_output }))
            }
            "request_shell" => {
                // ADR-0045: issue + register a backend default-shell launch spec for
                // a human-driven interactive terminal. Reachable only via the user's
                // `>` palette gesture; the automation allowlist still blocks
                // `terminal.*`, so no non-human actor can call this. cwd = workspace
                // project root when one is set.
                let cwd = self
                    .workspace_manager
                    .lock()
                    .ok()
                    .and_then(|ws| ws.current().project_root.clone())
                    .filter(|root| !root.trim().is_empty());
                let spec = TerminalLaunchSpec {
                    launch_id: Uuid::new_v4().to_string(),
                    program: default_shell_program(),
                    args: Vec::new(),
                    cwd,
                    title: None,
                    env: Vec::new(),
                    editor: false,
                };
                self.manager
                    .lock()
                    .map_err(|e| e.to_string())?
                    .register_launch_spec(spec.clone())?;
                serde_json::to_value(spec).map_err(|e| e.to_string())
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn terminal_open_requires_backend_issued_launch_spec() {
        let handler = TerminalHandler::new(
            Arc::new(Mutex::new(TerminalManager::new(Arc::new(|_, _| {})))),
            Arc::new(Mutex::new(WorkspaceManager::new())),
        );

        let error = handler
            .execute("open", json!({ "rows": 24, "cols": 80 }))
            .expect_err("default shell open must be rejected");

        assert!(error.contains("backend-issued launch_spec"));
    }

    #[test]
    fn request_shell_issues_a_registered_consumable_spec() {
        // ADR-0045: request_shell must register the spec so the subsequent
        // terminal.open (consume) path accepts it exactly once.
        let mgr = Arc::new(Mutex::new(TerminalManager::new(Arc::new(|_, _| {}))));
        let handler =
            TerminalHandler::new(Arc::clone(&mgr), Arc::new(Mutex::new(WorkspaceManager::new())));

        let value = handler
            .execute("request_shell", json!({}))
            .expect("request_shell should issue a spec");
        let spec: TerminalLaunchSpec =
            serde_json::from_value(value).expect("spec deserializes");
        assert!(!spec.launch_id.is_empty());
        assert!(!spec.editor, "human shell is not an editor session");
        assert!(!spec.program.is_empty());

        // Registered → consumable once, then rejected (one-shot).
        assert!(mgr.lock().unwrap().consume_registered_launch_spec(&spec).is_ok());
        assert!(mgr.lock().unwrap().consume_registered_launch_spec(&spec).is_err());
    }
}
