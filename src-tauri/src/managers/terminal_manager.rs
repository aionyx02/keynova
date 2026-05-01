use crate::models::terminal::{TerminalSession, TerminalStatus};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

struct PtyEntry {
    /// 包在 Arc<Mutex<>> 內，讓 write_to_pty 可用 &self 安全呼叫。
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    session: TerminalSession,
}

/// 管理多個 PTY 終端 Session 的生命週期。
pub struct TerminalManager {
    sessions: HashMap<String, PtyEntry>,
    /// 事件推送回呼：(terminal_id, output_chunk)
    on_output: Arc<dyn Fn(String, String) + Send + Sync>,
}

impl TerminalManager {
    pub fn new(on_output: Arc<dyn Fn(String, String) + Send + Sync>) -> Self {
        Self {
            sessions: HashMap::new(),
            on_output,
        }
    }

    /// 建立新的 PTY 行程並開始讀取輸出，回傳 terminal_id。
    pub fn create_pty(&mut self) -> Result<String, String> {
        let id = Uuid::new_v4().to_string();
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;

        let shell = default_shell();
        let cmd = CommandBuilder::new(&shell);
        pair.slave
            .spawn_command(cmd)
            .map_err(|e| e.to_string())?;

        let writer = Arc::new(Mutex::new(
            pair.master.take_writer().map_err(|e| e.to_string())?,
        ));
        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;

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
                writer,
                session: TerminalSession {
                    id: id.clone(),
                    rows: 24,
                    cols: 80,
                    status: TerminalStatus::Running,
                },
            },
        );
        Ok(id)
    }

    pub fn write_to_pty(&self, id: &str, input: &str) -> Result<(), String> {
        let entry = self
            .sessions
            .get(id)
            .ok_or_else(|| format!("terminal '{id}' not found"))?;
        let mut writer = entry.writer.lock().map_err(|e| e.to_string())?;
        writer.write_all(input.as_bytes()).map_err(|e| e.to_string())
    }

    pub fn resize_pty(&self, id: &str, rows: u16, cols: u16) -> Result<(), String> {
        let entry = self
            .sessions
            .get(id)
            .ok_or_else(|| format!("terminal '{id}' not found"))?;
        // PTY master resize is handled via portable-pty's MasterPty::resize()；
        // currently the master is consumed into writer — a future refactor will retain it.
        let _ = (&entry.session, rows, cols);
        Ok(())
    }

    pub fn close_pty(&mut self, id: &str) -> Result<(), String> {
        self.sessions
            .remove(id)
            .ok_or_else(|| format!("terminal '{id}' not found"))?;
        Ok(())
    }
}

fn default_shell() -> String {
    #[cfg(target_os = "windows")]
    return std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into());
    #[cfg(not(target_os = "windows"))]
    return std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
}
