use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;

use crate::core::AppEvent;

/// BCP-47 language code → display name (Traditional Chinese).
/// Covers all languages supported by Google Cloud Translation.
pub const SUPPORTED_LANGS: &[(&str, &str)] = &[
    ("auto", "自動偵測"),
    ("af", "南非荷蘭語"),
    ("sq", "阿爾巴尼亞語"),
    ("am", "阿姆哈拉語"),
    ("ar", "阿拉伯語"),
    ("hy", "亞美尼亞語"),
    ("az", "亞塞拜然語"),
    ("eu", "巴斯克語"),
    ("be", "白俄羅斯語"),
    ("bn", "孟加拉語"),
    ("bs", "波士尼亞語"),
    ("bg", "保加利亞語"),
    ("ca", "加泰羅尼亞語"),
    ("ceb", "宿霧語"),
    ("zh-CN", "簡體中文"),
    ("zh-TW", "繁體中文"),
    ("co", "科西嘉語"),
    ("hr", "克羅埃西亞語"),
    ("cs", "捷克語"),
    ("da", "丹麥語"),
    ("nl", "荷蘭語"),
    ("en", "英語"),
    ("eo", "世界語"),
    ("et", "愛沙尼亞語"),
    ("fi", "芬蘭語"),
    ("fr", "法語"),
    ("fy", "弗里斯蘭語"),
    ("gl", "加利西亞語"),
    ("ka", "喬治亞語"),
    ("de", "德語"),
    ("el", "希臘語"),
    ("gu", "古吉拉特語"),
    ("ht", "海地克里奧爾語"),
    ("ha", "豪薩語"),
    ("haw", "夏威夷語"),
    ("iw", "希伯來語"),
    ("hi", "印地語"),
    ("hmn", "苗語"),
    ("hu", "匈牙利語"),
    ("is", "冰島語"),
    ("ig", "伊博語"),
    ("id", "印尼語"),
    ("ga", "愛爾蘭語"),
    ("it", "義大利語"),
    ("ja", "日語"),
    ("jw", "爪哇語"),
    ("kn", "卡納達語"),
    ("kk", "哈薩克語"),
    ("km", "高棉語"),
    ("rw", "盧安達語"),
    ("ko", "韓語"),
    ("ku", "庫德語"),
    ("ky", "吉爾吉斯語"),
    ("lo", "寮語"),
    ("lv", "拉脫維亞語"),
    ("lt", "立陶宛語"),
    ("lb", "盧森堡語"),
    ("mk", "馬其頓語"),
    ("mg", "馬達加斯加語"),
    ("ms", "馬來語"),
    ("ml", "馬拉雅拉姆語"),
    ("mt", "馬爾他語"),
    ("mi", "毛利語"),
    ("mr", "馬拉地語"),
    ("mn", "蒙古語"),
    ("my", "緬甸語"),
    ("ne", "尼泊爾語"),
    ("no", "挪威語"),
    ("ny", "齊切瓦語"),
    ("or", "奧里亞語"),
    ("ps", "普什圖語"),
    ("fa", "波斯語"),
    ("pl", "波蘭語"),
    ("pt", "葡萄牙語"),
    ("pa", "旁遮普語"),
    ("ro", "羅馬尼亞語"),
    ("ru", "俄語"),
    ("sm", "薩摩亞語"),
    ("gd", "蘇格蘭蓋爾語"),
    ("sr", "塞爾維亞語"),
    ("st", "塞索托語"),
    ("sn", "紹納語"),
    ("sd", "信德語"),
    ("si", "僧伽羅語"),
    ("sk", "斯洛伐克語"),
    ("sl", "斯洛維尼亞語"),
    ("so", "索馬里語"),
    ("es", "西班牙語"),
    ("su", "巽他語"),
    ("sw", "斯瓦希里語"),
    ("sv", "瑞典語"),
    ("tl", "菲律賓語"),
    ("tg", "塔吉克語"),
    ("ta", "泰米爾語"),
    ("tt", "韃靼語"),
    ("te", "泰盧固語"),
    ("th", "泰語"),
    ("tr", "土耳其語"),
    ("tk", "土庫曼語"),
    ("uk", "烏克蘭語"),
    ("ur", "烏爾都語"),
    ("ug", "維吾爾語"),
    ("uz", "烏茲別克語"),
    ("vi", "越南語"),
    ("cy", "威爾斯語"),
    ("xh", "科薩語"),
    ("yi", "意第緒語"),
    ("yo", "約魯巴語"),
    ("zu", "祖魯語"),
];

pub struct TranslateRequest {
    pub request_id: String,
    pub src_lang: String,
    pub dst_lang: String,
    pub text: String,
    pub timeout_secs: u64,
    pub provider: String,
    pub api_key: String,
}

pub struct TranslationManager {
    publish_event: Arc<dyn Fn(AppEvent) + Send + Sync>,
}

enum TranslationProviderKind {
    GoogleCloudV2,
}

#[derive(Serialize)]
struct GoogleCloudV2TranslateBody<'a> {
    q: &'a str,
    target: &'a str,
    format: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<&'a str>,
}

impl TranslationManager {
    pub fn new(publish_event: Arc<dyn Fn(AppEvent) + Send + Sync>) -> Self {
        Self { publish_event }
    }

    pub fn translate_async(&self, req: TranslateRequest) {
        let publish = Arc::clone(&self.publish_event);
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tokio rt for translation");
            let result = match normalize_translation_provider(&req.provider) {
                Ok(TranslationProviderKind::GoogleCloudV2) => {
                    rt.block_on(translate_google_cloud_v2(
                        &req.src_lang,
                        &req.dst_lang,
                        &req.text,
                        &req.api_key,
                        req.timeout_secs,
                    ))
                }
                Err(error) => Err(error),
            };
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

fn normalize_translation_provider(provider: &str) -> Result<TranslationProviderKind, String> {
    match provider.trim() {
        "" | "google_cloud_v2" => Ok(TranslationProviderKind::GoogleCloudV2),
        other => Err(format!(
            "Unsupported translation provider '{other}'. Use /setting translation.provider to choose a supported provider."
        )),
    }
}

async fn translate_google_cloud_v2(
    src: &str,
    dst: &str,
    text: &str,
    api_key: &str,
    timeout_secs: u64,
) -> Result<String, String> {
    if api_key.trim().is_empty() {
        return Err(
            "Translation API key not configured. Use /setting translation.api_key to set it."
                .into(),
        );
    }

    let client = build_client(timeout_secs)?;
    let body = build_google_cloud_v2_request_body(src, dst, text);

    let response = client
        .post("https://translation.googleapis.com/language/translate/v2")
        .json(&body)
        .header("accept", "application/json")
        .header("x-goog-api-key", api_key.trim())
        .header("user-agent", "Keynova/0.3.0")
        .send()
        .await
        .map_err(|e| format!("Google Cloud Translation request failed: {e}"))?;

    let status = response.status();
    let body_text = response
        .text()
        .await
        .map_err(|e| format!("Google Cloud Translation response read failed: {e}"))?;

    if is_google_rate_limited(status, &body_text) {
        return Err(
            "Google Cloud Translation quota or rate limit reached. Please retry later or check quotas."
                .into(),
        );
    }

    if !status.is_success() {
        return Err(format!(
            "Google Cloud Translation API error {status}: {}",
            extract_google_error_message(&body_text)
                .or_else(|| extract_html_error_title(&body_text))
                .unwrap_or_else(|| compact_error_body(&body_text))
        ));
    }

    let value: Value = serde_json::from_str(&body_text)
        .map_err(|e| format!("Google Cloud Translation response parsing failed: {e}"))?;
    extract_google_cloud_translation(&value)
}

fn build_client(timeout_secs: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| e.to_string())
}

fn build_google_cloud_v2_request_body<'a>(
    src: &'a str,
    dst: &'a str,
    text: &'a str,
) -> GoogleCloudV2TranslateBody<'a> {
    GoogleCloudV2TranslateBody {
        q: text,
        target: dst,
        format: "text",
        source: if !src.trim().is_empty() && !src.eq_ignore_ascii_case("auto") {
            Some(src)
        } else {
            None
        },
    }
}

fn extract_google_cloud_translation(value: &Value) -> Result<String, String> {
    let Some(translations) = value
        .get("data")
        .and_then(|data| data.get("translations"))
        .and_then(Value::as_array)
    else {
        return Err("Google Cloud Translation returned an unexpected response format.".into());
    };

    let translated = translations
        .first()
        .and_then(|item| item.get("translatedText"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if translated.is_empty() {
        Err("Google Cloud Translation returned an empty translation.".into())
    } else {
        Ok(translated)
    }
}

fn extract_google_error_message(body: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    value
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(ToOwned::to_owned)
}

fn extract_html_error_title(body: &str) -> Option<String> {
    let lower = body.to_ascii_lowercase();
    let start = lower.find("<title>")? + "<title>".len();
    let end = start + lower[start..].find("</title>")?;
    let title = body[start..end]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let cleaned = title.trim().trim_end_matches("!!1").trim();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned.to_string())
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
    fn normalizes_google_cloud_v2_provider() {
        assert!(matches!(
            normalize_translation_provider("google_cloud_v2").unwrap(),
            TranslationProviderKind::GoogleCloudV2
        ));
    }

    #[test]
    fn extracts_google_cloud_translation_from_v2_response() {
        let value = serde_json::json!({
            "data": {
                "translations": [{
                    "translatedText": "hola",
                    "detectedSourceLanguage": "en"
                }]
            }
        });

        assert_eq!(extract_google_cloud_translation(&value).unwrap(), "hola");
    }

    #[test]
    fn extracts_google_error_message_from_structured_response() {
        let body = r#"{"error":{"code":403,"message":"Daily Limit Exceeded","errors":[]}}"#;
        assert_eq!(
            extract_google_error_message(body).as_deref(),
            Some("Daily Limit Exceeded")
        );
    }

    #[test]
    fn extracts_html_title_from_google_error_page() {
        let body = r#"<!DOCTYPE html><html lang=en><head><title>Error 411 (Length Required)!!1</title></head><body></body></html>"#;
        assert_eq!(
            extract_html_error_title(body).as_deref(),
            Some("Error 411 (Length Required)")
        );
    }

    #[test]
    fn builds_json_body_without_source_for_auto_detection() {
        let body = build_google_cloud_v2_request_body("auto", "zh-TW", "hello");
        assert_eq!(
            serde_json::to_value(&body).unwrap(),
            serde_json::json!({
                "q": "hello",
                "target": "zh-TW",
                "format": "text"
            })
        );
    }

    #[test]
    fn builds_json_body_with_explicit_source_language() {
        let body = build_google_cloud_v2_request_body("en", "zh-TW", "hello");
        assert_eq!(
            serde_json::to_value(&body).unwrap(),
            serde_json::json!({
                "q": "hello",
                "target": "zh-TW",
                "format": "text",
                "source": "en"
            })
        );
    }

    #[test]
    fn detects_google_rate_limit_messages() {
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
