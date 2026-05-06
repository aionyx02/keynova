use std::sync::Arc;

use serde_json::Value;

use crate::core::AppEvent;

pub struct TranslateRequest {
    pub request_id: String,
    pub src_lang: String,
    pub dst_lang: String,
    pub text: String,
    pub timeout_secs: u64,
}

pub struct TranslationManager {
    publish_event: Arc<dyn Fn(AppEvent) + Send + Sync>,
}

impl TranslationManager {
    pub fn new(publish_event: Arc<dyn Fn(AppEvent) + Send + Sync>) -> Self {
        Self { publish_event }
    }

    pub fn translate_async(&self, req: TranslateRequest) {
        let publish = Arc::clone(&self.publish_event);
        std::thread::spawn(move || {
            let result =
                translate_google_free(&req.src_lang, &req.dst_lang, &req.text, req.timeout_secs);
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

fn translate_google_free(
    src: &str,
    dst: &str,
    text: &str,
    timeout_secs: u64,
) -> Result<String, String> {
    let client = build_client(timeout_secs)?;
    let response = client
        .get("https://translate.googleapis.com/translate_a/single")
        .query(&[
            ("client", "gtx"),
            ("sl", if src.trim().is_empty() { "auto" } else { src }),
            ("tl", dst),
            ("dt", "t"),
            ("ie", "UTF-8"),
            ("oe", "UTF-8"),
            ("q", text),
        ])
        .header("accept", "application/json")
        .header("user-agent", "Mozilla/5.0 (Keynova)")
        .send()
        .map_err(|e| format!("GoogleFree request failed: {e}"))?;

    let status = response.status();
    let body_text = response
        .text()
        .map_err(|e| format!("GoogleFree response read failed: {e}"))?;

    if is_google_rate_limited(status, &body_text) {
        return Err("GoogleFree rate limit reached. Please retry later.".into());
    }

    if !status.is_success() {
        return Err(format!(
            "GoogleFree API error {status}: {}",
            compact_error_body(&body_text)
        ));
    }

    let value: Value = serde_json::from_str(&body_text)
        .map_err(|e| format!("GoogleFree response parsing failed: {e}"))?;
    extract_google_free_translation(&value)
}

fn build_client(timeout_secs: u64) -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| e.to_string())
}

fn extract_google_free_translation(value: &Value) -> Result<String, String> {
    let Some(segments) = value
        .as_array()
        .and_then(|items| items.first())
        .and_then(Value::as_array)
    else {
        return Err("GoogleFree returned an unexpected response format.".into());
    };

    let mut translated = String::new();
    for segment in segments {
        let Some(piece) = segment
            .as_array()
            .and_then(|items| items.first())
            .and_then(Value::as_str)
        else {
            continue;
        };
        translated.push_str(piece);
    }

    let translated = translated.trim().to_string();
    if translated.is_empty() {
        Err("GoogleFree returned an empty translation.".into())
    } else {
        Ok(translated)
    }
}

fn is_google_rate_limited(status: reqwest::StatusCode, body: &str) -> bool {
    let body = body.to_ascii_lowercase();
    status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || body.contains("rate limit")
        || body.contains("too many requests")
        || body.contains("quota exceeded")
        || body.contains("daily limit exceeded")
        || body.contains("user rate limit exceeded")
}

fn compact_error_body(body: &str) -> String {
    let compact = body
        .split_whitespace()
        .take(24)
        .collect::<Vec<_>>()
        .join(" ");
    if compact.is_empty() {
        "empty response body".into()
    } else {
        compact
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_google_free_translation_from_nested_response() {
        let value = serde_json::json!([
            [[
                "hola",
                "hello",
                null,
                null,
                11,
                null,
                null,
                [[]],
                [[["84b1db8c3c94d5ff25f228b8bdffe536", "zh_zh-hant_2023q3.md"]]]
            ]],
            null,
            "en",
            null,
            null,
            null,
            1,
            [],
            [["en"], null, [1], ["en"]]
        ]);

        assert_eq!(extract_google_free_translation(&value).unwrap(), "hola");
    }

    #[test]
    fn detects_google_free_rate_limit_messages() {
        assert!(is_google_rate_limited(
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            ""
        ));
        assert!(is_google_rate_limited(
            reqwest::StatusCode::FORBIDDEN,
            "Rate Limit Exceeded"
        ));
    }
}
