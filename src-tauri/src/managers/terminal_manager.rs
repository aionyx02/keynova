use crate::models::terminal::{TerminalLaunchSpec, TerminalSession, TerminalStatus};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

struct PtyEntry {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    session: TerminalSession,
}

pub struct TerminalManager {
    sessions: HashMap<String, PtyEntry>,
    on_output: Arc<dyn Fn(String, String) + Send + Sync>,
    pending_launches: HashMap<String, TerminalLaunchSpec>,
}

impl TerminalManager {
    pub fn new(on_output: Arc<dyn Fn(String, String) + Send + Sync>) -> Self {
        Self {
            sessions: HashMap::new(),
            on_output,
            pending_launches: HashMap::new(),
        }
    }

    pub fn create_pty_with_command(
        &mut self,
        launch: &TerminalLaunchSpec,
        rows: u16,
        cols: u16,
    ) -> Result<(String, String), String> {
        let mut cmd = CommandBuilder::new(&launch.program);
        configure_terminal_env(&mut cmd);
        for arg in &launch.args {
            cmd.arg(arg);
        }
        for env in &launch.env {
            cmd.env(&env.key, &env.value);
        }
        if let Some(cwd) = launch.cwd.as_deref().filter(|cwd| !cwd.is_empty()) {
            cmd.cwd(cwd);
        }
        self.create_pty_from_command(rows, cols, cmd)
    }

    pub fn register_launch_spec(&mut self, launch: TerminalLaunchSpec) -> Result<(), String> {
        if launch.launch_id.trim().is_empty() {
            return Err("terminal launch_id cannot be empty".into());
        }
        self.pending_launches
            .insert(launch.launch_id.clone(), launch);
        Ok(())
    }

    pub fn consume_registered_launch_spec(
        &mut self,
        launch: &TerminalLaunchSpec,
    ) -> Result<(), String> {
        let Some(registered) = self.pending_launches.remove(&launch.launch_id) else {
            return Err("terminal launch spec was not issued by the backend".into());
        };
        if registered != *launch {
            return Err("terminal launch spec does not match the backend-issued command".into());
        }
        Ok(())
    }

    fn create_pty_from_command(
        &mut self,
        rows: u16,
        cols: u16,
        cmd: CommandBuilder,
    ) -> Result<(String, String), String> {
        let id = Uuid::new_v4().to_string();
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;

        let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;

        let writer = Arc::new(Mutex::new(
            pair.master.take_writer().map_err(|e| e.to_string())?,
        ));
        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let master = pair.master;

        let id_clone = id.clone();
        let on_output = Arc::clone(&self.on_output);
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let text = String::from_utf8_lossy(&buf[..n]).into_owned();
                        on_output(id_clone.clone(), text);
                    }
                }
            }
        });

        self.sessions.insert(
            id.clone(),
            PtyEntry {
                master,
                child,
                writer,
                session: TerminalSession {
                    id: id.clone(),
                    rows,
                    cols,
                    status: TerminalStatus::Running,
                },
            },
        );
        Ok((id, String::new()))
    }

    pub fn write_to_pty(&self, id: &str, input: &str) -> Result<(), String> {
        let entry = self
            .sessions
            .get(id)
            .ok_or_else(|| format!("terminal '{id}' not found"))?;
        let mut writer = entry.writer.lock().map_err(|e| e.to_string())?;
        writer
            .write_all(input.as_bytes())
            .map_err(|e| e.to_string())
    }

    pub fn resize_pty(&mut self, id: &str, rows: u16, cols: u16) -> Result<(), String> {
        let entry = self
            .sessions
            .get_mut(id)
            .ok_or_else(|| format!("terminal '{id}' not found"))?;
        entry
            .master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;
        entry.session.rows = rows;
        entry.session.cols = cols;
        Ok(())
    }

    pub fn close_pty(&mut self, id: &str) -> Result<(), String> {
        let mut entry = self
            .sessions
            .remove(id)
            .ok_or_else(|| format!("terminal '{id}' not found"))?;
        let _ = entry.child.kill();
        Ok(())
    }
}

fn configure_terminal_env(cmd: &mut CommandBuilder) {
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("TERM_PROGRAM", "Keynova");

    #[cfg(target_os = "windows")]
    {
        let existing = std::env::var("WSLENV").unwrap_or_default();
        let mut entries: Vec<&str> = existing.split(':').filter(|s| !s.is_empty()).collect();
        for entry in ["TERM/u", "COLORTERM/u", "TERM_PROGRAM/u"] {
            if !entries
                .iter()
                .any(|value| value.eq_ignore_ascii_case(entry))
            {
                entries.push(entry);
            }
        }
        cmd.env("WSLENV", entries.join(":"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::terminal::TerminalEnvVar;

    fn manager() -> TerminalManager {
        TerminalManager::new(Arc::new(|_, _| {}))
    }

    fn spec() -> TerminalLaunchSpec {
        TerminalLaunchSpec {
            launch_id: "launch-1".into(),
            program: "nvim".into(),
            args: vec!["note.md".into()],
            cwd: Some("notes".into()),
            title: Some("Note".into()),
            env: vec![TerminalEnvVar {
                key: "NVIM_APPNAME".into(),
                value: "keynova-lazyvim".into(),
            }],
            editor: true,
        }
    }

    #[test]
    fn registered_launch_spec_can_be_consumed_once() {
        let mut manager = manager();
        let launch = spec();

        manager
            .register_launch_spec(launch.clone())
            .expect("register launch spec");

        assert!(manager.consume_registered_launch_spec(&launch).is_ok());
        assert!(manager.consume_registered_launch_spec(&launch).is_err());
    }

    #[test]
    fn unregistered_launch_spec_is_rejected() {
        let mut manager = manager();

        let error = manager
            .consume_registered_launch_spec(&spec())
            .expect_err("unregistered spec must be rejected");

        assert!(error.contains("not issued"));
    }

    #[test]
    fn mutated_launch_spec_is_rejected() {
        let mut manager = manager();
        let launch = spec();
        manager
            .register_launch_spec(launch.clone())
            .expect("register launch spec");

        let mut mutated = launch;
        mutated.program = "powershell.exe".into();
        mutated.args = vec!["-NoLogo".into(), "-Command".into(), "calc".into()];

        let error = manager
            .consume_registered_launch_spec(&mutated)
            .expect_err("mutated spec must be rejected");

        assert!(error.contains("does not match"));
    }
}
