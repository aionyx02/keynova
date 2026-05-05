use std::sync::Arc;

use serde::Deserialize;

use crate::core::AppEvent;
use crate::managers::ai_manager::AiProvider;

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

pub struct TranslateRequest {
    pub request_id: String,
    pub src_lang: String,
    pub dst_lang: String,
    pub text: String,
    pub provider: AiProvider,
    pub timeout_secs: u64,
}

/// 翻譯管理器：依 translation.* 設定選擇 Claude/Ollama/OpenAI provider。
pub struct TranslationManager {
    publish_event: Arc<dyn Fn(AppEvent) + Send + Sync>,
}

impl TranslationManager {
    pub fn new(publish_event: Arc<dyn Fn(AppEvent) + Send + Sync>) -> Self {
        Self { publish_event }
    }

    /// 非同步翻譯：立即回傳，結果透過 translation.result 事件推送。
    pub fn translate_async(&self, req: TranslateRequest) {
        let publish = Arc::clone(&self.publish_event);
        std::thread::spawn(move || {
            let result = do_translate(
                &req.provider,
                &req.src_lang,
                &req.dst_lang,
                &req.text,
                req.timeout_secs,
            );
            let payload = match result {
                Ok(translated) => serde_json::json!({
                    "request_id": req.request_id,
                    "ok": true,
                    "text": translated,
                    "source_lang": req.src_lang,
                    "target_lang": req.dst_lang,
                }),
                Err(e) => serde_json::json!({
                    "request_id": req.request_id,
                    "ok": false,
                    "error": e,
                }),
            };
            publish(AppEvent::new("translation.result", payload));
        });
    }
}

fn do_translate(
    provider: &AiProvider,
    src: &str,
    dst: &str,
    text: &str,
    timeout_secs: u64,
) -> Result<String, String> {
    match provider {
        AiProvider::Claude { api_key, model } => {
            translate_claude(api_key, model, src, dst, text, timeout_secs)
        }
        AiProvider::Ollama { base_url, model } => {
            translate_ollama(base_url, model, src, dst, text, timeout_secs)
        }
        AiProvider::OpenAI {
            api_key,
            base_url,
            model,
        } => translate_openai(api_key, base_url, model, src, dst, text, timeout_secs),
    }
}

fn translate_claude(
    api_key: &str,
    model: &str,
    src: &str,
    dst: &str,
    text: &str,
    timeout_secs: u64,
) -> Result<String, String> {
    if api_key.trim().is_empty() {
        return Err("AI API key not configured. Set ai.api_key in /setting.".into());
    }

    let client = build_client(timeout_secs)?;
    let body = serde_json::json!({
        "model": model,
        "max_tokens": 2048,
        "messages": [{ "role": "user", "content": translation_prompt(src, dst, text) }]
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

fn translate_ollama(
    base_url: &str,
    model: &str,
    src: &str,
    dst: &str,
    text: &str,
    timeout_secs: u64,
) -> Result<String, String> {
    let client = build_client(timeout_secs)?;
    let body = serde_json::json!({
        "model": model,
        "stream": false,
        "messages": [
            {
                "role": "system",
                "content": "You are a translator. Output only the translation, no explanations or notes."
            },
            { "role": "user", "content": translation_prompt(src, dst, text) }
        ]
    });

    let response = client
        .post(format!("{}/api/chat", normalize_base_url(base_url)))
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| format!("Ollama request failed: {e}. For large local models, increase ai.ollama_timeout_secs."))?;

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().unwrap_or_default();
        return Err(format!("Ollama error {status}: {body_text}"));
    }

    let resp: OllamaChatResponse = response.json().map_err(|e| e.to_string())?;
    let text = resp.message.content.trim().to_string();
    if text.is_empty() {
        Err("empty response from Ollama".into())
    } else {
        Ok(text)
    }
}

fn translate_openai(
    api_key: &str,
    base_url: &str,
    model: &str,
    src: &str,
    dst: &str,
    text: &str,
    timeout_secs: u64,
) -> Result<String, String> {
    if api_key.trim().is_empty() {
        return Err("OpenAI-compatible API key not configured.".into());
    }

    let client = build_client(timeout_secs)?;
    let body = serde_json::json!({
        "model": model,
        "max_tokens": 2048,
        "messages": [
            {
                "role": "system",
                "content": "You are a translator. Output only the translation, no explanations or notes."
            },
            { "role": "user", "content": translation_prompt(src, dst, text) }
        ]
    });

    let response = client
        .post(format!("{}/chat/completions", normalize_base_url(base_url)))
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

fn translation_prompt(src: &str, dst: &str, text: &str) -> String {
    let src_desc = if src == "auto" {
        "the source language (auto-detect)".to_string()
    } else {
        src.to_string()
    };
    format!(
        "Translate the following text from {src_desc} to {dst}. \
         Output ONLY the translated text, no explanations or notes.\n\n{text}"
    )
}

fn build_client(timeout_secs: u64) -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| e.to_string())
}

fn normalize_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim();
    if trimmed.is_empty() {
        "http://localhost:11434".to_string()
    } else {
        trimmed.trim_end_matches('/').to_string()
    }
}
