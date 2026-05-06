use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::{json, Value};

use crate::core::config_manager::ConfigManager;
use crate::core::{AppEvent, CommandHandler, CommandResult};
use crate::managers::model_manager::{LocalModel, ModelManager};

#[derive(Debug, Serialize)]
struct ApiModel {
    name: String,
    provider: String,
    model: String,
    configured: bool,
    active: bool,
}

#[derive(Debug, Serialize)]
struct LocalModelEntry {
    name: String,
    provider: String,
    size_gb: Option<f32>,
    active: bool,
}

fn tool_label(tool: &str) -> &'static str {
    match tool {
        _ => "AI Chat",
    }
}

fn tool_keys(tool: &str) -> Result<(&'static str, &'static str), String> {
    match tool {
        "ai" | "chat" => Ok(("ai.provider", "ai.model")),
        "translation" | "tr" => {
            Err("Translation no longer has configurable models; /tr uses Google Translate.".into())
        }
        other => Err(format!("unsupported ai tool '{other}'")),
    }
}

fn normalized_tool(tool: Option<&str>) -> Result<&'static str, String> {
    match tool.unwrap_or("ai") {
        "ai" | "chat" => Ok("ai"),
        "translation" | "tr" => {
            Err("Translation no longer has configurable models; /tr uses Google Translate.".into())
        }
        other => Err(format!("unsupported ai tool '{other}'")),
    }
}

/// Handles `model.*` commands for local model discovery and active provider selection.
pub struct ModelHandler {
    manager: Arc<ModelManager>,
    config: Arc<Mutex<ConfigManager>>,
    publish_event: Arc<dyn Fn(AppEvent) + Send + Sync>,
}

impl ModelHandler {
    pub fn new(
        manager: Arc<ModelManager>,
        config: Arc<Mutex<ConfigManager>>,
        publish_event: Arc<dyn Fn(AppEvent) + Send + Sync>,
    ) -> Self {
        Self {
            manager,
            config,
            publish_event,
        }
    }

    fn ollama_url(&self) -> Result<String, String> {
        let cfg = self.config.lock().map_err(|e| e.to_string())?;
        Ok(cfg
            .get("ai.ollama_url")
            .unwrap_or_else(|| "http://localhost:11434".to_string()))
    }

    fn active_provider_model(&self, tool: &str) -> Result<(String, String), String> {
        let (provider_key, model_key) = tool_keys(tool)?;
        let cfg = self.config.lock().map_err(|e| e.to_string())?;
        Ok((
            cfg.get(provider_key)
                .unwrap_or_else(|| "claude".to_string()),
            cfg.get(model_key)
                .unwrap_or_else(|| "claude-sonnet-4-6".to_string()),
        ))
    }

    fn set_active_from_payload(&self, payload: &Value) -> Result<(), String> {
        let tool = normalized_tool(payload.get("tool").and_then(Value::as_str))?;
        let (provider_key, model_key) = tool_keys(tool)?;
        let provider = payload
            .get("provider")
            .and_then(Value::as_str)
            .ok_or_else(|| "missing 'provider'".to_string())?;
        let model = payload
            .get("model")
            .and_then(Value::as_str)
            .ok_or_else(|| "missing 'model'".to_string())?;
        let model = ModelManager::parse_model_input(model)?;

        let mut cfg = self.config.lock().map_err(|e| e.to_string())?;
        cfg.set(provider_key, provider)?;
        cfg.set(model_key, &model)?;

        match provider {
            "ollama" => {
                if let Some(base_url) = payload.get("base_url").and_then(Value::as_str) {
                    cfg.set("ai.ollama_url", base_url)?;
                }
            }
            "claude" => {
                cfg.set("ai.claude_model", &model)?;
                if let Some(api_key) = payload.get("api_key").and_then(Value::as_str) {
                    cfg.set("ai.api_key", api_key)?;
                }
            }
            "openai" => {
                cfg.set("ai.openai_model", &model)?;
                if let Some(api_key) = payload.get("api_key").and_then(Value::as_str) {
                    cfg.set("ai.openai_api_key", api_key)?;
                }
                if let Some(base_url) = payload.get("base_url").and_then(Value::as_str) {
                    cfg.set("ai.openai_base_url", base_url)?;
                }
            }
            other => return Err(format!("unsupported provider '{other}'")),
        }

        Ok(())
    }

    fn api_models(
        &self,
        active_provider: &str,
        active_model: &str,
    ) -> Result<Vec<ApiModel>, String> {
        let cfg = self.config.lock().map_err(|e| e.to_string())?;
        let claude_model = if active_provider == "claude" {
            active_model.to_string()
        } else {
            cfg.get("ai.claude_model")
                .unwrap_or_else(|| "claude-sonnet-4-6".to_string())
        };
        let openai_model = if active_provider == "openai" {
            active_model.to_string()
        } else {
            cfg.get("ai.openai_model")
                .unwrap_or_else(|| "gpt-4o-mini".to_string())
        };

        Ok(vec![
            ApiModel {
                name: "Claude API".to_string(),
                provider: "claude".to_string(),
                model: claude_model.clone(),
                configured: cfg.get("ai.api_key").is_some_and(|v| !v.trim().is_empty()),
                active: active_provider == "claude" && active_model == claude_model,
            },
            ApiModel {
                name: "OpenAI compatible".to_string(),
                provider: "openai".to_string(),
                model: openai_model.clone(),
                configured: cfg
                    .get("ai.openai_api_key")
                    .is_some_and(|v| !v.trim().is_empty()),
                active: active_provider == "openai" && active_model == openai_model,
            },
        ])
    }

    fn local_entries(
        &self,
        local_models: Vec<LocalModel>,
        active_provider: &str,
        active_model: &str,
    ) -> Vec<LocalModelEntry> {
        local_models
            .into_iter()
            .map(|model| LocalModelEntry {
                active: active_provider == "ollama" && active_model == model.name,
                name: model.name,
                provider: model.provider,
                size_gb: model.size_gb,
            })
            .collect()
    }
}

impl CommandHandler for ModelHandler {
    fn namespace(&self) -> &'static str {
        "model"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "detect_hardware" => Ok(json!(self.manager.detect_hardware())),
            "recommend" => {
                let hardware = self.manager.detect_hardware();
                let publish = Arc::clone(&self.publish_event);
                self.manager
                    .refresh_catalog_async(hardware.clone(), publish);
                Ok(json!(self.manager.catalog_fast(&hardware)))
            }
            "list_local" => {
                let tool = normalized_tool(payload.get("tool").and_then(Value::as_str))?;
                let base_url = self.ollama_url()?;
                let (active_provider, active_model) = self.active_provider_model(tool)?;
                let local = self.manager.list_local(&base_url)?;
                Ok(json!(self.local_entries(
                    local,
                    &active_provider,
                    &active_model
                )))
            }
            "list_available" => {
                let tool = normalized_tool(payload.get("tool").and_then(Value::as_str))?;
                let base_url = self.ollama_url()?;
                let (active_provider, active_model) = self.active_provider_model(tool)?;
                let local = self.manager.list_local(&base_url).unwrap_or_default();
                Ok(json!({
                    "tool": tool,
                    "tool_label": tool_label(tool),
                    "active_provider": active_provider,
                    "active_model": active_model,
                    "local_models": self.local_entries(local, &active_provider, &active_model),
                    "api_models": self.api_models(&active_provider, &active_model)?,
                }))
            }
            "check" => {
                let name = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing 'name'".to_string())?;
                let name = ModelManager::parse_model_input(name)?;
                let base_url = self.ollama_url()?;
                Ok(json!({ "exists": self.manager.check(&base_url, &name)?, "name": name }))
            }
            "pull" => {
                let tool =
                    normalized_tool(payload.get("tool").and_then(Value::as_str))?.to_string();
                let (provider_key, model_key) = tool_keys(&tool)?;
                let name = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing 'name'".to_string())?
                    .to_string();
                let name = ModelManager::parse_model_input(&name)?;
                let base_url = self.ollama_url()?;
                let manager = Arc::clone(&self.manager);
                let config = Arc::clone(&self.config);
                let publish_for_manager = Arc::clone(&self.publish_event);
                let publish_done = Arc::clone(&self.publish_event);
                let publish_error = Arc::clone(&self.publish_event);
                let name_for_thread = name.clone();
                let provider_key = provider_key.to_string();
                let model_key = model_key.to_string();
                let tool_for_thread = tool.clone();
                let tool_for_progress = tool.clone();
                let publish_for_manager = Arc::new(move |event: AppEvent| {
                    let mut payload = event.payload;
                    if let Some(obj) = payload.as_object_mut() {
                        obj.insert("tool".to_string(), json!(tool_for_progress));
                    }
                    publish_for_manager(AppEvent::new(event.topic, payload));
                });

                std::thread::spawn(move || {
                    let result = manager.pull(&base_url, &name_for_thread, publish_for_manager);
                    match result {
                        Ok(()) => {
                            let set_result =
                                config
                                    .lock()
                                    .map_err(|e| e.to_string())
                                    .and_then(|mut cfg| {
                                        cfg.set(&provider_key, "ollama")?;
                                        cfg.set(&model_key, &name_for_thread)?;
                                        cfg.set("ai.ollama_url", &base_url)
                                    });
                            match set_result {
                                Ok(()) => publish_done(AppEvent::new(
                                    "model.pull.done",
                                    json!({ "name": name_for_thread, "tool": tool_for_thread.clone() }),
                                )),
                                Err(error) => publish_error(AppEvent::new(
                                    "model.pull.error",
                                    json!({ "name": name_for_thread, "tool": tool_for_thread.clone(), "error": error }),
                                )),
                            }
                        }
                        Err(error) => publish_error(AppEvent::new(
                            "model.pull.error",
                            json!({ "name": name_for_thread, "tool": tool_for_thread.clone(), "error": error }),
                        )),
                    }
                });

                Ok(json!({ "status": "pending", "name": name, "tool": tool }))
            }
            "delete" => {
                let name = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing 'name'".to_string())?;
                let base_url = self.ollama_url()?;
                self.manager.delete(&base_url, name)?;
                Ok(json!({ "ok": true }))
            }
            "set_active" => {
                self.set_active_from_payload(&payload)?;
                Ok(json!({ "ok": true }))
            }
            _ => Err(format!("unknown model command '{command}'")),
        }
    }
}
