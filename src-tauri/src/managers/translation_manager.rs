use std::sync::Arc;

use serde_json::Value;

use crate::core::AppEvent;

/// BCP-47 language code → display name (Traditional Chinese).
/// Covers all languages supported by Google Translate free API.
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
        tokio::spawn(async move {
            let result =
                translate_google_free(&req.src_lang, &req.dst_lang, &req.text, req.timeout_secs)
                    .await;
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

async fn translate_google_free(
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
        .await
        .map_err(|e| format!("GoogleFree request failed: {e}"))?;

    let status = response.status();
    let body_text = response
        .text()
        .await
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

fn build_client(timeout_secs: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
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
