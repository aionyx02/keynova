use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::app::feature_registry::{AssemblyCtx, FeatureRegistrar, FeatureSpec};
use crate::core::config_manager::ConfigManager;
use crate::core::{AppEvent, CommandHandler, CommandResult, EventBus};
use crate::managers::portable_nvim_manager;

pub struct NvimHandler {
    event_bus: Arc<EventBus>,
    config: Arc<Mutex<ConfigManager>>,
}

impl NvimHandler {
    pub fn new(event_bus: Arc<EventBus>, config: Arc<Mutex<ConfigManager>>) -> Self {
        Self { event_bus, config }
    }
}

/// DECOUP.3 (ADR-0044): self-register nvim. No manager; uses the shared event
/// bus + config from ctx.
pub fn register(reg: &mut FeatureRegistrar, ctx: &AssemblyCtx) {
    reg.handler(Arc::new(NvimHandler::new(
        Arc::new(ctx.event_bus.clone()),
        Arc::clone(&ctx.config),
    )));
    reg.spec(FeatureSpec {
        namespace: "nvim",
        flag_key: None,
    });
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
                let allowed_hosts = {
                    let config = self.config.lock().map_err(|e| e.to_string())?;
                    crate::core::network_policy::allowlist_from_config(&config)
                };
                std::thread::spawn(move || {
                    let emit: Arc<dyn Fn(AppEvent) + Send + Sync> = Arc::new(move |evt| {
                        let _ = event_bus.publish(evt);
                    });
                    match portable_nvim_manager::download_nvim(Arc::clone(&emit), allowed_hosts) {
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
