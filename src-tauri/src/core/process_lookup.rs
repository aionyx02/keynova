//! UTIL.2.J — find the TCP-LISTEN process bound to a given port, then kill it
//! after user confirmation.
//!
//! Cross-platform via native CLI tools (`netstat`/`tasklist` on Windows,
//! `lsof` on Unix). Output parsing lives in pure functions so it can be
//! unit-tested without spawning subprocesses; the dispatch shell-out is a thin
//! orchestration layer that integration tests cover with `#[ignore]`.
//!
//! `kill_pid` is a low-level primitive. Callers must gate it behind a backend
//! approval token; text arguments are not a security boundary because `cmd.run`
//! is itself IPC-reachable.

use std::process::Command;

#[cfg(target_os = "windows")]
use crate::core::SilentCommandExt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessInfo {
    pub pid: u32,
    pub process_name: String,
    pub port: u16,
    pub protocol: String,
}

// ─── Public cross-platform API ───────────────────────────────────────────────

pub fn find_process_by_port(port: u16) -> Result<Option<ProcessInfo>, String> {
    if port == 0 {
        return Err("port must be > 0".into());
    }
    #[cfg(target_os = "windows")]
    {
        find_process_windows(port)
    }
    #[cfg(not(target_os = "windows"))]
    {
        find_process_unix(port)
    }
}

pub fn kill_pid(pid: u32) -> Result<(), String> {
    if pid == 0 {
        return Err("refusing to kill pid 0".into());
    }
    #[cfg(target_os = "windows")]
    {
        let out = Command::new("taskkill")
            .no_window()
            .args(["/F", "/PID", &pid.to_string()])
            .output()
            .map_err(|e| format!("taskkill spawn failed: {e}"))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(format!("taskkill failed: {stderr}"));
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let out = Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output()
            .map_err(|e| format!("kill spawn failed: {e}"))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(format!("kill failed: {stderr}"));
        }
        Ok(())
    }
}

// ─── Windows ─────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn find_process_windows(port: u16) -> Result<Option<ProcessInfo>, String> {
    let out = Command::new("netstat")
        .no_window()
        .args(["-ano", "-p", "TCP"])
        .output()
        .map_err(|e| format!("netstat spawn failed: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "netstat failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let Some(pid) = parse_netstat_tcp_listen(&text, port) else {
        return Ok(None);
    };
    let name = process_name_from_tasklist(pid)?.unwrap_or_else(|| "unknown".into());
    Ok(Some(ProcessInfo {
        pid,
        process_name: name,
        port,
        protocol: "TCP".into(),
    }))
}

#[cfg(target_os = "windows")]
fn process_name_from_tasklist(pid: u32) -> Result<Option<String>, String> {
    let out = Command::new("tasklist")
        .no_window()
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
        .map_err(|e| format!("tasklist spawn failed: {e}"))?;
    if !out.status.success() {
        return Ok(None);
    }
    let text = String::from_utf8_lossy(&out.stdout);
    Ok(parse_tasklist_csv(&text))
}

/// Parse `netstat -ano -p TCP` output and return the PID listening on `port`,
/// if any. Skips non-TCP lines and the header. Local address column may be
/// `0.0.0.0:80`, `127.0.0.1:5432`, `[::]:80`, etc.
pub fn parse_netstat_tcp_listen(out: &str, port: u16) -> Option<u32> {
    let needle = format!(":{port}");
    for line in out.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        // Expect: Proto Local Foreign State PID
        if parts.len() < 5 || !parts[0].eq_ignore_ascii_case("TCP") {
            continue;
        }
        if parts[3] != "LISTENING" {
            continue;
        }
        let local = parts[1];
        // Strip [::] bracket form: match literal ":<port>" at end.
        if let Some(stripped) = local.rsplit_once(':') {
            if format!(":{}", stripped.1) == needle {
                if let Ok(pid) = parts[4].parse::<u32>() {
                    return Some(pid);
                }
            }
        }
    }
    None
}

/// Parse `tasklist /FO CSV /NH` first row's process name (first CSV field).
pub fn parse_tasklist_csv(out: &str) -> Option<String> {
    let line = out.lines().next()?.trim();
    if line.is_empty() {
        return None;
    }
    // CSV row format: "name.exe","12345","Console","1","123,456 K"
    let inner = line.strip_prefix('"')?;
    let end = inner.find('"')?;
    Some(inner[..end].to_string())
}

// ─── Unix (Linux / macOS) ────────────────────────────────────────────────────

#[cfg(not(target_os = "windows"))]
fn find_process_unix(port: u16) -> Result<Option<ProcessInfo>, String> {
    let port_filter = format!("-iTCP:{port}");
    let out = Command::new("lsof")
        .args(["-nP", &port_filter, "-sTCP:LISTEN"])
        .output()
        .map_err(|e| format!("lsof spawn failed: {e}"))?;
    if !out.status.success() {
        // lsof returns 1 when nothing matches — surface as None, not error.
        let code = out.status.code().unwrap_or(-1);
        if code == 1 && out.stdout.is_empty() {
            return Ok(None);
        }
        return Err(format!(
            "lsof failed (exit {code}): {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let Some((name, pid)) = parse_lsof_listen(&text) else {
        return Ok(None);
    };
    Ok(Some(ProcessInfo {
        pid,
        process_name: name,
        port,
        protocol: "TCP".into(),
    }))
}

/// Parse `lsof -nP -iTCP:PORT -sTCP:LISTEN` output. Skip header, take first data row.
pub fn parse_lsof_listen(out: &str) -> Option<(String, u32)> {
    for line in out.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let name = parts[0].to_string();
        let pid: u32 = parts[1].parse().ok()?;
        return Some((name, pid));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── netstat parser ───────────────────────────────────────────────────────

    #[test]
    fn parse_netstat_finds_listening_pid() {
        let out = "\
Active Connections

  Proto  Local Address          Foreign Address        State           PID
  TCP    0.0.0.0:80             0.0.0.0:0              LISTENING       4
  TCP    127.0.0.1:5432         0.0.0.0:0              LISTENING       12345
  TCP    0.0.0.0:445            0.0.0.0:0              LISTENING       4
  TCP    127.0.0.1:5432         127.0.0.1:54321        ESTABLISHED     12345
";
        assert_eq!(parse_netstat_tcp_listen(out, 5432), Some(12345));
        assert_eq!(parse_netstat_tcp_listen(out, 80), Some(4));
    }

    #[test]
    fn parse_netstat_ignores_non_listening() {
        let out = "  TCP    0.0.0.0:8080  127.0.0.1:1234  ESTABLISHED  99\n";
        assert_eq!(parse_netstat_tcp_listen(out, 8080), None);
    }

    #[test]
    fn parse_netstat_ignores_udp() {
        let out = "  UDP    0.0.0.0:53     *:*                                    1234\n";
        assert_eq!(parse_netstat_tcp_listen(out, 53), None);
    }

    #[test]
    fn parse_netstat_handles_ipv6_bracket_form() {
        let out = "  TCP    [::]:80      [::]:0    LISTENING    7\n";
        assert_eq!(parse_netstat_tcp_listen(out, 80), Some(7));
    }

    #[test]
    fn parse_netstat_missing_port_returns_none() {
        let out = "  TCP    0.0.0.0:80   0.0.0.0:0   LISTENING   4\n";
        assert_eq!(parse_netstat_tcp_listen(out, 9999), None);
    }

    // ── tasklist parser ─────────────────────────────────────────────────────

    #[test]
    fn parse_tasklist_csv_extracts_process_name() {
        let out = "\"chrome.exe\",\"12345\",\"Console\",\"1\",\"123,456 K\"\n";
        assert_eq!(parse_tasklist_csv(out), Some("chrome.exe".into()));
    }

    #[test]
    fn parse_tasklist_csv_empty_returns_none() {
        assert_eq!(parse_tasklist_csv(""), None);
        assert_eq!(parse_tasklist_csv("\n"), None);
    }

    #[test]
    fn parse_tasklist_csv_malformed_returns_none() {
        assert_eq!(parse_tasklist_csv("no quotes here"), None);
    }

    // ── lsof parser ─────────────────────────────────────────────────────────

    #[test]
    fn parse_lsof_finds_first_data_row() {
        let out = "\
COMMAND     PID USER   FD   TYPE  DEVICE SIZE/OFF NODE NAME
postgres  12345 user   6u  IPv4 1234567      0t0  TCP *:5432 (LISTEN)
postgres  12346 user   6u  IPv4 1234568      0t0  TCP *:5432 (LISTEN)
";
        assert_eq!(parse_lsof_listen(out), Some(("postgres".into(), 12345)));
    }

    #[test]
    fn parse_lsof_empty_returns_none() {
        assert_eq!(parse_lsof_listen(""), None);
        assert_eq!(parse_lsof_listen("COMMAND  PID  USER\n"), None);
    }

    // ── kill_pid guards ──────────────────────────────────────────────────────

    #[test]
    fn kill_pid_zero_refused() {
        assert!(kill_pid(0).is_err());
    }

    #[test]
    fn find_process_zero_port_errors() {
        assert!(find_process_by_port(0).is_err());
    }
}
