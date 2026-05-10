/// Cross-platform process sandbox — research implementation for ADR-027.
///
/// **Not connected to any Agent tool registry.**
/// Generic shell execution remains blocked until ADR-027 is lifted to product status.
/// This module provides the infrastructure that satisfies the platform-constraint
/// requirements described in ADR-027 so that future lifting only needs a registry wire-up.
use std::path::Path;
use std::time::Duration;
use thiserror::Error;

// ─── Public API ─────────────────────────────────────────────────────────────

/// Constraints applied to the sandboxed subprocess.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Wall-clock timeout. The process is killed after this elapses.
    pub timeout: Duration,
    /// Maximum committed memory for the job (0 = no limit).
    /// Enforced via Job Object on Windows; best-effort on other platforms.
    pub max_memory_bytes: u64,
    /// Deny outbound network connections.
    /// Enforced via namespace isolation on Linux (bwrap); advisory on others.
    pub deny_network: bool,
    /// Stdout/stderr byte cap applied before returning to the caller.
    /// Protects callers from unexpectedly large subprocess output.
    pub output_cap_bytes: usize,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
            max_memory_bytes: 128 * 1024 * 1024,
            deny_network: true,
            output_cap_bytes: 64 * 1024,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct SandboxedOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub kill_reason: Option<KillReason>,
    /// True when stdout or stderr was truncated to `output_cap_bytes`.
    pub output_truncated: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KillReason {
    Timeout,
    MemoryExceeded,
}

#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum SandboxError {
    #[error("sandbox not available: {0}")]
    Unavailable(String),
    #[error("sandbox setup failed: {0}")]
    SetupFailed(String),
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
}

/// Reports the sandbox mechanism available on the current platform.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxAvailability {
    /// Full sandbox available with all requested constraints.
    Available { mechanism: &'static str },
    /// Sandbox binary (bwrap / sandbox-exec) not found in PATH.
    BinaryMissing { required: &'static str },
    /// Platform not yet supported.
    Unsupported,
}

/// Check whether sandboxed execution is available without running anything.
#[allow(dead_code)]
pub fn sandbox_available() -> SandboxAvailability {
    imp::availability()
}

/// Execute `program args` in a sandboxed subprocess and return captured output.
///
/// stdout and stderr are capped at `config.output_cap_bytes` each.
/// Callers should pipe the output through the Agent observation redaction pipeline
/// before exposing it to an LLM context (see `core::agent_observation`).
#[allow(dead_code)]
pub fn execute_sandboxed(
    program: &str,
    args: &[&str],
    cwd: &Path,
    config: &SandboxConfig,
) -> Result<SandboxedOutput, SandboxError> {
    imp::run(program, args, cwd, config)
}

// ─── Shared helpers (non-Windows) ───────────────────────────────────────────

#[cfg(not(target_os = "windows"))]
mod common {
    use super::{KillReason, SandboxConfig, SandboxError, SandboxedOutput};
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::time::Instant;

    pub fn run_with_poll_timeout(
        program: &str,
        args: &[&str],
        cwd: &std::path::Path,
        config: &SandboxConfig,
    ) -> Result<SandboxedOutput, SandboxError> {
        let mut child = Command::new(program)
            .args(args)
            .current_dir(cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(SandboxError::Io)?;

        // Read pipes in threads to avoid deadlock on full pipe buffer.
        let stdout_pipe = child.stdout.take().expect("stdout piped");
        let stderr_pipe = child.stderr.take().expect("stderr piped");
        let stdout_thread = std::thread::spawn(move || -> Vec<u8> { read_bytes(stdout_pipe) });
        let stderr_thread = std::thread::spawn(move || -> Vec<u8> { read_bytes(stderr_pipe) });

        let deadline = Instant::now() + config.timeout;
        let kill_reason = loop {
            match child.try_wait().map_err(SandboxError::Io)? {
                Some(_) => break None,
                None => {
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        break Some(KillReason::Timeout);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
        };

        let status = child.wait().map_err(SandboxError::Io)?;
        let raw_stdout = stdout_thread.join().unwrap_or_default();
        let raw_stderr = stderr_thread.join().unwrap_or_default();

        Ok(build_output(
            raw_stdout,
            raw_stderr,
            status.code().unwrap_or(-1),
            kill_reason,
            config.output_cap_bytes,
        ))
    }

    fn read_bytes(mut reader: impl Read) -> Vec<u8> {
        let mut buf = Vec::new();
        let _ = reader.read_to_end(&mut buf);
        buf
    }

    pub fn build_output(
        raw_stdout: Vec<u8>,
        raw_stderr: Vec<u8>,
        exit_code: i32,
        kill_reason: Option<KillReason>,
        cap: usize,
    ) -> SandboxedOutput {
        let (stdout, so_trunc) = cap_bytes(raw_stdout, cap);
        let (stderr, se_trunc) = cap_bytes(raw_stderr, cap);
        SandboxedOutput {
            stdout,
            stderr,
            exit_code,
            kill_reason,
            output_truncated: so_trunc || se_trunc,
        }
    }

    fn cap_bytes(bytes: Vec<u8>, cap: usize) -> (String, bool) {
        if cap > 0 && bytes.len() > cap {
            (
                String::from_utf8_lossy(&bytes[..cap]).into_owned(),
                true,
            )
        } else {
            (String::from_utf8_lossy(&bytes).into_owned(), false)
        }
    }
}

// ─── Windows ────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
mod imp {
    use super::{KillReason, SandboxAvailability, SandboxConfig, SandboxError, SandboxedOutput};
    use std::io::Read;
    use std::process::{Command, Stdio};
    use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_EVENT, WAIT_OBJECT_0};
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_JOB_MEMORY, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_ALL_ACCESS, PROCESS_SYNCHRONIZE,
    };
    use windows::core::PCWSTR;

    #[allow(dead_code)]
    pub(super) fn availability() -> SandboxAvailability {
        SandboxAvailability::Available {
            mechanism: "Windows Job Object (memory limit + kill-on-close)",
        }
    }

    pub(super) fn run(
        program: &str,
        args: &[&str],
        cwd: &std::path::Path,
        config: &SandboxConfig,
    ) -> Result<SandboxedOutput, SandboxError> {
        let mut child = Command::new(program)
            .args(args)
            .current_dir(cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(SandboxError::Io)?;

        let pid = child.id();

        // Apply Job Object memory + cleanup constraints immediately after spawn.
        let job = unsafe { create_and_assign_job(pid, config) }
            .map_err(SandboxError::SetupFailed)?;

        // Read stdout/stderr in background threads (prevents pipe-buffer deadlock).
        let stdout_pipe = child.stdout.take().expect("stdout piped");
        let stderr_pipe = child.stderr.take().expect("stderr piped");
        let so_cap = config.output_cap_bytes;
        let se_cap = config.output_cap_bytes;
        let stdout_thread =
            std::thread::spawn(move || -> Vec<u8> { read_capped(stdout_pipe, so_cap) });
        let stderr_thread =
            std::thread::spawn(move || -> Vec<u8> { read_capped(stderr_pipe, se_cap) });

        // Wait with Win32 timeout — avoids 50 ms polling.
        let timeout_ms = config
            .timeout
            .as_millis()
            .clamp(1, u32::MAX as u128) as u32;

        let (wait_result, sync_handle) = unsafe { wait_for_process(pid, timeout_ms) }
            .map_err(SandboxError::SetupFailed)?;

        let kill_reason = if wait_result != WAIT_OBJECT_0 {
            let _ = child.kill();
            Some(KillReason::Timeout)
        } else {
            None
        };

        if let Some(h) = sync_handle {
            unsafe { let _ = CloseHandle(h); }
        }
        unsafe { let _ = CloseHandle(job); }

        let status = child.wait().map_err(SandboxError::Io)?;
        let raw_stdout = stdout_thread.join().unwrap_or_default();
        let raw_stderr = stderr_thread.join().unwrap_or_default();

        let (stdout, so_trunc) = lossy_cap(raw_stdout, config.output_cap_bytes);
        let (stderr, se_trunc) = lossy_cap(raw_stderr, config.output_cap_bytes);

        Ok(SandboxedOutput {
            stdout,
            stderr,
            exit_code: status.code().unwrap_or(-1),
            kill_reason,
            output_truncated: so_trunc || se_trunc,
        })
    }

    unsafe fn create_and_assign_job(pid: u32, config: &SandboxConfig) -> Result<HANDLE, String> {
        let job =
            CreateJobObjectW(None, PCWSTR::null()).map_err(|e| format!("CreateJobObject: {e}"))?;

        let mut flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if config.max_memory_bytes > 0 {
            flags |= JOB_OBJECT_LIMIT_JOB_MEMORY;
        }

        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = flags;
        if config.max_memory_bytes > 0 {
            info.JobMemoryLimit = config.max_memory_bytes as usize;
        }

        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &raw const info as *const std::ffi::c_void,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
        .map_err(|e| {
            let _ = CloseHandle(job);
            format!("SetInformationJobObject: {e}")
        })?;

        let proc_handle = OpenProcess(PROCESS_ALL_ACCESS, false, pid)
            .map_err(|e| format!("OpenProcess(assign): {e}"))?;

        let assign_result = AssignProcessToJobObject(job, proc_handle);
        let _ = CloseHandle(proc_handle);

        assign_result.map_err(|e| {
            let _ = CloseHandle(job);
            format!("AssignProcessToJobObject: {e}")
        })?;

        Ok(job)
    }

    /// Returns `(WaitResult, Option<sync_handle>)`. The caller must close the handle.
    unsafe fn wait_for_process(
        pid: u32,
        timeout_ms: u32,
    ) -> Result<(WAIT_EVENT, Option<HANDLE>), String> {
        let handle = OpenProcess(PROCESS_SYNCHRONIZE, false, pid)
            .map_err(|e| format!("OpenProcess(wait): {e}"))?;
        let result = WaitForSingleObject(handle, timeout_ms);
        Ok((result, Some(handle)))
    }

    fn read_capped(mut reader: impl Read, cap: usize) -> Vec<u8> {
        let mut buf = Vec::new();
        let _ = reader.read_to_end(&mut buf);
        if cap > 0 && buf.len() > cap {
            buf.truncate(cap);
        }
        buf
    }

    fn lossy_cap(bytes: Vec<u8>, cap: usize) -> (String, bool) {
        let truncated = cap > 0 && bytes.len() >= cap;
        (String::from_utf8_lossy(&bytes).into_owned(), truncated)
    }
}

// ─── Linux (bwrap) ──────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
mod imp {
    use super::{SandboxAvailability, SandboxConfig, SandboxError, SandboxedOutput};

    const BWRAP_BIN: &str = "bwrap";

    pub(super) fn availability() -> SandboxAvailability {
        if which_bwrap().is_some() {
            SandboxAvailability::Available {
                mechanism: "bubblewrap (bwrap) — network + pid namespace isolation",
            }
        } else {
            SandboxAvailability::BinaryMissing {
                required: BWRAP_BIN,
            }
        }
    }

    pub(super) fn run(
        program: &str,
        args: &[&str],
        cwd: &std::path::Path,
        config: &SandboxConfig,
    ) -> Result<SandboxedOutput, SandboxError> {
        let bwrap = which_bwrap().ok_or_else(|| {
            SandboxError::Unavailable(
                "bwrap not found; install bubblewrap: `apt install bubblewrap`".into(),
            )
        })?;

        // Build bwrap arguments.
        // --unshare-net isolates network; --unshare-pid isolates PID namespace.
        // We bind read-only host paths the program needs to execute.
        let mut bwrap_args: Vec<String> = vec![
            "--unshare-net".into(),
            "--unshare-pid".into(),
            "--unshare-ipc".into(),
            "--proc".into(),
            "/proc".into(),
            "--dev".into(),
            "/dev".into(),
            "--ro-bind".into(),
            "/usr".into(),
            "/usr".into(),
            "--ro-bind-try".into(),
            "/lib".into(),
            "/lib".into(),
            "--ro-bind-try".into(),
            "/lib64".into(),
            "/lib64".into(),
            "--ro-bind-try".into(),
            "/bin".into(),
            "/bin".into(),
            "--tmpfs".into(),
            "/tmp".into(),
            "--chdir".into(),
            cwd.to_string_lossy().into_owned(),
            "--".into(),
            program.into(),
        ];
        for arg in args {
            bwrap_args.push((*arg).into());
        }

        let str_args: Vec<&str> = bwrap_args.iter().map(|s| s.as_str()).collect();
        super::common::run_with_poll_timeout(&bwrap, &str_args, cwd, config)
    }

    fn which_bwrap() -> Option<String> {
        std::env::var("PATH").ok().and_then(|paths| {
            paths.split(':').find_map(|dir| {
                let candidate = std::path::Path::new(dir).join(BWRAP_BIN);
                candidate.is_file().then(|| candidate.to_string_lossy().into_owned())
            })
        })
    }
}

// ─── macOS (sandbox-exec) ───────────────────────────────────────────────────

#[cfg(target_os = "macos")]
mod imp {
    use super::{SandboxAvailability, SandboxConfig, SandboxError, SandboxedOutput};

    const SANDBOX_EXEC: &str = "/usr/bin/sandbox-exec";

    pub(super) fn availability() -> SandboxAvailability {
        if std::path::Path::new(SANDBOX_EXEC).exists() {
            SandboxAvailability::Available {
                mechanism: "sandbox-exec (Apple Sandbox, deny-by-default profile)",
            }
        } else {
            SandboxAvailability::BinaryMissing {
                required: SANDBOX_EXEC,
            }
        }
    }

    pub(super) fn run(
        program: &str,
        args: &[&str],
        cwd: &std::path::Path,
        config: &SandboxConfig,
    ) -> Result<SandboxedOutput, SandboxError> {
        if !std::path::Path::new(SANDBOX_EXEC).exists() {
            return Err(SandboxError::Unavailable(
                "sandbox-exec not found; requires macOS".into(),
            ));
        }

        let profile_path = write_temp_profile(config)
            .map_err(|e| SandboxError::SetupFailed(format!("sandbox profile write: {e}")))?;

        let profile_str = profile_path.to_string_lossy().into_owned();
        let mut exec_args: Vec<&str> = vec!["-f", &profile_str, "--", program];
        exec_args.extend_from_slice(args);

        let result = super::common::run_with_poll_timeout(SANDBOX_EXEC, &exec_args, cwd, config);
        let _ = std::fs::remove_file(&profile_path);
        result
    }

    fn write_temp_profile(config: &SandboxConfig) -> std::io::Result<std::path::PathBuf> {
        let network_rule = if config.deny_network {
            "(deny network*)"
        } else {
            "(allow network*)"
        };
        // Minimal deny-by-default profile: allow process execution + file reads,
        // deny everything else (including network when configured).
        let profile = format!(
            "(version 1)\n(deny default)\n(allow process*)\n(allow file-read*)\n{network_rule}\n"
        );
        let path = std::env::temp_dir().join(format!(
            "keynova-sandbox-{}.sb",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        ));
        std::fs::write(&path, profile)?;
        Ok(path)
    }
}

// ─── Unsupported platforms ───────────────────────────────────────────────────

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
mod imp {
    use super::{SandboxAvailability, SandboxConfig, SandboxError, SandboxedOutput};
    use std::path::Path;

    pub(super) fn availability() -> SandboxAvailability {
        SandboxAvailability::Unsupported
    }

    pub(super) fn run(
        _program: &str,
        _args: &[&str],
        _cwd: &Path,
        _config: &SandboxConfig,
    ) -> Result<SandboxedOutput, SandboxError> {
        Err(SandboxError::Unavailable(
            "sandbox not implemented for this platform".into(),
        ))
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_available_returns_some_status() {
        // Must not panic; availability is platform-determined.
        let status = sandbox_available();
        match status {
            SandboxAvailability::Available { mechanism } => {
                assert!(!mechanism.is_empty());
            }
            SandboxAvailability::BinaryMissing { required } => {
                assert!(!required.is_empty());
            }
            SandboxAvailability::Unsupported => {}
        }
    }

    #[test]
    fn default_config_is_sane() {
        let cfg = SandboxConfig::default();
        assert!(cfg.timeout.as_secs() > 0);
        assert!(cfg.max_memory_bytes > 0);
        assert!(cfg.deny_network);
        assert!(cfg.output_cap_bytes > 0);
    }

    #[cfg(target_os = "windows")]
    mod windows_tests {
        use super::*;
        use windows::Win32::Foundation::{CloseHandle, HANDLE};
        use windows::Win32::System::JobObjects::CreateJobObjectW;
        use windows::core::PCWSTR;

        #[test]
        fn create_job_object_succeeds() {
            // Verify the Windows API is reachable and creates a valid handle.
            let job: HANDLE = unsafe {
                CreateJobObjectW(None, PCWSTR::null())
                    .expect("CreateJobObjectW should succeed in tests")
            };
            assert!(!job.is_invalid());
            unsafe { let _ = CloseHandle(job); }
        }

        #[test]
        fn echo_runs_sandboxed() {
            let cfg = SandboxConfig {
                timeout: Duration::from_secs(5),
                ..Default::default()
            };
            let cwd = std::env::temp_dir();
            let result = execute_sandboxed("cmd.exe", &["/C", "echo hello"], &cwd, &cfg);
            let out = result.expect("sandboxed echo should succeed");
            assert_eq!(out.exit_code, 0);
            assert!(out.stdout.contains("hello"), "stdout: {:?}", out.stdout);
            assert!(out.kill_reason.is_none());
        }

        #[test]
        fn timeout_kills_long_running_process() {
            let cfg = SandboxConfig {
                timeout: Duration::from_millis(300),
                ..Default::default()
            };
            let cwd = std::env::temp_dir();
            let result = execute_sandboxed("cmd.exe", &["/C", "ping 127.0.0.1 -n 10"], &cwd, &cfg);
            let out = result.expect("should return output even on timeout");
            assert_eq!(out.kill_reason, Some(KillReason::Timeout));
        }
    }

    #[cfg(target_os = "linux")]
    mod linux_tests {
        use super::*;

        #[test]
        fn run_echo_via_bwrap_or_skip() {
            if sandbox_available() == (SandboxAvailability::BinaryMissing { required: "bwrap" }) {
                eprintln!("bwrap not found — skipping sandbox execution test");
                return;
            }
            let cfg = SandboxConfig::default();
            let cwd = std::env::temp_dir();
            let out = execute_sandboxed("echo", &["hello"], &cwd, &cfg)
                .expect("bwrap echo should succeed");
            assert!(out.stdout.contains("hello"));
        }
    }
}