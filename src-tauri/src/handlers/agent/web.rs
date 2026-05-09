use std::time::Duration;

use serde_json::{json, Value};

use crate::models::agent::{ContextVisibility, GroundingSource};

use super::formatting::{extract_backticked, extract_quoted, truncate};
use super::safety::contains_any;

pub(super) fn search_searxng(
    base_url: &str,
    query: &str,
    limit: usize,
    timeout_secs: u64,
) -> Result<Vec<GroundingSource>, String> {
    if base_url.trim().is_empty() {
        return Err("agent.searxng_url is required for web.search".into());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_secs.max(1)))
        .build()
        .map_err(|e| e.to_string())?;
    let url = format!("{}/search", base_url.trim_end_matches('/'));
    let response = client
        .get(url)
        .query(&[("q", query), ("format", "json")])
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .map_err(|e| e.to_string())?;
    let results = response
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| "searxng response missing results".to_string())?;
    Ok(results
        .iter()
        .take(limit.max(1))
        .enumerate()
        .map(|(index, item)| {
            let title = item
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("Untitled")
                .to_string();
            let url = item
                .get("url")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            let snippet = item
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            GroundingSource {
                source_id: format!("web:searxng:{index}"),
                source_type: "web".into(),
                title,
                snippet: truncate(&snippet, 240),
                uri: url,
                score: 1.0 - (index as f32 * 0.03),
                visibility: ContextVisibility::PublicContext,
                redacted_reason: None,
            }
        })
        .collect())
}

pub(super) fn search_tavily(
    api_key: &str,
    query: &str,
    limit: usize,
    timeout_secs: u64,
) -> Result<Vec<GroundingSource>, String> {
    if api_key.trim().is_empty() {
        return Err("agent.web_search_api_key is required for Tavily web.search".into());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_secs.max(1)))
        .user_agent("Keynova/0.1 structured agent search")
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .post("https://api.tavily.com/search")
        .header("content-type", "application/json")
        .json(&json!({
            "api_key": api_key,
            "query": query,
            "search_depth": "basic",
            "max_results": limit.max(1),
            "include_answer": false,
            "include_raw_content": false,
        }))
        .send()
        .map_err(|e| format!("Tavily request failed: {e}"))?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .map_err(|e| e.to_string())?;
    parse_tavily_response(&response, limit)
}

pub(super) fn parse_tavily_response(response: &Value, limit: usize) -> Result<Vec<GroundingSource>, String> {
    let results = response
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| "Tavily response missing results".to_string())?;
    Ok(results
        .iter()
        .take(limit.max(1))
        .enumerate()
        .map(|(index, item)| {
            let title = item
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("Untitled")
                .to_string();
            let url = item
                .get("url")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            let snippet = item
                .get("content")
                .or_else(|| item.get("snippet"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let score = item
                .get("score")
                .and_then(Value::as_f64)
                .map(|value| value as f32)
                .unwrap_or(1.0 - (index as f32 * 0.03));
            GroundingSource {
                source_id: format!("web:tavily:{index}"),
                source_type: "web".into(),
                title,
                snippet: truncate(&snippet, 240),
                uri: url,
                score,
                visibility: ContextVisibility::PublicContext,
                redacted_reason: None,
            }
        })
        .collect())
}

pub(super) fn search_duckduckgo_html(
    query: &str,
    limit: usize,
    timeout_secs: u64,
) -> Result<Vec<GroundingSource>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_secs.max(1)))
        .user_agent("Keynova/0.1 read-only agent search")
        .build()
        .map_err(|e| e.to_string())?;
    let html = client
        .get("https://duckduckgo.com/html/")
        .query(&[("q", query)])
        .send()
        .map_err(|e| format!("DuckDuckGo request failed: {e}"))?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .text()
        .map_err(|e| e.to_string())?;
    let results = parse_duckduckgo_html_results(&html, limit.max(1));
    if results.is_empty() {
        return Err("DuckDuckGo returned no parseable results".into());
    }
    Ok(results)
}

pub(super) fn parse_duckduckgo_html_results(html: &str, limit: usize) -> Vec<GroundingSource> {
    let mut results = Vec::new();
    let mut rest = html;
    while results.len() < limit {
        let Some(link_pos) = rest.find("result__a") else {
            break;
        };
        rest = &rest[link_pos..];
        let Some(href_pos) = rest.find("href=\"") else {
            break;
        };
        let href_start = href_pos + "href=\"".len();
        let Some(href_end) = rest[href_start..].find('"') else {
            break;
        };
        let raw_href = &rest[href_start..href_start + href_end];
        let Some(text_start_rel) = rest[href_start + href_end..].find('>') else {
            break;
        };
        let text_start = href_start + href_end + text_start_rel + 1;
        let Some(text_end_rel) = rest[text_start..].find("</a>") else {
            break;
        };
        let title = strip_html(&rest[text_start..text_start + text_end_rel]);
        rest = &rest[text_start + text_end_rel..];

        let snippet = if let Some(snippet_pos) = rest.find("result__snippet") {
            let snippet_rest = &rest[snippet_pos..];
            if let Some(start_rel) = snippet_rest.find('>') {
                if let Some(end_rel) = snippet_rest[start_rel + 1..].find("</a>") {
                    strip_html(&snippet_rest[start_rel + 1..start_rel + 1 + end_rel])
                } else if let Some(end_rel) = snippet_rest[start_rel + 1..].find("</div>") {
                    strip_html(&snippet_rest[start_rel + 1..start_rel + 1 + end_rel])
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        if title.trim().is_empty() {
            continue;
        }
        results.push(GroundingSource {
            source_id: format!("web:duckduckgo:{}", results.len()),
            source_type: "web".into(),
            title,
            snippet: truncate(&snippet, 240),
            uri: Some(normalize_duckduckgo_href(raw_href)),
            score: 1.0 - (results.len() as f32 * 0.03),
            visibility: ContextVisibility::PublicContext,
            redacted_reason: None,
        });
    }
    results
}

pub(super) fn normalize_duckduckgo_href(raw_href: &str) -> String {
    let decoded = decode_html_entities(raw_href);
    if let Some(index) = decoded.find("uddg=") {
        let encoded = &decoded[index + "uddg=".len()..];
        let value = encoded.split('&').next().unwrap_or(encoded);
        return percent_decode(value);
    }
    decoded
}

pub(super) fn strip_html(value: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in value.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    decode_html_entities(out.trim())
}

pub(super) fn decode_html_entities(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

pub(super) fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[index + 1..index + 3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte);
                    index += 3;
                    continue;
                }
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

pub(super) fn extract_web_search_query(prompt: &str) -> Option<String> {
    let lower = prompt.to_lowercase();
    if !contains_any(
        &lower,
        &[
            "web",
            "internet",
            "online",
            "news",
            "latest",
            "網路",
            "网络",
            "上網",
            "联网",
            "查網路",
            "查网络",
            "新聞",
            "新闻",
            "最新",
            "今天",
        ],
    ) {
        return None;
    }

    if let Some(query) = extract_backticked(prompt).or_else(|| extract_quoted(prompt)) {
        return Some(query);
    }

    let query = prompt
        .trim()
        .trim_start_matches("please search")
        .trim_start_matches("search")
        .trim_start_matches("look up")
        .trim_start_matches("查詢")
        .trim_start_matches("查询")
        .trim_start_matches("幫我")
        .trim_start_matches("帮我")
        .trim_start_matches("查詢")
        .trim_start_matches("查询")
        .trim_start_matches("搜尋")
        .trim_start_matches("搜索")
        .trim_start_matches("查")
        .trim()
        .to_string();
    (!query.is_empty()).then_some(query)
}

pub(super) fn format_web_search_answer(query: &str, sources: &[GroundingSource]) -> String {
    let list = sources
        .iter()
        .take(5)
        .enumerate()
        .map(|(index, source)| {
            let uri = source.uri.as_deref().unwrap_or("no url");
            format!(
                "{}. {}\n   {}\n   {}",
                index + 1,
                source.title,
                uri,
                source.snippet
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "我查詢了網路：`{query}`\n\n找到 {} 個結果：\n{}\n\n這是 read-only 網路查詢；如果你要我根據結果整理摘要或後續工作流，可以直接接著下任務。",
        sources.len(),
        list
    )
}

#[derive(Debug, Clone)]
pub(super) struct GithubTrendingRepo {
    pub(super) owner: String,
    pub(super) name: String,
    pub(super) description: String,
    pub(super) url: String,
}

pub(super) fn is_github_trending_prompt(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    lower.contains("github")
        && (lower.contains("trending")
            || lower.contains("popular")
            || lower.contains("hot")
            || lower.contains("熱門")
            || lower.contains("热门")
            || lower.contains("最熱門")
            || lower.contains("最热门"))
}

pub(super) fn fetch_github_trending(limit: usize) -> Result<Vec<GithubTrendingRepo>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("Keynova/0.1 read-only github trending")
        .build()
        .map_err(|e| e.to_string())?;
    let html = client
        .get("https://github.com/trending")
        .query(&[("since", "daily")])
        .send()
        .map_err(|e| format!("GitHub request failed: {e}"))?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .text()
        .map_err(|e| e.to_string())?;
    Ok(parse_github_trending_html(&html, limit.max(1)))
}

pub(super) fn parse_github_trending_html(html: &str, limit: usize) -> Vec<GithubTrendingRepo> {
    let mut repos = Vec::new();
    let mut rest = html;
    while repos.len() < limit {
        let Some(article_pos) = rest.find("<article") else {
            break;
        };
        rest = &rest[article_pos..];
        let Some(article_end) = rest.find("</article>") else {
            break;
        };
        let article = &rest[..article_end];
        rest = &rest[article_end + "</article>".len()..];

        let Some(href_pos) = article.find("href=\"/") else {
            continue;
        };
        let href_start = href_pos + "href=\"/".len();
        let Some(href_end) = article[href_start..].find('"') else {
            continue;
        };
        let repo_path = article[href_start..href_start + href_end]
            .split_whitespace()
            .collect::<String>();
        let mut parts = repo_path.split('/');
        let (Some(owner), Some(name)) = (parts.next(), parts.next()) else {
            continue;
        };
        let description = extract_first_paragraph(article);
        repos.push(GithubTrendingRepo {
            owner: decode_html_entities(owner.trim()).to_string(),
            name: decode_html_entities(name.trim()).to_string(),
            description,
            url: format!("https://github.com/{repo_path}"),
        });
    }
    repos
}

pub(super) fn extract_first_paragraph(html: &str) -> String {
    let Some(p_pos) = html.find("<p") else {
        return String::new();
    };
    let rest = &html[p_pos..];
    let Some(start_rel) = rest.find('>') else {
        return String::new();
    };
    let Some(end_rel) = rest[start_rel + 1..].find("</p>") else {
        return String::new();
    };
    strip_html(&rest[start_rel + 1..start_rel + 1 + end_rel])
}

pub(super) fn format_github_trending_answer(repos: &[GithubTrendingRepo]) -> String {
    let list = repos
        .iter()
        .take(10)
        .enumerate()
        .map(|(index, repo)| {
            let desc = if repo.description.trim().is_empty() {
                "No description".to_string()
            } else {
                repo.description.clone()
            };
            format!(
                "{}. {}/{}\n   {}\n   {}",
                index + 1,
                repo.owner,
                repo.name,
                repo.url,
                desc
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "今天 GitHub Trending daily 前 {} 個專案：\n{}\n\n來源：GitHub Trending daily（read-only 查詢）。",
        repos.len().min(10),
        list
    )
}

pub(super) fn answer_workflow_plan(prompt: &str) -> Option<String> {
    let lower = prompt.to_lowercase();
    if !contains_any(
        &lower,
        &[
            "任務",
            "工作流",
            "計畫",
            "计划",
            "完成",
            "workflow",
            "plan",
            "task",
        ],
    ) {
        return None;
    }
    Some(format!(
        "我會把這個任務拆成可執行工作流：\n\n1. 釐清目標與輸出：確認你要的最終結果、限制、是否需要網路或本機資料。\n2. 蒐集資料：優先用 read-only 本機搜尋/讀取；需要外部資訊時使用 web.search。\n3. 制定步驟：把任務拆成可驗證的小步驟，標記哪些是 read-only、哪些需要 approval。\n4. 執行安全步驟：read-only 步驟可直接完成；任何修改檔案、terminal、system/model 動作都會先請你批准。\n5. 回報結果：列出完成項、證據來源、失敗原因與下一步。\n\n目前任務：\n{}",
        prompt.trim()
    ))
}
