use crate::models::terminal::{TerminalLaunchSpec, TerminalSession, TerminalStatus};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use uuid::Uuid;

struct PtyEntry {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    session: TerminalSession,
}

/// Pre-warmed PTY session waiting to be claimed by the next terminal.open call.
struct WarmEntry {
    id: String,
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    /// Shell output accumulated before claim (initial prompt, etc.)
    buffer: Arc<Mutex<String>>,
    /// false = buffering mode; true = live EventBus mode
    live: Arc<AtomicBool>,
}

/// 管理多個 PTY 終端 Session 的生命週期。
pub struct TerminalManager {
    sessions: HashMap<String, PtyEntry>,
    on_output: Arc<dyn Fn(String, String) + Send + Sync>,
    warm: Option<WarmEntry>,
    warming: bool,
}

impl TerminalManager {
    pub fn new(on_output: Arc<dyn Fn(String, String) + Send + Sync>) -> Self {
        Self {
            sessions: HashMap::new(),
            on_output,
            warm: None,
            warming: false,
        }
    }

    fn begin_prewarm(&mut self) -> Option<Arc<dyn Fn(String, String) + Send + Sync>> {
        if self.warm.is_some() || self.warming {
            return None;
        }
        self.warming = true;
        Some(Arc::clone(&self.on_output))
    }

    fn finish_prewarm(&mut self, entry: Option<WarmEntry>) {
        self.warming = false;
        if self.warm.is_none() {
            self.warm = entry;
        }
    }

    fn take_warm(&mut self, rows: u16, cols: u16) -> Option<(String, String)> {
        let entry = self.warm.take()?;
        let _ = entry.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
        // Switch reader thread to live EventBus mode
        entry.live.store(true, Ordering::Release);
        let initial = entry.buffer.lock().map(|b| b.clone()).unwrap_or_default();
        self.sessions.insert(
            entry.id.clone(),
            PtyEntry {
                master: entry.master,
                child: entry.child,
                writer: entry.writer,
                session: TerminalSession {
                    id: entry.id.clone(),
                    rows,
                    cols,
                    status: TerminalStatus::Running,
                },
            },
        );
        Some((entry.id, initial))
    }

    /// 建立新的 PTY 行程並開始讀取輸出，回傳 (terminal_id, initial_output)。
    /// 若有 pre-warmed session 則直接取用（零延遲），否則同步建立。
    pub fn create_pty(&mut self, rows: u16, cols: u16) -> Result<(String, String), String> {
        if let Some(result) = self.take_warm(rows, cols) {
            return Ok(result);
        }
        self.create_pty_from_command(rows, cols, terminal_command())
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

fn spawn_prewarm(
    on_output: Arc<dyn Fn(String, String) + Send + Sync>,
) -> Result<WarmEntry, String> {
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

    let cmd = terminal_command();
    let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;

    let writer = Arc::new(Mutex::new(
        pair.master.take_writer().map_err(|e| e.to_string())?,
    ));
    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let master = pair.master;

    let buffer: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let live = Arc::new(AtomicBool::new(false));

    let id_clone = id.clone();
    let buffer_clone = Arc::clone(&buffer);
    let live_clone = Arc::clone(&live);

    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let text = String::from_utf8_lossy(&buf[..n]).into_owned();
                    if live_clone.load(Ordering::Acquire) {
                        on_output(id_clone.clone(), text);
                    } else if let Ok(mut b) = buffer_clone.lock() {
                        b.push_str(&text);
                    }
                }
            }
        }
    });

    Ok(WarmEntry {
        id,
        master,
        child,
        writer,
        buffer,
        live,
    })
}

/// Kick off a background pre-warm. Safe to call at any time:
/// - Locks briefly to check has_warm + clone on_output, then releases
/// - All blocking PTY work happens outside the Mutex on a background thread
pub fn start_prewarm(manager: Arc<Mutex<TerminalManager>>) {
    let on_output = match manager.lock() {
        Ok(mut mgr) => mgr.begin_prewarm(),
        Err(_) => return,
    };
    let Some(on_output) = on_output else { return };

    std::thread::spawn(move || match spawn_prewarm(on_output) {
        Ok(entry) => {
            if let Ok(mut mgr) = manager.lock() {
                mgr.finish_prewarm(Some(entry));
            }
        }
        Err(e) => {
            if let Ok(mut mgr) = manager.lock() {
                mgr.finish_prewarm(None);
            }
            eprintln!("[keynova] terminal pre-warm failed: {e}");
        }
    });
}

fn terminal_command() -> CommandBuilder {
    #[cfg(target_os = "windows")]
    {
        let shell = std::env::var_os("KEYNOVA_TERMINAL_SHELL")
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .or_else(|| find_in_path("pwsh.exe"))
            .or_else(|| find_in_path("powershell.exe"))
            .or_else(|| std::env::var_os("COMSPEC").map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("powershell.exe"));

        let shell_name = shell
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let mut cmd = CommandBuilder::new(shell);
        configure_terminal_env(&mut cmd);
        if shell_name == "powershell.exe" || shell_name == "pwsh.exe" {
            cmd.arg("-NoLogo");
        }
        cmd
    }

    #[cfg(not(target_os = "windows"))]
    {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let mut cmd = CommandBuilder::new(shell);
        configure_terminal_env(&mut cmd);
        cmd
    }
}

fn configure_terminal_env(cmd: &mut CommandBuilder) {
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("TERM_PROGRAM", "Keynova");

    #[cfg(target_os = "windows")]
    {
        // Let WSL inherit terminal capability hints when launched from the shell.
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

#[cfg(target_os = "windows")]
fn find_in_path(program: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|dir| dir.join(program))
            .find(|candidate| candidate.is_file())
    })
}
