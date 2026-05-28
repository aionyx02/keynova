//! Bounded, read-only dev command runner.
//!
//! Used by the agent ReAct tool dispatch path (`dev.cargo_test`,
//! `dev.cargo_check`, `dev.npm_build`, `dev.npm_lint`) and reserved for the
//! upcoming `fix_error` capability (REF.4). Lives under `core/` so the
//! ai_capability layer can call it without depending on a handler module.
//!
//! Guarantees:
//! - The command + args slice is caller-controlled; no shell, no user-quoted
//!   segments — caller must supply a fixed deterministic invocation.
//! - cwd is caller-controlled; callers must enforce workspace scope before
//!   handing the path in.
//! - stdout/stderr are read in background threads to avoid pipe-buffer
//!   deadlock and truncated at `DEV_CMD_OUTPUT_LIMIT` bytes.
//! - Process is killed when `timeout` elapses.

use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

pub const DEV_CMD_OUTPUT_LIMIT: usize = 64 * 1024;
pub const DEV_CARGO_TIMEOUT_SECS: u64 = 120;
pub const DEV_NPM_TIMEOUT_SECS: u64 = 60;

/// Run a read-only, workspace-scoped dev command with timeout and bounded output.
/// `program` and `args` must be a fixed, deterministic command — no
/// user-controlled segments.
pub fn run_bounded_dev_cmd(
    program: &str,
    args: &[&str],
    cwd: &Path,
    timeout: Duration,
) -> Result<Value, String> {
    use std::io::Read;

    let mut child = std::process::Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("{program}: failed to spawn: {e}"))?;

    let stdout_pipe = child.stdout.take();
    let stderr_pipe = child.stderr.take();

    let (out_tx, out_rx) = mpsc::channel::<Vec<u8>>();
    let (err_tx, err_rx) = mpsc::channel::<Vec<u8>>();

    if let Some(mut pipe) = stdout_pipe {
        thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = pipe.read_to_end(&mut buf);
            let _ = out_tx.send(buf);
        });
    }
    if let Some(mut pipe) = stderr_pipe {
        thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = pipe.read_to_end(&mut buf);
            let _ = err_tx.send(buf);
        });
    }

    let deadline = Instant::now() + timeout;
    let exit_code = loop {
        match child
            .try_wait()
            .map_err(|e| format!("{program}: wait error: {e}"))?
        {
            Some(status) => break status.code(),
            None => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    return Err(format!(
                        "{program}: timed out after {}s — process killed",
                        timeout.as_secs()
                    ));
                }
                thread::sleep(Duration::from_millis(50));
            }
        }
    };

    let stdout_bytes = out_rx.recv().unwrap_or_default();
    let stderr_bytes = err_rx.recv().unwrap_or_default();

    Ok(json!({
        "stdout": bound_output_n(&stdout_bytes, DEV_CMD_OUTPUT_LIMIT),
        "stderr": bound_output_n(&stderr_bytes, DEV_CMD_OUTPUT_LIMIT),
        "exit_code": exit_code,
        "preview": format!("{program} {}", args.join(" ")),
    }))
}

/// Parse raw compiler/lint output and return a list of structured error objects.
///
/// Handles two common formats:
/// - Rust/cargo: lines starting with `error` or `error[Exxxx]`, followed by location `-->`.
/// - NPM/ESLint: lines starting with `ERROR` or matching `✖ N problems`.
pub fn extract_compiler_errors(output: &str) -> Vec<Value> {
    let mut errors: Vec<Value> = Vec::new();
    let mut pending_message: Option<String> = None;
    let mut pending_code: Option<String> = None;

    for line in output.lines() {
        let trimmed = line.trim();

        // Cargo: `error[E0308]: mismatched types` or `error: ...`
        if trimmed.starts_with("error[")
            || (trimmed.starts_with("error")
                && trimmed.contains(':')
                && !trimmed.starts_with("error -->"))
        {
            if let Some(msg) = pending_message.take() {
                errors
                    .push(json!({ "message": msg, "code": pending_code.take(), "location": null }));
            }
            // Extract optional error code like E0308
            let code = if trimmed.starts_with("error[") {
                trimmed.find(']').map(|end| trimmed[6..end].to_string())
            } else {
                None
            };
            let message = trimmed
                .split_once(':')
                .map(|(_, rest)| rest.trim().to_string())
                .unwrap_or_else(|| trimmed.to_string());
            pending_code = code;
            pending_message = Some(message);
        }
        // Cargo: `  --> src/foo.rs:10:5`
        else if trimmed.starts_with("-->") && pending_message.is_some() {
            let location = trimmed.trim_start_matches("-->").trim().to_string();
            errors.push(json!({
                "message": pending_message.take().unwrap_or_default(),
                "code": pending_code.take(),
                "location": location,
            }));
        }
        // NPM/ESLint: `  10:5  error  'foo' is not defined  no-undef`
        else {
            let parts: Vec<&str> = trimmed.splitn(4, "  ").collect();
            if parts.len() >= 3 && parts[1] == "error" && parts[0].contains(':') {
                errors.push(json!({
                    "message": parts.get(2).unwrap_or(&"").trim(),
                    "code": parts.get(3).map(|s| s.trim()),
                    "location": parts[0].trim(),
                }));
            }
        }
    }

    // Flush any trailing pending message without a location line.
    if let Some(msg) = pending_message.take() {
        errors.push(json!({ "message": msg, "code": pending_code.take(), "location": null }));
    }

    // Cap at 20 errors to keep response size bounded.
    errors.truncate(20);
    errors
}

/// Truncate command-output bytes at `limit`, appending a `[output truncated: ...]`
/// marker when the original exceeded the limit.
pub fn bound_output_n(bytes: &[u8], limit: usize) -> String {
    let s = String::from_utf8_lossy(bytes);
    if s.len() > limit {
        format!(
            "{}\n[output truncated: {} bytes total, limit {}B]",
            &s[..limit],
            bytes.len(),
            limit
        )
    } else {
        s.into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn bound_output_n_truncates_at_given_limit() {
        let big = vec![b'a'; DEV_CMD_OUTPUT_LIMIT + 50];
        let out = bound_output_n(&big, DEV_CMD_OUTPUT_LIMIT);
        assert!(out.contains("[output truncated:"));
        assert!(out.starts_with(&"a".repeat(DEV_CMD_OUTPUT_LIMIT)));
    }

    #[test]
    fn run_bounded_dev_cmd_captures_echo_output() {
        let dir = std::env::temp_dir().join(format!("keynova-dev-cmd-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create temp dir");

        #[cfg(target_os = "windows")]
        let (prog, args): (&str, &[&str]) = ("cmd", &["/C", "echo hello"]);
        #[cfg(not(target_os = "windows"))]
        let (prog, args): (&str, &[&str]) = ("sh", &["-c", "echo hello"]);

        let result = run_bounded_dev_cmd(prog, args, &dir, Duration::from_secs(5));
        assert!(result.is_ok(), "command should succeed: {:?}", result);
        let val = result.unwrap();
        let stdout = val["stdout"].as_str().unwrap_or("");
        assert!(
            stdout.contains("hello"),
            "stdout should contain 'hello', got: {stdout}"
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn run_bounded_dev_cmd_times_out_slow_process() {
        let dir = std::env::temp_dir().join(format!("keynova-dev-timeout-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create temp dir");

        // On Windows, `timeout` exits immediately when stdout is piped (non-interactive).
        // `ping -n 30 127.0.0.1` works in any mode and takes ~30 s.
        #[cfg(target_os = "windows")]
        let (prog, args): (&str, &[&str]) = ("ping", &["-n", "30", "127.0.0.1"]);
        #[cfg(not(target_os = "windows"))]
        let (prog, args): (&str, &[&str]) = ("sh", &["-c", "sleep 30"]);

        let result = run_bounded_dev_cmd(prog, args, &dir, Duration::from_millis(300));
        assert!(result.is_err(), "slow process should time out");
        assert!(
            result.unwrap_err().contains("timed out"),
            "error should mention timeout"
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn extract_compiler_errors_parses_cargo_error_with_location() {
        let output = "error[E0308]: mismatched types\n  --> src/main.rs:10:5\n  |\n10 |     let x: i32 = \"hello\";\n";
        let errors = extract_compiler_errors(output);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0]["code"].as_str().unwrap(), "E0308");
        assert!(errors[0]["message"]
            .as_str()
            .unwrap()
            .contains("mismatched types"));
        assert!(errors[0]["location"]
            .as_str()
            .unwrap()
            .contains("src/main.rs:10:5"));
    }

    #[test]
    fn extract_compiler_errors_parses_multiple_cargo_errors() {
        let output = "error[E0308]: mismatched types\n  --> src/a.rs:1:1\nerror[E0425]: unresolved name\n  --> src/b.rs:5:3\n";
        let errors = extract_compiler_errors(output);
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0]["code"].as_str().unwrap(), "E0308");
        assert_eq!(errors[1]["code"].as_str().unwrap(), "E0425");
    }

    #[test]
    fn extract_compiler_errors_returns_empty_for_clean_output() {
        let output = "   Compiling foo v0.1.0\n    Finished dev [unoptimized] target(s) in 0.45s\n";
        let errors = extract_compiler_errors(output);
        assert!(errors.is_empty());
    }

    #[test]
    fn extract_compiler_errors_caps_at_twenty() {
        let mut output = String::new();
        for i in 0..30 {
            output.push_str(&format!("error: thing {i} failed\n  --> src/x.rs:{i}:1\n"));
        }
        let errors = extract_compiler_errors(&output);
        assert_eq!(errors.len(), 20, "should be capped at 20");
    }
}
