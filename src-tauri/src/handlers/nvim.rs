use std::sync::Arc;

use serde_json::{json, Value};

use crate::core::{AppEvent, CommandHandler, CommandResult, EventBus};
use crate::managers::portable_nvim_manager;

pub struct NvimHandler {
    event_bus: Arc<EventBus>,
}

impl NvimHandler {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self { event_bus }
    }
}

impl CommandHandler for NvimHandler {
    fn namespace(&self) -> &'static str {
        "nvim"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "detect" => {
                let configured = payload
                    .get("configured_path")
                    .and_then(Value::as_str)
                    .map(String::from);
                let found = portable_nvim_manager::detect_nvim(configured.as_deref());
                Ok(json!({
                    "found": found.is_some(),
                    "path": found.map(|p| p.to_string_lossy().into_owned()),
                }))
            }
            "download" => {
                let event_bus = Arc::clone(&self.event_bus);
                std::thread::spawn(move || {
                    let emit: Arc<dyn Fn(AppEvent) + Send + Sync> =
                        Arc::new(move |evt| { let _ = event_bus.publish(evt); });
                    match portable_nvim_manager::download_nvim(Arc::clone(&emit)) {
                        Ok(path) => emit(AppEvent::new(
                            "nvim.download_progress",
                            json!({
                                "stage": "done",
                                "pct": 100,
                                "path": path.to_string_lossy().into_owned(),
                            }),
                        )),
                        Err(e) => emit(AppEvent::new(
                            "nvim.download_progress",
                            json!({ "stage": "error", "message": e }),
                        )),
                    }
                });
                Ok(json!({ "ok": true, "status": "started" }))
            }
            _ => Err(format!("unknown nvim command '{command}'")),
        }
    }
}