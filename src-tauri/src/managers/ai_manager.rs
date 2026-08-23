use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::AppEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMessage {
    pub role: String,
    pub content: String,
}

/// Runtime AI provider selected from config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiProvider {
    Claude {
        api_key: String,
        model: String,
    },
    Ollama {
        base_url: String,
        model: String,
    },
    OpenAI {
        api_key: String,
        base_url: String,
        model: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiRuntimeConfig {
    pub provider: AiProvider,
    pub max_tokens: u32,
    pub timeout_secs: u64,
    pub ollama_keep_alive: String,
    /// When true, providers stream chunks via `ai.stream.chunk` events.
    pub stream_enabled: bool,
}

pub fn resolve_ai_runtime_config<F>(mut get: F) -> Result<AiRuntimeConfig, String>
where
    F: FnMut(&str) -> Option<String>,
{
    let allowed_hosts = crate::core::network_policy::allowlist_from_getter(&mut get);
    let provider_name = get("ai.provider")
        .unwrap_or_else(|| "claude".into())
        .trim()
        .to_lowercase();
    let model = selected_model_for_provider(&mut get, &provider_name);
    let max_tokens = get("ai.max_tokens")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(4096);

    let provider = match provider_name.as_str() {
        "ollama" => AiProvider::Ollama {
            base_url: crate::core::network_policy::enforce_outbound_url(
                &get("ai.ollama_url").unwrap_or_else(|| "http://localhost:11434".into()),
                &allowed_hosts,
                "ai.ollama_url",
            )?,
            model,
        },
        "openai" => AiProvider::OpenAI {
            api_key: get("ai.openai_api_key").unwrap_or_default(),
            base_url: crate::core::network_policy::enforce_outbound_url(
                &get("ai.openai_base_url").unwrap_or_else(|| "https://api.openai.com/v1".into()),
                &allowed_hosts,
                "ai.openai_base_url",
            )?,
            model,
        },
        "claude" => AiProvider::Claude {
            api_key: get("ai.api_key").unwrap_or_default(),
            model,
        },
        other => return Err(format!("unsupported ai.provider '{other}'")),
    };

    if matches!(&provider, AiProvider::Claude { .. }) {
        crate::core::network_policy::enforce_known_endpoint(
            "https://api.anthropic.com/v1/messages",
            &allowed_hosts,
            "Claude API",
        )?;
    }

    let timeout = match &provider {
        AiProvider::Ollama { .. } => get("ai.ollama_timeout_secs")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(120),
        _ => get("ai.timeout_secs")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(30),
    };

    let low_memory_mode = get("performance.low_memory_mode")
        .map(|v| v.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let ollama_keep_alive = get("ai.ollama_keep_alive").unwrap_or_else(|| "5m".into());
    let ollama_keep_alive = if low_memory_mode && ollama_keep_alive == "5m" {
        "0s".into()
    } else {
        ollama_keep_alive
    };

    let stream_enabled = get("ai.stream_enabled")
        .map(|v| !v.trim().eq_ignore_ascii_case("false"))
        .unwrap_or(true);

    Ok(AiRuntimeConfig {
        provider,
        max_tokens,
        timeout_secs: timeout,
        ollama_keep_alive,
        stream_enabled,
    })
}

fn selected_model_for_provider<F>(get: &mut F, provider_name: &str) -> String
where
    F: FnMut(&str) -> Option<String>,
{
    let active_model = get("ai.model").filter(|value| !value.trim().is_empty());
    match provider_name {
        "openai" => active_model
            .or_else(|| get("ai.openai_model"))
            .unwrap_or_else(|| "gpt-4o-mini".into()),
        "ollama" => active_model.unwrap_or_else(|| "qwen2.5:7b".into()),
        _ => active_model
            .or_else(|| get("ai.claude_model"))
            .unwrap_or_else(|| "claude-sonnet-4-6".into()),
    }
}

// ─── Response structs for chat APIs ─────────────────────────────────────────

/// Claude API response format used by the messages endpoint.
#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
}

#[derive(Deserialize)]
struct ClaudeContent {
    text: String,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: OllamaMessage,
}

#[derive(Deserialize)]
struct OllamaMessage {
    content: String,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Deserialize)]
struct OpenAiMessage {
    content: Option<String>,
}

/// Shared registry of in-flight cancel tokens keyed by request id. Producers
/// (handlers) own this Arc; consumers (the spawned chat thread) clone it to
/// self-clean their entry on completion. Aliased to satisfy `clippy::type_complexity`.
pub type CancelRegistry = Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>;

/// AI 對話管理器：管理多輪對話歷史，依 config 選擇 Claude/Ollama/OpenAI provider。
pub struct AiManager {
    publish_event: Arc<dyn Fn(AppEvent) + Send + Sync>,
    /// 多輪對話歷史（全域共享，前端可呼叫 clear_history 重置）
    history: Arc<Mutex<Vec<AiMessage>>>,
}

impl AiManager {
    pub fn new(publish_event: Arc<dyn Fn(AppEvent) + Send + Sync>) -> Self {
        Self {
            publish_event,
            history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 取得對話歷史。
    pub fn get_history(&self) -> Vec<AiMessage> {
        self.history.lock().map(|h| h.clone()).unwrap_or_default()
    }

    /// 清除對話歷史。
    pub fn clear_history(&self) {
        if let Ok(mut h) = self.history.lock() {
            h.clear();
        }
    }

    /// 非同步對話：立即回傳 request_id，結果透過 ai.response 事件推送。
    /// `completion_flag` — 若提供，回應發布後設為 `false`（用於 in-flight 追蹤）。
    /// `cancel_flag` — 若提供，呼叫端可隨時 set true 來請求中斷。check 點在
    ///   thread 起始與 `do_chat` 回應後。回應若已抵達但 cancel 已被設定，
    ///   reply 會被丟棄、user message rollback、事件以 `cancelled:true` 發出。
    /// `cancel_registry` — 若提供，thread 結束時會以 `request_id` 為 key 自動移除
    ///   對應 entry，避免 map 無限長大。
    /// `stream_enabled` — true 時走 streaming 路徑並發出 `ai.stream.chunk` 事件；
    ///   false 退回原本 single-shot `ai.response` 行為。
    #[allow(clippy::too_many_arguments)]
    pub fn chat_async(
        &self,
        request_id: String,
        prompt: String,
        provider: AiProvider,
        max_tokens: u32,
        timeout_secs: u64,
        keep_alive: String,
        completion_flag: Option<Arc<AtomicBool>>,
        cancel_flag: Option<Arc<AtomicBool>>,
        cancel_registry: Option<CancelRegistry>,
        stream_enabled: bool,
    ) {
        let publish = Arc::clone(&self.publish_event);
        let history = Arc::clone(&self.history);

        if let Ok(mut h) = history.lock() {
            h.push(AiMessage {
                role: "user".into(),
                content: prompt.clone(),
            });
            let len = h.len();
            if len > 20 {
                h.drain(0..len - 20);
            }
        }

        let messages_snapshot: Vec<AiMessage> =
            history.lock().map(|h| h.clone()).unwrap_or_default();

        std::thread::spawn(move || {
            let is_cancelled = || {
                cancel_flag
                    .as_ref()
                    .map(|f| f.load(Ordering::SeqCst))
                    .unwrap_or(false)
            };

            let rollback_user_msg = || {
                if let Ok(mut h) = history.lock() {
                    if let Some(pos) = h
                        .iter()
                        .rposition(|m| m.role == "user" && m.content == prompt)
                    {
                        h.remove(pos);
                    }
                }
            };

            // Early cancel: cancel arrived between dispatch and thread start.
            if is_cancelled() {
                rollback_user_msg();
                publish(AppEvent::new(
                    "ai.response",
                    serde_json::json!({
                        "request_id": request_id,
                        "ok": false,
                        "cancelled": true,
                        "error": "cancelled",
                    }),
                ));
            } else {
                let result = if stream_enabled {
                    let request_id_for_chunks = request_id.clone();
                    let publish_for_chunks = Arc::clone(&publish);
                    let on_chunk = move |delta: &str| {
                        publish_for_chunks(AppEvent::new(
                            "ai.stream.chunk",
                            serde_json::json!({
                                "request_id": request_id_for_chunks,
                                "delta": delta,
                            }),
                        ));
                    };
                    let cancel_check = || is_cancelled();
                    do_chat_stream(
                        &provider,
                        max_tokens,
                        timeout_secs,
                        &keep_alive,
                        &messages_snapshot,
                        &on_chunk,
                        &cancel_check,
                    )
                } else {
                    do_chat(
                        &provider,
                        max_tokens,
                        timeout_secs,
                        &keep_alive,
                        &messages_snapshot,
                    )
                };
                // Late cancel: cancel arrived during the HTTP request. Drop the reply,
                // rollback the user message, emit cancelled event instead of ok.
                if is_cancelled() {
                    rollback_user_msg();
                    publish(AppEvent::new(
                        "ai.response",
                        serde_json::json!({
                            "request_id": request_id,
                            "ok": false,
                            "cancelled": true,
                            "error": "cancelled",
                        }),
                    ));
                } else {
                    match result {
                        Ok(reply) => {
                            if let Ok(mut h) = history.lock() {
                                h.push(AiMessage {
                                    role: "assistant".into(),
                                    content: reply.clone(),
                                });
                            }
                            publish(AppEvent::new(
                                "ai.response",
                                serde_json::json!({
                                    "request_id": request_id,
                                    "ok": true,
                                    "reply": reply,
                                }),
                            ));
                        }
                        Err(e) => {
                            rollback_user_msg();
                            publish(AppEvent::new(
                                "ai.response",
                                serde_json::json!({
                                    "request_id": request_id,
                                    "ok": false,
                                    "error": e,
                                }),
                            ));
                        }
                    }
                }
            }

            if let Some(flag) = completion_flag {
                flag.store(false, Ordering::Relaxed);
            }
            if let Some(registry) = cancel_registry {
                if let Ok(mut map) = registry.lock() {
                    map.remove(&request_id);
                }
            }
        });
    }

    /// Blocking single-shot chat used by stateless `core/ai_capability` calls.
    ///
    /// Does NOT read or mutate `self.history` — capability execution is
    /// stateless per ADR-0029 §4. The caller passes a fully-assembled prompt;
    /// the provider sees exactly one user turn.
    pub fn chat_sync(
        &self,
        prompt: &str,
        provider: &AiProvider,
        max_tokens: u32,
        timeout_secs: u64,
        keep_alive: &str,
    ) -> Result<String, String> {
        let messages = [AiMessage {
            role: "user".into(),
            content: prompt.to_string(),
        }];
        do_chat(provider, max_tokens, timeout_secs, keep_alive, &messages)
    }

    /// Streaming single-shot chat for capabilities. `on_chunk(delta)` fires per
    /// provider chunk; `cancel()` is polled cooperatively. Returns the
    /// accumulated reply. Like `chat_sync`, does not touch `self.history`.
    #[allow(clippy::too_many_arguments)]
    pub fn chat_sync_stream(
        &self,
        prompt: &str,
        provider: &AiProvider,
        max_tokens: u32,
        timeout_secs: u64,
        keep_alive: &str,
        on_chunk: &dyn Fn(&str),
        cancel: &dyn Fn() -> bool,
    ) -> Result<String, String> {
        let messages = [AiMessage {
            role: "user".into(),
            content: prompt.to_string(),
        }];
        do_chat_stream(
            provider,
            max_tokens,
            timeout_secs,
            keep_alive,
            &messages,
            on_chunk,
            cancel,
        )
    }
}

fn do_chat(
    provider: &AiProvider,
    max_tokens: u32,
    timeout_secs: u64,
    keep_alive: &str,
    messages: &[AiMessage],
) -> Result<String, String> {
    match provider {
        AiProvider::Claude { api_key, model } => {
            chat_claude(api_key, model, max_tokens, timeout_secs, messages)
        }
        AiProvider::Ollama { base_url, model } => chat_ollama(
            base_url,
            model,
            max_tokens,
            timeout_secs,
            keep_alive,
            messages,
        ),
        AiProvider::OpenAI {
            api_key,
            base_url,
            model,
        } => chat_openai(api_key, base_url, model, max_tokens, timeout_secs, messages),
    }
}

fn chat_claude(
    api_key: &str,
    model: &str,
    max_tokens: u32,
    timeout_secs: u64,
    messages: &[AiMessage],
) -> Result<String, String> {
    if api_key.trim().is_empty() {
        return Err("AI API key not configured. Use /setting ai.api_key <key> to set it.".into());
    }

    let client = build_client(timeout_secs)?;
    let messages_json = messages_json(messages);
    let body = serde_json::json!({
        "model": model,
        "max_tokens": max_tokens,
        "system": "You are a helpful assistant integrated into Keynova, a keyboard-centric productivity launcher. Be concise and practical.",
        "messages": messages_json,
    });

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| format!("request failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().unwrap_or_default();
        return Err(format!("API error {status}: {body_text}"));
    }

    let resp: ClaudeResponse = response.json().map_err(|e| e.to_string())?;
    resp.content
        .into_iter()
        .next()
        .map(|c| c.text.trim().to_string())
        .ok_or_else(|| "empty response from API".into())
}

fn chat_ollama(
    base_url: &str,
    model: &str,
    max_tokens: u32,
    timeout_secs: u64,
    keep_alive: &str,
    messages: &[AiMessage],
) -> Result<String, String> {
    let client = build_client(timeout_secs)?;
    let url = format!("{}/api/chat", normalize_base_url(base_url));
    let body = serde_json::json!({
        "model": model,
        "stream": false,
        "keep_alive": keep_alive,
        "messages": messages_json(messages),
        "options": {
            "num_predict": max_tokens,
        },
    });

    let response = client
        .post(&url)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| {
            if e.is_connect() {
                format!("Ollama is not running at {url}. Start the Ollama daemon and try again.")
            } else {
                format!("Ollama request failed: {e}. For large local models, increase ai.ollama_timeout_secs.")
            }
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().unwrap_or_default();
        return Err(format!("Ollama error {status}: {body_text}"));
    }

    let resp: OllamaChatResponse = response.json().map_err(|e| e.to_string())?;
    let reply = resp.message.content.trim().to_string();
    if reply.is_empty() {
        Err("empty response from Ollama".into())
    } else {
        Ok(reply)
    }
}

// ─── Chunk parsers (pure, unit-testable) ────────────────────────────────────

/// Parse one NDJSON line from Ollama `/api/chat` streaming response.
/// Returns `(delta, done)`. `delta` is empty for control-only lines.
fn parse_ollama_chunk(line: &str) -> Option<(String, bool)> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let json = serde_json::from_str::<Value>(trimmed).ok()?;
    let delta = json
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string();
    let done = json.get("done").and_then(|d| d.as_bool()).unwrap_or(false);
    Some((delta, done))
}

/// Parse one SSE line from OpenAI `chat/completions` streaming response.
/// Returns `Some((delta, done))` for content lines, `None` for unrelated lines.
/// `done == true` signals `[DONE]` terminator.
fn parse_openai_chunk(line: &str) -> Option<(String, bool)> {
    let trimmed = line.trim_end_matches(['\r', '\n']);
    let payload = trimmed.strip_prefix("data: ")?;
    if payload == "[DONE]" {
        return Some((String::new(), true));
    }
    let json = serde_json::from_str::<Value>(payload).ok()?;
    let delta = json
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|c| c.get("delta"))
        .and_then(|d| d.get("content"))
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    Some((delta, false))
}

/// Parse one SSE line from Anthropic `messages` streaming response.
/// Returns `Some((delta, done))` for `content_block_delta` / `message_stop`,
/// `None` for unrelated lines.
fn parse_claude_chunk(line: &str) -> Option<(String, bool)> {
    let trimmed = line.trim_end_matches(['\r', '\n']);
    let payload = trimmed.strip_prefix("data: ")?;
    let json = serde_json::from_str::<Value>(payload).ok()?;
    let kind = json.get("type").and_then(|t| t.as_str()).unwrap_or("");
    match kind {
        "content_block_delta" => {
            let delta = json
                .get("delta")
                .and_then(|d| d.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            Some((delta, false))
        }
        "message_stop" => Some((String::new(), true)),
        _ => None,
    }
}

// ─── Streaming variants (Slice 3) ───────────────────────────────────────────
//
// Each `chat_*_stream` issues a streaming HTTP request, parses the provider's
// per-chunk format, invokes `on_chunk(delta)` for each text fragment, checks
// `cancel()` between chunks for cooperative interruption, and returns the
// accumulated full reply. Errors map to the same shape as the non-stream path.
//
// Provider chunk formats:
//   - Ollama  : NDJSON, one `{"message":{"content":"..."},"done":false}` per line.
//   - OpenAI  : SSE `data: {"choices":[{"delta":{"content":"..."}}]}\n\n`, ending with `data: [DONE]`.
//   - Claude  : SSE `event: content_block_delta\ndata: {"delta":{"text":"..."}}\n\n`.

#[allow(clippy::too_many_arguments)]
fn chat_ollama_stream(
    base_url: &str,
    model: &str,
    max_tokens: u32,
    timeout_secs: u64,
    keep_alive: &str,
    messages: &[AiMessage],
    on_chunk: &dyn Fn(&str),
    cancel: &dyn Fn() -> bool,
) -> Result<String, String> {
    use std::io::BufReader;

    let client = build_client(timeout_secs)?;
    let url = format!("{}/api/chat", normalize_base_url(base_url));
    let body = serde_json::json!({
        "model": model,
        "stream": true,
        "keep_alive": keep_alive,
        "messages": messages_json(messages),
        "options": { "num_predict": max_tokens },
    });

    let response = client
        .post(&url)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| {
            if e.is_connect() {
                format!("Ollama is not running at {url}. Start the Ollama daemon and try again.")
            } else {
                format!("Ollama request failed: {e}. For large local models, increase ai.ollama_timeout_secs.")
            }
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().unwrap_or_default();
        return Err(format!("Ollama error {status}: {body_text}"));
    }

    let reader = BufReader::new(response);
    accumulate_stream(reader, on_chunk, cancel, parse_ollama_chunk, "Ollama")
}

#[allow(clippy::too_many_arguments)]
fn chat_openai_stream(
    api_key: &str,
    base_url: &str,
    model: &str,
    max_tokens: u32,
    timeout_secs: u64,
    messages: &[AiMessage],
    on_chunk: &dyn Fn(&str),
    cancel: &dyn Fn() -> bool,
) -> Result<String, String> {
    use std::io::BufReader;

    if api_key.trim().is_empty() {
        return Err("OpenAI-compatible API key not configured.".into());
    }

    let client = build_client(timeout_secs)?;
    let url = format!("{}/chat/completions", normalize_base_url(base_url));
    let body = serde_json::json!({
        "model": model,
        "max_tokens": max_tokens,
        "stream": true,
        "messages": messages_json(messages),
    });

    let response = client
        .post(url)
        .bearer_auth(api_key)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| format!("request failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().unwrap_or_default();
        return Err(format!("OpenAI-compatible API error {status}: {body_text}"));
    }

    let reader = BufReader::new(response);
    accumulate_stream(
        reader,
        on_chunk,
        cancel,
        parse_openai_chunk,
        "OpenAI-compatible API",
    )
}

fn chat_claude_stream(
    api_key: &str,
    model: &str,
    max_tokens: u32,
    timeout_secs: u64,
    messages: &[AiMessage],
    on_chunk: &dyn Fn(&str),
    cancel: &dyn Fn() -> bool,
) -> Result<String, String> {
    use std::io::BufReader;

    if api_key.trim().is_empty() {
        return Err("AI API key not configured. Use /setting ai.api_key <key> to set it.".into());
    }

    let client = build_client(timeout_secs)?;
    let body = serde_json::json!({
        "model": model,
        "max_tokens": max_tokens,
        "stream": true,
        "system": "You are a helpful assistant integrated into Keynova, a keyboard-centric productivity launcher. Be concise and practical.",
        "messages": messages_json(messages),
    });

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| format!("request failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().unwrap_or_default();
        return Err(format!("API error {status}: {body_text}"));
    }

    let reader = BufReader::new(response);
    accumulate_stream(reader, on_chunk, cancel, parse_claude_chunk, "API")
}

/// Generic streaming accumulator. Reads lines from `reader`, runs each through
/// `parse`, fires `on_chunk` for non-empty deltas, and stops when `parse` reports
/// `done=true`, EOF, or `cancel()` returns true. Malformed lines are skipped
/// (parser returns `None`), matching provider quirks like stray comments and
/// empty heartbeat lines.
fn accumulate_stream<R, P>(
    mut reader: R,
    on_chunk: &dyn Fn(&str),
    cancel: &dyn Fn() -> bool,
    parse: P,
    provider_label: &str,
) -> Result<String, String>
where
    R: std::io::BufRead,
    P: Fn(&str) -> Option<(String, bool)>,
{
    let mut accumulated = String::new();
    let mut line = String::new();
    loop {
        if cancel() {
            return Err("cancelled".into());
        }
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let Some((delta, done)) = parse(&line) else {
                    continue;
                };
                if !delta.is_empty() {
                    on_chunk(&delta);
                    accumulated.push_str(&delta);
                }
                if done {
                    break;
                }
            }
            Err(e) => return Err(format!("{provider_label} stream read failed: {e}")),
        }
    }

    let reply = accumulated.trim().to_string();
    if reply.is_empty() {
        Err(format!("empty response from {provider_label}"))
    } else {
        Ok(reply)
    }
}

fn do_chat_stream(
    provider: &AiProvider,
    max_tokens: u32,
    timeout_secs: u64,
    keep_alive: &str,
    messages: &[AiMessage],
    on_chunk: &dyn Fn(&str),
    cancel: &dyn Fn() -> bool,
) -> Result<String, String> {
    match provider {
        AiProvider::Claude { api_key, model } => chat_claude_stream(
            api_key,
            model,
            max_tokens,
            timeout_secs,
            messages,
            on_chunk,
            cancel,
        ),
        AiProvider::Ollama { base_url, model } => chat_ollama_stream(
            base_url,
            model,
            max_tokens,
            timeout_secs,
            keep_alive,
            messages,
            on_chunk,
            cancel,
        ),
        AiProvider::OpenAI {
            api_key,
            base_url,
            model,
        } => chat_openai_stream(
            api_key,
            base_url,
            model,
            max_tokens,
            timeout_secs,
            messages,
            on_chunk,
            cancel,
        ),
    }
}

fn chat_openai(
    api_key: &str,
    base_url: &str,
    model: &str,
    max_tokens: u32,
    timeout_secs: u64,
    messages: &[AiMessage],
) -> Result<String, String> {
    if api_key.trim().is_empty() {
        return Err("OpenAI-compatible API key not configured.".into());
    }

    let client = build_client(timeout_secs)?;
    let url = format!("{}/chat/completions", normalize_base_url(base_url));
    let body = serde_json::json!({
        "model": model,
        "max_tokens": max_tokens,
        "messages": messages_json(messages),
    });

    let response = client
        .post(url)
        .bearer_auth(api_key)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| format!("request failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().unwrap_or_default();
        return Err(format!("OpenAI-compatible API error {status}: {body_text}"));
    }

    let resp: OpenAiResponse = response.json().map_err(|e| e.to_string())?;
    resp.choices
        .into_iter()
        .find_map(|choice| choice.message.content)
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .ok_or_else(|| "empty response from OpenAI-compatible API".into())
}

fn build_client(timeout_secs: u64) -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| e.to_string())
}

fn messages_json(messages: &[AiMessage]) -> Vec<serde_json::Value> {
    messages
        .iter()
        .map(|m| serde_json::json!({ "role": m.role, "content": m.content }))
        .collect()
}

fn normalize_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim();
    if trimmed.is_empty() {
        "https://api.openai.com/v1".to_string()
    } else {
        trimmed.trim_end_matches('/').to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn normalizes_base_urls() {
        assert_eq!(
            normalize_base_url("https://example.test/v1/"),
            "https://example.test/v1"
        );
        assert_eq!(normalize_base_url(""), "https://api.openai.com/v1");
    }

    #[test]
    fn resolves_selected_ollama_runtime_config_without_api_key() {
        let config = HashMap::from([
            ("ai.provider", "ollama"),
            ("ai.model", "qwen2.5:7b"),
            ("ai.ollama_url", "http://localhost:11434"),
            ("ai.ollama_timeout_secs", "180"),
            ("ai.max_tokens", "2048"),
        ]);

        let resolved =
            resolve_ai_runtime_config(|key| config.get(key).map(|value| value.to_string()))
                .expect("resolve config");

        assert_eq!(
            resolved,
            AiRuntimeConfig {
                provider: AiProvider::Ollama {
                    base_url: "http://localhost:11434".into(),
                    model: "qwen2.5:7b".into(),
                },
                max_tokens: 2048,
                timeout_secs: 180,
                ollama_keep_alive: "5m".into(),
                stream_enabled: true,
            }
        );
    }

    #[test]
    fn low_memory_mode_shrinks_default_ollama_keep_alive() {
        let config = HashMap::from([
            ("ai.provider", "ollama"),
            ("ai.model", "qwen2.5:7b"),
            ("performance.low_memory_mode", "true"),
        ]);

        let resolved =
            resolve_ai_runtime_config(|key| config.get(key).map(|value| value.to_string()))
                .expect("resolve config");

        assert_eq!(resolved.ollama_keep_alive, "0s");
    }

    #[test]
    fn low_memory_mode_preserves_custom_ollama_keep_alive() {
        let config = HashMap::from([
            ("ai.provider", "ollama"),
            ("ai.model", "qwen2.5:7b"),
            ("ai.ollama_keep_alive", "30s"),
            ("performance.low_memory_mode", "true"),
        ]);

        let resolved =
            resolve_ai_runtime_config(|key| config.get(key).map(|value| value.to_string()))
                .expect("resolve config");

        assert_eq!(resolved.ollama_keep_alive, "30s");
    }

    #[test]
    fn rejects_openai_base_url_outside_network_allowlist() {
        let config = HashMap::from([
            ("ai.provider", "openai"),
            ("ai.openai_api_key", "key"),
            ("ai.openai_model", "gpt-4o-mini"),
            ("ai.openai_base_url", "https://evil.example.com/v1"),
            ("security.network_allowlist", "api.openai.com"),
        ]);

        let error = resolve_ai_runtime_config(|key| config.get(key).map(|value| value.to_string()))
            .unwrap_err();
        assert!(error.contains("security.network_allowlist"));
    }

    #[test]
    fn rejects_claude_when_host_removed_from_allowlist() {
        let config = HashMap::from([
            ("ai.provider", "claude"),
            ("ai.api_key", "key"),
            ("security.network_allowlist", "api.openai.com"),
        ]);

        let error = resolve_ai_runtime_config(|key| config.get(key).map(|value| value.to_string()))
            .unwrap_err();
        assert!(error.contains("api.anthropic.com"));
    }

    #[test]
    fn resolves_openai_model_from_provider_specific_fallback() {
        let config = HashMap::from([
            ("ai.provider", "openai"),
            ("ai.openai_model", "gpt-4o-mini"),
            ("ai.openai_api_key", "key"),
            ("ai.openai_base_url", "https://api.openai.com/v1"),
        ]);

        let resolved =
            resolve_ai_runtime_config(|key| config.get(key).map(|value| value.to_string()))
                .expect("resolve config");

        assert_eq!(
            resolved.provider,
            AiProvider::OpenAI {
                api_key: "key".into(),
                base_url: "https://api.openai.com/v1".into(),
                model: "gpt-4o-mini".into(),
            }
        );
    }

    // ── 5.1.B: rposition message-retain fix ──────────────────────────────────

    #[test]
    fn rposition_removes_only_the_last_duplicate_user_message() {
        // Simulate: two identical user prompts in history; request fails.
        // With retain() the first message would also be gone.
        // With rposition() only the last matching entry is removed.
        let mut history = vec![
            AiMessage {
                role: "user".into(),
                content: "ping".into(),
            },
            AiMessage {
                role: "assistant".into(),
                content: "pong".into(),
            },
            AiMessage {
                role: "user".into(),
                content: "ping".into(),
            },
        ];
        let prompt = "ping";
        if let Some(pos) = history
            .iter()
            .rposition(|m| m.role == "user" && m.content == prompt)
        {
            history.remove(pos);
        }
        assert_eq!(
            history.len(),
            2,
            "only the last duplicate should be removed"
        );
        assert_eq!(history[0].role, "user");
        assert_eq!(history[0].content, "ping"); // first user message preserved
        assert_eq!(history[1].role, "assistant"); // assistant message preserved
    }

    #[test]
    fn rposition_retains_all_messages_when_prompt_not_found() {
        let mut history = vec![
            AiMessage {
                role: "user".into(),
                content: "hello".into(),
            },
            AiMessage {
                role: "assistant".into(),
                content: "hi".into(),
            },
        ];
        let before_len = history.len();
        let prompt = "nonexistent";
        if let Some(pos) = history
            .iter()
            .rposition(|m| m.role == "user" && m.content == prompt)
        {
            history.remove(pos);
        }
        assert_eq!(history.len(), before_len, "no message should be removed");
    }

    // ── Slice 3: streaming chunk parser tests ────────────────────────────────

    #[test]
    fn parse_ollama_chunk_extracts_delta_and_done_flag() {
        let line = r#"{"message":{"role":"assistant","content":"hello"},"done":false}"#;
        let (delta, done) = parse_ollama_chunk(line).unwrap();
        assert_eq!(delta, "hello");
        assert!(!done);

        let final_line = r#"{"message":{"role":"assistant","content":""},"done":true}"#;
        let (delta, done) = parse_ollama_chunk(final_line).unwrap();
        assert_eq!(delta, "");
        assert!(done);

        assert!(parse_ollama_chunk("   ").is_none());
        assert!(parse_ollama_chunk("not json").is_none());
    }

    #[test]
    fn parse_openai_chunk_decodes_sse_data_lines() {
        let line = r#"data: {"choices":[{"delta":{"content":"hi"}}]}"#;
        let (delta, done) = parse_openai_chunk(line).unwrap();
        assert_eq!(delta, "hi");
        assert!(!done);

        let (delta, done) = parse_openai_chunk("data: [DONE]").unwrap();
        assert_eq!(delta, "");
        assert!(done);

        assert!(parse_openai_chunk("event: ping").is_none());
        assert!(parse_openai_chunk("").is_none());
    }

    #[test]
    fn parse_claude_chunk_decodes_content_block_delta() {
        let line =
            r#"data: {"type":"content_block_delta","delta":{"type":"text_delta","text":"abc"}}"#;
        let (delta, done) = parse_claude_chunk(line).unwrap();
        assert_eq!(delta, "abc");
        assert!(!done);

        let stop = r#"data: {"type":"message_stop"}"#;
        let (delta, done) = parse_claude_chunk(stop).unwrap();
        assert_eq!(delta, "");
        assert!(done);

        let unrelated = r#"data: {"type":"message_start"}"#;
        assert!(parse_claude_chunk(unrelated).is_none());
    }

    #[test]
    fn accumulate_stream_concats_chunks_and_respects_done() {
        use std::cell::RefCell;

        let input = b"{\"message\":{\"content\":\"hello \"},\"done\":false}\n\
                      {\"message\":{\"content\":\"world\"},\"done\":false}\n\
                      {\"message\":{\"content\":\"\"},\"done\":true}\n\
                      {\"message\":{\"content\":\"after-done\"},\"done\":false}\n";

        let captured: RefCell<Vec<String>> = RefCell::new(Vec::new());
        let on_chunk = |delta: &str| {
            captured.borrow_mut().push(delta.to_string());
        };
        let cancel = || false;

        let reply = accumulate_stream(&input[..], &on_chunk, &cancel, parse_ollama_chunk, "Ollama")
            .unwrap();
        assert_eq!(reply, "hello world");
        assert_eq!(captured.borrow().as_slice(), &["hello ", "world"]);
    }

    #[test]
    fn accumulate_stream_returns_cancelled_when_cancel_set() {
        let input = b"{\"message\":{\"content\":\"x\"},\"done\":false}\n";
        let cancel = || true;
        let result = accumulate_stream(&input[..], &|_| {}, &cancel, parse_ollama_chunk, "Ollama");
        assert_eq!(result, Err("cancelled".into()));
    }

    // ── Slice 2: ai.cancel cooperative-abort tests ───────────────────────────

    /// `chat_async`'s worker publishes its terminal `ai.response` event and only
    /// then, at thread end, removes its `cancel_registry` entry. So receiving
    /// the event establishes nothing about the registry, and asserting on it
    /// straight afterwards is a race — one that turned `rust (windows-latest)`
    /// red on 2026-08-23.
    ///
    /// The production ordering is fine: cleanup is centralized at thread end so
    /// it runs on every path. It is the assertion that has to wait.
    fn wait_for_registry_drain(registry: &CancelRegistry) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !registry.lock().unwrap().is_empty() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    #[test]
    fn cancel_flag_aborts_pending_chat_before_request() {
        use std::sync::mpsc::channel;
        use std::time::Duration;

        let (tx, rx) = channel::<AppEvent>();
        let publish: Arc<dyn Fn(AppEvent) + Send + Sync> = Arc::new(move |event| {
            let _ = tx.send(event);
        });
        let manager = AiManager::new(publish);

        let cancel_flag = Arc::new(AtomicBool::new(true));
        let registry: CancelRegistry = Arc::new(Mutex::new(HashMap::new()));
        registry
            .lock()
            .unwrap()
            .insert("rid-1".to_string(), Arc::clone(&cancel_flag));

        manager.chat_async(
            "rid-1".into(),
            "hello".into(),
            AiProvider::Ollama {
                base_url: "http://127.0.0.1:1".into(),
                model: "x".into(),
            },
            32,
            1,
            "0".into(),
            None,
            Some(cancel_flag),
            Some(Arc::clone(&registry)),
            false, // non-stream path: keeps these tests independent of streaming logic
        );

        let event = rx
            .recv_timeout(Duration::from_secs(3))
            .expect("ai.response event must be published within 3s");
        assert_eq!(event.topic, "ai.response");
        assert_eq!(event.payload["request_id"], "rid-1");
        assert_eq!(event.payload["ok"], false);
        assert_eq!(event.payload["cancelled"], true);
        assert!(
            manager.get_history().is_empty(),
            "user message must be rolled back"
        );
        wait_for_registry_drain(&registry);
        assert!(
            registry.lock().unwrap().is_empty(),
            "cancel registry entry must be self-cleaned"
        );
    }

    #[test]
    fn registry_entry_is_cleaned_up_after_natural_completion() {
        use std::sync::mpsc::channel;
        use std::time::Duration;

        // Provider call will fail fast (unreachable URL + 1s timeout); we just need
        // the spawned thread to run completion cleanup.
        let (tx, rx) = channel::<AppEvent>();
        let publish: Arc<dyn Fn(AppEvent) + Send + Sync> = Arc::new(move |event| {
            let _ = tx.send(event);
        });
        let manager = AiManager::new(publish);

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let registry: CancelRegistry = Arc::new(Mutex::new(HashMap::new()));
        registry
            .lock()
            .unwrap()
            .insert("rid-2".to_string(), Arc::clone(&cancel_flag));

        manager.chat_async(
            "rid-2".into(),
            "hello".into(),
            AiProvider::Ollama {
                base_url: "http://127.0.0.1:1".into(),
                model: "x".into(),
            },
            32,
            1,
            "0".into(),
            None,
            Some(cancel_flag),
            Some(Arc::clone(&registry)),
            false, // non-stream path: keeps these tests independent of streaming logic
        );

        // Wait for completion event (will be ok:false network error).
        let _ = rx.recv_timeout(Duration::from_secs(5));
        wait_for_registry_drain(&registry);
        assert!(
            registry.lock().unwrap().is_empty(),
            "cancel registry entry must be removed on natural completion"
        );
    }

    #[test]
    fn rposition_handles_empty_history_gracefully() {
        let mut history: Vec<AiMessage> = Vec::new();
        let prompt = "anything";
        if let Some(pos) = history
            .iter()
            .rposition(|m| m.role == "user" && m.content == prompt)
        {
            history.remove(pos);
        }
        assert!(history.is_empty());
    }
}
