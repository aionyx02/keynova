use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fmt;

/// Structured IPC error returned to the frontend command boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl IpcError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(
        code: impl Into<String>,
        message: impl Into<String>,
        details: Value,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: Some(details),
        }
    }

    pub fn handler(route: &str, message: impl Into<String>) -> Self {
        Self::with_details(
            "handler_error",
            message,
            json!({
                "route": route,
            }),
        )
    }

    pub fn state_lock(resource: &str, message: impl Into<String>) -> Self {
        Self::with_details(
            "state_lock_failed",
            message,
            json!({
                "resource": resource,
            }),
        )
    }

    pub fn tauri_api(action: &str, message: impl Into<String>) -> Self {
        Self::with_details(
            "tauri_api_error",
            message,
            json!({
                "action": action,
            }),
        )
    }
}

impl From<String> for IpcError {
    fn from(message: String) -> Self {
        Self::new("internal_error", message)
    }
}

impl From<&str> for IpcError {
    fn from(message: &str) -> Self {
        Self::new("internal_error", message)
    }
}

impl fmt::Display for IpcError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for IpcError {}
