use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const CONTROL_ADDR: &str = "127.0.0.1:47839";

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlCommand {
    Start,
    Down,
    Reload,
    Status,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ControlRequest {
    pub command: ControlCommand,
    pub token: String,
}

impl ControlRequest {
    pub fn new(command: ControlCommand, token: String) -> Self {
        Self { command, token }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ControlResponse {
    pub ok: bool,
    pub message: String,
    #[serde(default)]
    pub data: Value,
}

impl ControlResponse {
    pub fn ok(message: impl Into<String>, data: Value) -> Self {
        Self {
            ok: true,
            message: message.into(),
            data,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            ok: false,
            message: message.into(),
            data: json!(null),
        }
    }
}

pub fn send_request(command: ControlCommand, timeout: Duration) -> Result<ControlResponse, String> {
    let token = load_or_create_control_token()?;
    let addr = CONTROL_ADDR
        .parse()
        .map_err(|e| format!("invalid control address: {e}"))?;
    let mut stream =
        TcpStream::connect_timeout(&addr, timeout).map_err(|e| format!("connect failed: {e}"))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|e| e.to_string())?;

    let body =
        serde_json::to_string(&ControlRequest::new(command, token)).map_err(|e| e.to_string())?;
    stream
        .write_all(format!("{body}\n").as_bytes())
        .map_err(|e| e.to_string())?;

    let mut line = String::new();
    let mut reader = BufReader::new(stream);
    reader.read_line(&mut line).map_err(|e| e.to_string())?;
    if line.trim().is_empty() {
        return Err("empty response from Keynova".into());
    }
    serde_json::from_str(line.trim()).map_err(|e| format!("invalid response: {e}"))
}

pub fn load_or_create_control_token() -> Result<String, String> {
    let path = control_token_path();
    if let Ok(value) = std::fs::read_to_string(&path) {
        let token = value.trim().to_string();
        if !token.is_empty() {
            return Ok(token);
        }
    }

    let token = generate_control_token();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    std::fs::write(&path, format!("{token}\n")).map_err(|e| format!("{}: {e}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }

    Ok(token)
}

pub fn control_request_authorized(request: &ControlRequest, expected_token: &str) -> bool {
    !expected_token.is_empty() && request.token == expected_token
}

fn control_token_path() -> PathBuf {
    crate::platform_dirs::keynova_config_dir().join("control-token")
}

fn generate_control_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn serve(
    handler: Arc<dyn Fn(ControlRequest) -> ControlResponse + Send + Sync + 'static>,
) -> Result<(), String> {
    let listener = bind_listener()?;
    serve_listener(listener, handler)
}

pub fn bind_listener() -> Result<TcpListener, String> {
    TcpListener::bind(CONTROL_ADDR).map_err(|e| e.to_string())
}

pub fn serve_listener(
    listener: TcpListener,
    handler: Arc<dyn Fn(ControlRequest) -> ControlResponse + Send + Sync + 'static>,
) -> Result<(), String> {
    for incoming in listener.incoming() {
        let handler = Arc::clone(&handler);
        match incoming {
            Ok(stream) => {
                std::thread::spawn(move || handle_client(stream, handler));
            }
            Err(e) => eprintln!("[keynova] control connection failed: {e}"),
        }
    }
    Ok(())
}

fn handle_client(
    stream: TcpStream,
    handler: Arc<dyn Fn(ControlRequest) -> ControlResponse + Send + Sync + 'static>,
) {
    let mut writer = match stream.try_clone() {
        Ok(stream) => stream,
        Err(e) => {
            eprintln!("[keynova] control stream clone failed: {e}");
            return;
        }
    };
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    let response = match reader.read_line(&mut line) {
        Ok(0) => ControlResponse::error("empty request"),
        Ok(_) => match serde_json::from_str::<ControlRequest>(line.trim()) {
            Ok(request) => handler(request),
            Err(e) => ControlResponse::error(format!("invalid request: {e}")),
        },
        Err(e) => ControlResponse::error(format!("read failed: {e}")),
    };

    match serde_json::to_string(&response) {
        Ok(body) => {
            let _ = writer.write_all(format!("{body}\n").as_bytes());
        }
        Err(e) => {
            let fallback = ControlResponse::error(format!("serialization failed: {e}"));
            if let Ok(body) = serde_json::to_string(&fallback) {
                let _ = writer.write_all(format!("{body}\n").as_bytes());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_control_token_is_non_empty_and_url_safe() {
        let token = generate_control_token();

        assert!(token.len() >= 40);
        assert!(token
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'));
    }

    #[test]
    fn control_request_authorization_requires_matching_token() {
        let request = ControlRequest::new(ControlCommand::Status, "secret-token".into());

        assert!(control_request_authorized(&request, "secret-token"));
        assert!(!control_request_authorized(&request, "other-token"));
        assert!(!control_request_authorized(&request, ""));
    }
}
