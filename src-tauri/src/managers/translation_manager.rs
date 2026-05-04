use std::sync::Arc;

use serde::Deserialize;

use crate::core::AppEvent;

/// Claude API 回應格式（部分欄位）。
#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
}

#[derive(Deserialize)]
struct ClaudeContent {
    text: String,
}

pub struct TranslateRequest {
    pub request_id: String,
    pub src_lang: String,
    pub dst_lang: String,
    pub text: String,
    pub api_key: String,
    pub model: String,
    pub timeout_secs: u64,
}

/// 翻譯管理器：使用 Claude API 進行翻譯，透過 EventBus 推送結果。
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
            let result = do_translate(&req.api_key, &req.model, &req.src_lang, &req.dst_lang, &req.text, req.timeout_secs);
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
    api_key: &str,
    model: &str,
    src: &str,
    dst: &str,
    text: &str,
    timeout_secs: u64,
) -> Result<String, String> {
    if api_key.is_empty() {
        return Err("AI API key not configured. Set ai.api_key in /setting.".into());
    }

    let src_desc = if src == "auto" { "the source language (auto-detect)".into() } else { src.to_string() };
    let prompt = format!(
        "Translate the following text from {src_desc} to {dst}. \
         Output ONLY the translated text, no explanations or notes.\n\n{text}"
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| e.to_string())?;

    let body = serde_json::json!({
        "model": model,
        "max_tokens": 2048,
        "messages": [{ "role": "user", "content": prompt }]
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