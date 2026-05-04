use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::Duration;

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
}

impl ControlRequest {
    pub fn new(command: ControlCommand) -> Self {
        Self { command }
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

    let body = serde_json::to_string(&ControlRequest::new(command)).map_err(|e| e.to_string())?;
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
