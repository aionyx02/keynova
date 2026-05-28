//! UTIL.2 dev utility BuiltinCommand wrappers. Each command parses its
//! arguments, calls into `core::dev_utils`, and returns the result inline.
//!
//! Commands intentionally skip the manager actor pattern — they are stateless
//! pure functions and the BuiltinCommand `execute` signature is already sync.

use std::str::FromStr;

use crate::core::builtin_command_registry::BuiltinCommand;
use crate::core::dev_utils;
use crate::core::dev_utils::PasswordMode;
use crate::core::process_lookup;
use crate::models::builtin_command::{BuiltinCommandResult, CommandUiType};

fn inline(text: String) -> BuiltinCommandResult {
    BuiltinCommandResult {
        text,
        ui_type: CommandUiType::Inline,
    }
}

fn split_two(args: &str) -> Option<(&str, &str)> {
    let trimmed = args.trim();
    let idx = trimmed.find(char::is_whitespace)?;
    let (a, b) = trimmed.split_at(idx);
    Some((a, b.trim()))
}

// ── uuid ────────────────────────────────────────────────────────────────────

pub struct UuidCmd;
impl BuiltinCommand for UuidCmd {
    fn name(&self) -> &'static str {
        "uuid"
    }
    fn description(&self) -> &'static str {
        "Generate a UUID v4"
    }
    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        inline(dev_utils::uuid_v4())
    }
}

pub struct NanoidCmd;
impl BuiltinCommand for NanoidCmd {
    fn name(&self) -> &'static str {
        "nanoid"
    }
    fn description(&self) -> &'static str {
        "Generate a nanoid (URL-safe random ID)"
    }
    fn args_hint(&self) -> Option<&'static str> {
        Some("[length]  default 21")
    }
    fn execute(&self, args: &str) -> BuiltinCommandResult {
        let len: usize = args.trim().parse().unwrap_or(21);
        inline(dev_utils::nanoid(len))
    }
}

// ── password ───────────────────────────────────────────────────────────────

pub struct PwCmd;
impl BuiltinCommand for PwCmd {
    fn name(&self) -> &'static str {
        "pw"
    }
    fn description(&self) -> &'static str {
        "Generate a random password"
    }
    fn args_hint(&self) -> Option<&'static str> {
        Some("<length> [sym|alnum|alpha]")
    }
    fn execute(&self, args: &str) -> BuiltinCommandResult {
        let trimmed = args.trim();
        if trimmed.is_empty() {
            return inline(dev_utils::generate_password(16, PasswordMode::All));
        }
        let mut parts = trimmed.split_whitespace();
        let length: usize = match parts.next().and_then(|s| s.parse().ok()) {
            Some(n) => n,
            None => return inline("error: <length> must be a positive integer".into()),
        };
        let mode = match parts.next() {
            Some(s) => match PasswordMode::from_str(s) {
                Ok(m) => m,
                Err(e) => return inline(format!("error: {e}")),
            },
            None => PasswordMode::All,
        };
        inline(dev_utils::generate_password(length, mode))
    }
}

// ── hash ────────────────────────────────────────────────────────────────────

pub struct HashCmd;
impl BuiltinCommand for HashCmd {
    fn name(&self) -> &'static str {
        "hash"
    }
    fn description(&self) -> &'static str {
        "Hash text (md5/sha1/sha256/sha512)"
    }
    fn args_hint(&self) -> Option<&'static str> {
        Some("<algo> <text>")
    }
    fn execute(&self, args: &str) -> BuiltinCommandResult {
        let Some((algo, text)) = split_two(args) else {
            return inline("usage: hash <md5|sha1|sha256|sha512> <text>".into());
        };
        match dev_utils::hash_text(algo, text) {
            Ok(hex) => inline(hex),
            Err(e) => inline(format!("error: {e}")),
        }
    }
}

// ── base64 / urlenc ─────────────────────────────────────────────────────────

pub struct B64encCmd;
impl BuiltinCommand for B64encCmd {
    fn name(&self) -> &'static str {
        "b64enc"
    }
    fn description(&self) -> &'static str {
        "Base64 encode UTF-8 text"
    }
    fn args_hint(&self) -> Option<&'static str> {
        Some("<text>")
    }
    fn execute(&self, args: &str) -> BuiltinCommandResult {
        if args.trim().is_empty() {
            return inline("usage: b64enc <text>".into());
        }
        inline(dev_utils::b64_encode(args))
    }
}

pub struct B64decCmd;
impl BuiltinCommand for B64decCmd {
    fn name(&self) -> &'static str {
        "b64dec"
    }
    fn description(&self) -> &'static str {
        "Base64 decode to UTF-8 text"
    }
    fn args_hint(&self) -> Option<&'static str> {
        Some("<base64>")
    }
    fn execute(&self, args: &str) -> BuiltinCommandResult {
        if args.trim().is_empty() {
            return inline("usage: b64dec <base64>".into());
        }
        match dev_utils::b64_decode(args) {
            Ok(text) => inline(text),
            Err(e) => inline(format!("error: {e}")),
        }
    }
}

pub struct UrlencCmd;
impl BuiltinCommand for UrlencCmd {
    fn name(&self) -> &'static str {
        "urlenc"
    }
    fn description(&self) -> &'static str {
        "URL percent-encode"
    }
    fn args_hint(&self) -> Option<&'static str> {
        Some("<text>")
    }
    fn execute(&self, args: &str) -> BuiltinCommandResult {
        if args.trim().is_empty() {
            return inline("usage: urlenc <text>".into());
        }
        inline(dev_utils::url_encode(args))
    }
}

pub struct UrldecCmd;
impl BuiltinCommand for UrldecCmd {
    fn name(&self) -> &'static str {
        "urldec"
    }
    fn description(&self) -> &'static str {
        "URL percent-decode"
    }
    fn args_hint(&self) -> Option<&'static str> {
        Some("<text>")
    }
    fn execute(&self, args: &str) -> BuiltinCommandResult {
        if args.trim().is_empty() {
            return inline("usage: urldec <text>".into());
        }
        match dev_utils::url_decode(args) {
            Ok(text) => inline(text),
            Err(e) => inline(format!("error: {e}")),
        }
    }
}

// ── json ────────────────────────────────────────────────────────────────────

pub struct JsonCmd;
impl BuiltinCommand for JsonCmd {
    fn name(&self) -> &'static str {
        "json"
    }
    fn description(&self) -> &'static str {
        "Pretty-print JSON"
    }
    fn args_hint(&self) -> Option<&'static str> {
        Some("<json>")
    }
    fn execute(&self, args: &str) -> BuiltinCommandResult {
        if args.trim().is_empty() {
            return inline("usage: json <json>".into());
        }
        match dev_utils::json_pretty(args) {
            Ok(text) => inline(text),
            Err(e) => inline(format!("error: {e}")),
        }
    }
}

pub struct JsonmCmd;
impl BuiltinCommand for JsonmCmd {
    fn name(&self) -> &'static str {
        "jsonm"
    }
    fn description(&self) -> &'static str {
        "Minify JSON"
    }
    fn args_hint(&self) -> Option<&'static str> {
        Some("<json>")
    }
    fn execute(&self, args: &str) -> BuiltinCommandResult {
        if args.trim().is_empty() {
            return inline("usage: jsonm <json>".into());
        }
        match dev_utils::json_minify(args) {
            Ok(text) => inline(text),
            Err(e) => inline(format!("error: {e}")),
        }
    }
}

// ── regex ───────────────────────────────────────────────────────────────────

pub struct RegexCmd;
impl BuiltinCommand for RegexCmd {
    fn name(&self) -> &'static str {
        "regex"
    }
    fn description(&self) -> &'static str {
        "Test a regex pattern against text"
    }
    fn args_hint(&self) -> Option<&'static str> {
        Some("<pattern> <text>")
    }
    fn execute(&self, args: &str) -> BuiltinCommandResult {
        let Some((pattern, text)) = split_two(args) else {
            return inline("usage: regex <pattern> <text>".into());
        };
        match dev_utils::regex_test(pattern, text) {
            Ok(report) => inline(report),
            Err(e) => inline(format!("error: {e}")),
        }
    }
}

// ── jwt ─────────────────────────────────────────────────────────────────────

pub struct JwtCmd;
impl BuiltinCommand for JwtCmd {
    fn name(&self) -> &'static str {
        "jwt"
    }
    fn description(&self) -> &'static str {
        "Decode a JWT (no signature verification)"
    }
    fn args_hint(&self) -> Option<&'static str> {
        Some("<token>")
    }
    fn execute(&self, args: &str) -> BuiltinCommandResult {
        if args.trim().is_empty() {
            return inline("usage: jwt <token>".into());
        }
        match dev_utils::jwt_decode(args) {
            Ok(text) => inline(text),
            Err(e) => inline(format!("error: {e}")),
        }
    }
}

// ── color ───────────────────────────────────────────────────────────────────

pub struct ColorCmd;
impl BuiltinCommand for ColorCmd {
    fn name(&self) -> &'static str {
        "color"
    }
    fn description(&self) -> &'static str {
        "Convert color between hex/rgb/hsl"
    }
    fn args_hint(&self) -> Option<&'static str> {
        Some("<#hex | rgb(r,g,b) | hsl(h,s%,l%)>")
    }
    fn execute(&self, args: &str) -> BuiltinCommandResult {
        if args.trim().is_empty() {
            return inline("usage: color <#hex | rgb(r,g,b) | hsl(h,s%,l%)>".into());
        }
        match dev_utils::color_convert(args) {
            Ok(text) => inline(text),
            Err(e) => inline(format!("error: {e}")),
        }
    }
}

// ── killport (UTIL.2.J — destructive, two-phase confirm) ───────────────────

pub struct KillPortCmd;
impl BuiltinCommand for KillPortCmd {
    fn name(&self) -> &'static str {
        "killport"
    }
    fn description(&self) -> &'static str {
        "Find and (after confirm) kill the process listening on a TCP port"
    }
    fn args_hint(&self) -> Option<&'static str> {
        Some("<port>  ·  <port> kill")
    }
    /// Two-phase invocation:
    /// - `killport <port>` → look up and preview process info; do nothing else.
    /// - `killport <port> kill` → look up again and actually kill.
    ///
    /// The second lookup is intentional — between preview and confirm the
    /// owning process may have changed.
    fn execute(&self, args: &str) -> BuiltinCommandResult {
        let trimmed = args.trim();
        if trimmed.is_empty() {
            return inline("usage: killport <port>  ·  killport <port> kill".into());
        }
        let mut parts = trimmed.split_whitespace();
        let port_token = parts.next().unwrap_or("");
        let confirm = parts
            .next()
            .map(|s| s.eq_ignore_ascii_case("kill"))
            .unwrap_or(false);

        let port: u16 = match port_token.parse() {
            Ok(p) if p > 0 => p,
            _ => return inline(format!("error: invalid port '{port_token}'")),
        };

        let info = match process_lookup::find_process_by_port(port) {
            Ok(Some(info)) => info,
            Ok(None) => return inline(format!("no process listening on port {port}")),
            Err(e) => return inline(format!("error: {e}")),
        };

        if !confirm {
            return inline(format!(
                "Preview · port {} ({})\n  pid          {}\n  process_name {}\n\nTo kill, run: killport {} kill",
                info.port, info.protocol, info.pid, info.process_name, info.port,
            ));
        }

        match process_lookup::kill_pid(info.pid) {
            Ok(()) => inline(format!(
                "killed pid {} ({}) listening on port {} ({})",
                info.pid, info.process_name, info.port, info.protocol,
            )),
            Err(e) => inline(format!("error: kill failed: {e}")),
        }
    }
}

// ── cron ────────────────────────────────────────────────────────────────────

pub struct CronCmd;
impl BuiltinCommand for CronCmd {
    fn name(&self) -> &'static str {
        "cron"
    }
    fn description(&self) -> &'static str {
        "Explain a cron expression and show next 5 fire times"
    }
    fn args_hint(&self) -> Option<&'static str> {
        Some("<expr>")
    }
    fn execute(&self, args: &str) -> BuiltinCommandResult {
        if args.trim().is_empty() {
            return inline("usage: cron <expr>".into());
        }
        match dev_utils::cron_explain(args) {
            Ok(text) => inline(text),
            Err(e) => inline(format!("error: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::builtin_command::CommandUiType;

    fn execute_inline(cmd: &dyn BuiltinCommand, args: &str) -> String {
        let r = cmd.execute(args);
        assert!(matches!(r.ui_type, CommandUiType::Inline));
        r.text
    }

    #[test]
    fn uuid_cmd_produces_v4_shape() {
        let s = execute_inline(&UuidCmd, "");
        assert_eq!(s.len(), 36);
    }

    #[test]
    fn hash_cmd_returns_known_vector() {
        assert_eq!(
            execute_inline(&HashCmd, "sha256 abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hash_cmd_missing_args_shows_usage() {
        assert!(execute_inline(&HashCmd, "").starts_with("usage:"));
        assert!(execute_inline(&HashCmd, "sha256").starts_with("usage:"));
    }

    #[test]
    fn b64_cmd_roundtrip() {
        let enc = execute_inline(&B64encCmd, "hello");
        assert_eq!(enc, "aGVsbG8=");
        assert_eq!(execute_inline(&B64decCmd, &enc), "hello");
    }

    #[test]
    fn json_cmd_pretty() {
        let out = execute_inline(&JsonCmd, r#"{"a":1}"#);
        assert!(out.contains('\n'));
    }

    #[test]
    fn regex_cmd_reports_matches() {
        let out = execute_inline(&RegexCmd, r"\d+ abc 1 def 22");
        assert!(out.contains("1 match") || out.contains("2 match"));
    }

    #[test]
    fn pw_cmd_respects_length() {
        let pw = execute_inline(&PwCmd, "12 alpha");
        assert_eq!(pw.len(), 12);
        assert!(pw.chars().all(|c| c.is_ascii_alphabetic()));
    }

    #[test]
    fn color_cmd_converts_hex() {
        let out = execute_inline(&ColorCmd, "#ff0000");
        assert!(out.contains("rgb(255, 0, 0)"));
    }

    #[test]
    fn cron_cmd_handles_5_field() {
        let out = execute_inline(&CronCmd, "*/15 * * * *");
        assert!(out.contains("schedule"));
    }

    // ── killport ────────────────────────────────────────────────────────────

    #[test]
    fn killport_no_args_shows_usage() {
        assert!(execute_inline(&KillPortCmd, "").starts_with("usage:"));
    }

    #[test]
    fn killport_invalid_port_shows_error() {
        assert!(execute_inline(&KillPortCmd, "abc").starts_with("error: invalid port"));
        assert!(execute_inline(&KillPortCmd, "0").starts_with("error: invalid port"));
    }

    /// Pick a port unlikely to be bound (>= 60000) and assert the command
    /// reports "no process listening". This is the only end-to-end path we
    /// can exercise without binding a real socket or risking false positives.
    #[test]
    fn killport_unbound_port_reports_none() {
        let out = execute_inline(&KillPortCmd, "62345");
        assert!(
            out.contains("no process listening") || out.starts_with("error:"),
            "unexpected: {out}"
        );
    }
}
