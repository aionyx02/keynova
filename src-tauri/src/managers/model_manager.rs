use std::cmp::Ordering;
use std::io::BufRead;
use std::sync::Arc;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::core::AppEvent;

/// Hardware capacity used to choose a reasonable local model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub ram_mb: u64,
    pub vram_mb: u64,
}

/// A local model recommendation with an approximate footprint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCandidate {
    pub name: String,
    pub provider: String,
    pub size_gb: f32,
    pub rating: String,
    pub source: String,
}

/// A model currently available from the local Ollama runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModel {
    pub name: String,
    pub provider: String,
    pub size_gb: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaTag>,
}

#[derive(Debug, Deserialize)]
struct OllamaTag {
    name: String,
    size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct OllamaPullChunk {
    status: Option<String>,
    completed: Option<u64>,
    total: Option<u64>,
    error: Option<String>,
}

/// Manages local model discovery and Ollama model lifecycle operations.
pub struct ModelManager {
    client: reqwest::blocking::Client,
    catalog_cache: Arc<Mutex<Vec<ModelCandidate>>>,
}

impl ModelManager {
    pub fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            catalog_cache: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Detects RAM and GPU VRAM where the platform exposes them.
    pub fn detect_hardware(&self) -> HardwareInfo {
        HardwareInfo {
            ram_mb: detect_ram_mb().unwrap_or(0),
            vram_mb: detect_vram_mb().unwrap_or(0),
        }
    }

    /// Recommends models from the curated catalog based on available memory.
    pub fn recommend_models(&self, hardware: &HardwareInfo) -> Vec<ModelCandidate> {
        rank_catalog(hardware, known_model_catalog())
    }

    /// Returns a fast model catalog from local cache and curated seeds.
    pub fn catalog_fast(&self, hardware: &HardwareInfo) -> Vec<ModelCandidate> {
        let mut models = self.recommend_models(hardware);
        if let Ok(cache) = self.catalog_cache.lock() {
            for candidate in cache.iter().cloned() {
                push_unique_model(&mut models, candidate);
            }
        }
        rank_catalog(hardware, models)
    }

    /// Refreshes Ollama library tags in the background and emits model.catalog.updated.
    pub fn refresh_catalog_async(
        &self,
        hardware: HardwareInfo,
        publish_event: Arc<dyn Fn(AppEvent) + Send + Sync>,
    ) {
        let client = self.client.clone();
        let cache = Arc::clone(&self.catalog_cache);
        std::thread::spawn(move || {
            let result = fetch_ollama_library(&client);
            match result {
                Ok(models) => {
                    if let Ok(mut slot) = cache.lock() {
                        *slot = models.clone();
                    }
                    let mut catalog = known_model_catalog();
                    for candidate in models {
                        push_unique_model(&mut catalog, candidate);
                    }
                    publish_event(AppEvent::new(
                        "model.catalog.updated",
                        serde_json::json!({ "models": rank_catalog(&hardware, catalog) }),
                    ));
                }
                Err(error) => {
                    publish_event(AppEvent::new(
                        "model.catalog.error",
                        serde_json::json!({ "error": error }),
                    ));
                }
            }
        });
    }

    /// Converts an Ollama model URL or plain name into a pullable model tag.
    pub fn parse_model_input(input: &str) -> Result<String, String> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err("model name or URL is empty".to_string());
        }

        let without_query = trimmed.split(['?', '#']).next().unwrap_or(trimmed);
        let candidate = without_query
            .trim_end_matches('/')
            .rsplit_once("/library/")
            .map(|(_, model)| model)
            .unwrap_or(without_query)
            .trim()
            .trim_start_matches("library/");

        let model = candidate.replace("%3A", ":").replace("%3a", ":");
        if model.contains("://") || model.contains('/') || model.contains('\\') {
            return Err(format!("unsupported model URL or name '{input}'"));
        }
        if model.is_empty() {
            return Err("model name or URL is empty".to_string());
        }
        Ok(model)
    }

    /// Lists models already downloaded in Ollama.
    pub fn list_local(&self, base_url: &str) -> Result<Vec<LocalModel>, String> {
        let url = format!("{}/api/tags", normalize_base_url(base_url));
        let response = self
            .client
            .get(url)
            .send()
            .map_err(|e| format!("Ollama is not reachable: {e}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(format!("Ollama list failed ({status}): {body}"));
        }

        let body: OllamaTagsResponse = response.json().map_err(|e| e.to_string())?;
        Ok(body
            .models
            .into_iter()
            .map(|model| LocalModel {
                name: model.name,
                provider: "ollama".to_string(),
                size_gb: model.size.map(bytes_to_gb),
            })
            .collect())
    }

    /// Checks whether a named Ollama model is already downloaded.
    pub fn check(&self, base_url: &str, name: &str) -> Result<bool, String> {
        Ok(self.list_local(base_url)?.iter().any(|m| m.name == name))
    }

    /// Pulls an Ollama model while emitting progress events.
    pub fn pull(
        &self,
        base_url: &str,
        name: &str,
        publish_event: Arc<dyn Fn(AppEvent) + Send + Sync>,
    ) -> Result<(), String> {
        let url = format!("{}/api/pull", normalize_base_url(base_url));
        let response = self
            .client
            .post(url)
            .json(&serde_json::json!({ "name": name, "stream": true }))
            .send()
            .map_err(|e| format!("Ollama pull failed: {e}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(format!("Ollama pull failed ({status}): {body}"));
        }

        let mut reader = std::io::BufReader::new(response);
        let mut line = String::new();
        loop {
            line.clear();
            let read = reader.read_line(&mut line).map_err(|e| e.to_string())?;
            if read == 0 {
                break;
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let chunk: OllamaPullChunk =
                serde_json::from_str(trimmed).map_err(|e| e.to_string())?;
            if let Some(error) = chunk.error {
                return Err(error);
            }

            let percent = match (chunk.completed, chunk.total) {
                (Some(completed), Some(total)) if total > 0 => {
                    Some(((completed as f64 / total as f64) * 100.0).min(100.0))
                }
                _ => None,
            };

            publish_event(AppEvent::new(
                "model.pull.progress",
                serde_json::json!({
                    "name": name,
                    "status": chunk.status.unwrap_or_else(|| "pulling".to_string()),
                    "completed_bytes": chunk.completed,
                    "total_bytes": chunk.total,
                    "percent": percent,
                }),
            ));
        }

        Ok(())
    }

    /// Deletes a local Ollama model.
    pub fn delete(&self, base_url: &str, name: &str) -> Result<(), String> {
        let url = format!("{}/api/delete", normalize_base_url(base_url));
        let response = self
            .client
            .delete(url)
            .json(&serde_json::json!({ "name": name }))
            .send()
            .map_err(|e| format!("Ollama delete failed: {e}"))?;

        if response.status().is_success() {
            return Ok(());
        }

        let status = response.status();
        let body = response.text().unwrap_or_default();
        Err(format!("Ollama delete failed ({status}): {body}"))
    }
}

impl Default for ModelManager {
    fn default() -> Self {
        Self::new()
    }
}

fn normalize_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim();
    if trimmed.is_empty() {
        "http://localhost:11434".to_string()
    } else {
        trimmed.trim_end_matches('/').to_string()
    }
}

fn bytes_to_gb(bytes: u64) -> f32 {
    (bytes as f32 / 1024.0 / 1024.0 / 1024.0 * 10.0).round() / 10.0
}

fn round_gb(size_gb: f32) -> f32 {
    (size_gb * 10.0).round() / 10.0
}

fn model_candidate(name: &str, size_gb: f32, rating: &str, source: &str) -> ModelCandidate {
    ModelCandidate {
        name: name.to_string(),
        provider: "ollama".to_string(),
        size_gb: round_gb(size_gb),
        rating: rating.to_string(),
        source: source.to_string(),
    }
}

fn known_model_catalog() -> Vec<ModelCandidate> {
    vec![
        model_candidate("qwen2.5:0.5b", 0.4, "Tiny multilingual model", "catalog"),
        model_candidate(
            "qwen2.5:1.5b",
            1.0,
            "Low memory multilingual default",
            "catalog",
        ),
        model_candidate("qwen2.5:3b", 1.9, "Balanced multilingual model", "catalog"),
        model_candidate("qwen2.5:7b", 4.7, "Strong local coding and chat", "catalog"),
        model_candidate("qwen2.5:14b", 9.0, "Higher quality, more memory", "catalog"),
        model_candidate("qwen2.5-coder:1.5b", 1.0, "Small coding helper", "catalog"),
        model_candidate(
            "qwen2.5-coder:7b",
            4.7,
            "Coding-focused 7B model",
            "catalog",
        ),
        model_candidate("llama3.2:1b", 1.3, "Very small and responsive", "catalog"),
        model_candidate("llama3.2:3b", 2.0, "Fast balanced default", "catalog"),
        model_candidate("llama3.1:8b", 4.7, "General quality default", "catalog"),
        model_candidate("mistral:7b", 4.1, "Fast 7B option", "catalog"),
        model_candidate("gemma4:e2b", 7.2, "Frontier small Gemma 4", "catalog"),
        model_candidate("gemma4:26b", 18.0, "Large Gemma 4 model", "catalog"),
        model_candidate("deepseek-r1:1.5b", 1.1, "Small reasoning model", "catalog"),
        model_candidate("deepseek-r1:7b", 4.7, "Reasoning model", "catalog"),
        model_candidate("phi4-mini:3.8b", 2.5, "Compact reasoning model", "catalog"),
    ]
}

fn preferred_models(hardware: &HardwareInfo) -> &'static [&'static str] {
    if hardware.ram_mb == 0 {
        return &["qwen2.5:1.5b", "llama3.2:1b", "deepseek-r1:1.5b"];
    }
    if hardware.ram_mb < 8 * 1024 {
        return &["qwen2.5:1.5b", "llama3.2:1b", "qwen2.5:0.5b"];
    }
    if hardware.ram_mb < 16 * 1024 && hardware.vram_mb < 6 * 1024 {
        return &["llama3.2:3b", "qwen2.5:3b", "phi4-mini:3.8b"];
    }
    if hardware.vram_mb >= 6 * 1024 {
        return &["gemma4:e2b", "qwen2.5:7b", "llama3.1:8b", "mistral:7b"];
    }
    &[
        "llama3.1:8b",
        "qwen2.5:7b",
        "mistral:7b",
        "qwen2.5-coder:7b",
    ]
}

fn recommended_rating(hardware: &HardwareInfo, candidate: &ModelCandidate) -> String {
    if hardware.ram_mb == 0 {
        return "Safe default until hardware is detected".to_string();
    }
    if candidate.size_gb > 0.0 {
        format!(
            "Recommended for {} GB RAM / {} GB VRAM",
            hardware.ram_mb / 1024,
            hardware.vram_mb / 1024
        )
    } else {
        "Recommended for this machine".to_string()
    }
}

fn rank_catalog(hardware: &HardwareInfo, mut models: Vec<ModelCandidate>) -> Vec<ModelCandidate> {
    let preferred = preferred_models(hardware);
    for candidate in &mut models {
        if preferred.iter().any(|name| *name == candidate.name) {
            candidate.source = "recommended".to_string();
            candidate.rating = recommended_rating(hardware, candidate);
        }
    }

    models.sort_by(|left, right| {
        let left_rank = preferred
            .iter()
            .position(|name| *name == left.name)
            .unwrap_or(usize::MAX);
        let right_rank = preferred
            .iter()
            .position(|name| *name == right.name)
            .unwrap_or(usize::MAX);
        left_rank
            .cmp(&right_rank)
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| {
                left.size_gb
                    .partial_cmp(&right.size_gb)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| left.name.cmp(&right.name))
    });
    models
}

fn push_unique_model(models: &mut Vec<ModelCandidate>, candidate: ModelCandidate) {
    if let Some(existing) = models.iter_mut().find(|model| model.name == candidate.name) {
        if existing.size_gb <= 0.0 && candidate.size_gb > 0.0 {
            existing.size_gb = candidate.size_gb;
        }
        if existing.rating == "Ollama library" && candidate.rating != "Ollama library" {
            existing.rating = candidate.rating;
        }
        if existing.source == "library" && candidate.source != "library" {
            existing.source = candidate.source;
        }
    } else {
        models.push(candidate);
    }
}

fn fetch_ollama_library(client: &reqwest::blocking::Client) -> Result<Vec<ModelCandidate>, String> {
    let response = client
        .get("https://ollama.com/library")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .map_err(|e| format!("model catalog refresh failed: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "model catalog refresh failed: {}",
            response.status()
        ));
    }
    let html = response.text().map_err(|e| e.to_string())?;
    let mut models = Vec::new();
    for name in parse_library_model_names(&html).into_iter().take(12) {
        match fetch_ollama_library_tags(client, &name) {
            Ok(candidates) if !candidates.is_empty() => {
                for candidate in candidates {
                    push_unique_model(&mut models, candidate);
                }
            }
            _ => push_unique_model(
                &mut models,
                model_candidate(
                    &name,
                    estimate_model_size_gb(&name).unwrap_or(0.0),
                    "Ollama library",
                    "library",
                ),
            ),
        }
    }
    Ok(models)
}

fn fetch_ollama_library_tags(
    client: &reqwest::blocking::Client,
    name: &str,
) -> Result<Vec<ModelCandidate>, String> {
    let url = format!("https://ollama.com/library/{name}/tags");
    let response = client
        .get(url)
        .timeout(std::time::Duration::from_secs(4))
        .send()
        .map_err(|e| format!("model tag refresh failed: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("model tag refresh failed: {}", response.status()));
    }
    let html = response.text().map_err(|e| e.to_string())?;
    Ok(parse_library_tag_candidates(&html, name, 8))
}

fn parse_library_model_names(html: &str) -> Vec<String> {
    let mut names = Vec::new();
    let marker = "href=\"/library/";
    let mut rest = html;
    while let Some(start) = rest.find(marker) {
        let after = &rest[start + marker.len()..];
        let Some(end) = after.find('"') else { break };
        let raw = &after[..end];
        let name = raw
            .split(['?', '#', '/'])
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        if !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
            && !names.iter().any(|existing| existing == &name)
        {
            names.push(name);
        }
        rest = &after[end..];
    }
    names
}

fn parse_library_tag_candidates(html: &str, base_name: &str, limit: usize) -> Vec<ModelCandidate> {
    let mut models = Vec::new();
    let marker = "href=\"/library/";
    let mut rest = html;
    while let Some(start) = rest.find(marker) {
        let after = &rest[start + marker.len()..];
        let Some(end) = after.find('"') else { break };
        let raw = after[..end].split(['?', '#', '/']).next().unwrap_or("");
        let name = raw.trim();
        if is_valid_model_tag(name) && name.starts_with(base_name) && name.contains(':') {
            let snippet_end = after.len().min(end + 900);
            let snippet = &after[end..snippet_end];
            let size_gb = parse_size_gb(snippet)
                .or_else(|| estimate_model_size_gb(name))
                .unwrap_or(0.0);
            push_unique_model(
                &mut models,
                model_candidate(name, size_gb, "Ollama library tag", "library"),
            );
            if models.len() >= limit {
                break;
            }
        }
        rest = &after[end..];
    }
    models
}

fn is_valid_model_tag(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '-' | '_' | '.'))
}

fn parse_size_gb(snippet: &str) -> Option<f32> {
    for unit in ["GB", "MB"] {
        if let Some(unit_pos) = snippet.find(unit) {
            let mut start = unit_pos;
            for (idx, ch) in snippet[..unit_pos].char_indices().rev() {
                if ch.is_ascii_digit() || ch == '.' {
                    start = idx;
                } else if start != unit_pos {
                    break;
                }
            }
            let value = snippet[start..unit_pos].trim().parse::<f32>().ok()?;
            return Some(if unit == "GB" {
                round_gb(value)
            } else {
                round_gb(value / 1024.0)
            });
        }
    }
    None
}

fn estimate_model_size_gb(name: &str) -> Option<f32> {
    let tag = name.rsplit_once(':').map(|(_, tag)| tag).unwrap_or(name);
    let marker = tag.find('b').or_else(|| tag.find('B'))?;
    let mut start = marker;
    for (idx, ch) in tag[..marker].char_indices().rev() {
        if ch.is_ascii_digit() || ch == '.' {
            start = idx;
        } else if start != marker {
            break;
        }
    }
    let params_b = tag[start..marker].parse::<f32>().ok()?;
    Some(round_gb((params_b * 0.62).max(0.3)))
}

#[cfg(target_os = "windows")]
fn detect_ram_mb() -> Option<u64> {
    detect_ram_mb_powershell().or_else(detect_ram_mb_wmic)
}

#[cfg(target_os = "windows")]
fn detect_ram_mb_wmic() -> Option<u64> {
    let output = std::process::Command::new("wmic")
        .args(["computersystem", "get", "TotalPhysicalMemory", "/value"])
        .output()
        .ok()?;
    parse_wmic_value(
        &String::from_utf8_lossy(&output.stdout),
        "TotalPhysicalMemory",
    )
    .map(|bytes| bytes / 1024 / 1024)
}

#[cfg(target_os = "linux")]
fn detect_ram_mb() -> Option<u64> {
    let meminfo = std::fs::read_to_string("/proc/meminfo").ok()?;
    meminfo.lines().find_map(|line| {
        let rest = line.strip_prefix("MemTotal:")?.trim();
        let kb = rest.split_whitespace().next()?.parse::<u64>().ok()?;
        Some(kb / 1024)
    })
}

#[cfg(target_os = "macos")]
fn detect_ram_mb() -> Option<u64> {
    let output = std::process::Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .ok()?;
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u64>()
        .ok()
        .map(|bytes| bytes / 1024 / 1024)
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn detect_ram_mb() -> Option<u64> {
    None
}

#[cfg(target_os = "windows")]
fn detect_vram_mb() -> Option<u64> {
    detect_vram_mb_nvidia_smi()
        .or_else(detect_vram_mb_powershell)
        .or_else(detect_vram_mb_wmic)
}

#[cfg(target_os = "windows")]
fn detect_vram_mb_wmic() -> Option<u64> {
    let output = std::process::Command::new("wmic")
        .args([
            "path",
            "Win32_VideoController",
            "get",
            "AdapterRAM",
            "/value",
        ])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let total_bytes = text
        .lines()
        .filter_map(|line| parse_wmic_line(line, "AdapterRAM"))
        .sum::<u64>();
    (total_bytes > 0).then_some(total_bytes / 1024 / 1024)
}

#[cfg(not(target_os = "windows"))]
fn detect_vram_mb() -> Option<u64> {
    None
}

#[cfg(target_os = "windows")]
fn parse_wmic_value(output: &str, key: &str) -> Option<u64> {
    output.lines().find_map(|line| parse_wmic_line(line, key))
}

#[cfg(target_os = "windows")]
fn parse_wmic_line(line: &str, key: &str) -> Option<u64> {
    let (found, value) = line.split_once('=')?;
    (found.trim() == key)
        .then(|| value.trim().parse::<u64>().ok())
        .flatten()
}

#[cfg(target_os = "windows")]
fn detect_ram_mb_powershell() -> Option<u64> {
    let script = "[uint64]((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1MB)";
    run_powershell_u64(script)
}

#[cfg(target_os = "windows")]
fn detect_vram_mb_powershell() -> Option<u64> {
    let script = "[uint64]((Get-CimInstance Win32_VideoController | Measure-Object AdapterRAM -Sum).Sum / 1MB)";
    run_powershell_u64(script)
}

#[cfg(target_os = "windows")]
fn detect_vram_mb_nvidia_smi() -> Option<u64> {
    let output = std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=memory.total", "--format=csv,noheader,nounits"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    parse_nvidia_smi_total_mb(&text)
}

#[cfg(target_os = "windows")]
fn parse_nvidia_smi_total_mb(output: &str) -> Option<u64> {
    let total = output
        .lines()
        .filter_map(|line| line.split(',').next()?.trim().parse::<u64>().ok())
        .sum::<u64>();
    (total > 0).then_some(total)
}

#[cfg(target_os = "windows")]
fn run_powershell_u64(script: &str) -> Option<u64> {
    let output = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .ok()?;
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommends_small_models_for_low_memory() {
        let manager = ModelManager::new();
        let models = manager.recommend_models(&HardwareInfo {
            ram_mb: 4096,
            vram_mb: 0,
        });
        assert_eq!(models[0].name, "qwen2.5:1.5b");
    }

    #[test]
    fn recommends_large_models_when_memory_allows() {
        let manager = ModelManager::new();
        let models = manager.recommend_models(&HardwareInfo {
            ram_mb: 32768,
            vram_mb: 0,
        });
        assert_eq!(models[0].name, "llama3.1:8b");
    }

    #[test]
    fn recommends_gpu_models_when_vram_allows() {
        let manager = ModelManager::new();
        let models = manager.recommend_models(&HardwareInfo {
            ram_mb: 32768,
            vram_mb: 8192,
        });
        assert_eq!(models[0].name, "gemma4:e2b");
        assert_eq!(models[0].source, "recommended");
    }

    #[test]
    fn normalizes_empty_ollama_url() {
        assert_eq!(normalize_base_url(""), "http://localhost:11434");
        assert_eq!(
            normalize_base_url("http://localhost:11434/"),
            "http://localhost:11434"
        );
    }

    #[test]
    fn parses_model_urls() {
        assert_eq!(
            ModelManager::parse_model_input("https://ollama.com/library/gemma4:e2b").unwrap(),
            "gemma4:e2b"
        );
        assert_eq!(
            ModelManager::parse_model_input("qwen2.5:1.5b").unwrap(),
            "qwen2.5:1.5b"
        );
    }

    #[test]
    fn parses_library_names_from_html() {
        let names = parse_library_model_names(
            r#"<a href="/library/qwen2.5">Qwen</a><a href="/library/gemma4">Gemma</a>"#,
        );
        assert_eq!(names, vec!["qwen2.5", "gemma4"]);
    }

    #[test]
    fn parses_library_tag_sizes_from_html() {
        let tags = parse_library_tag_candidates(
            r#"<a href="/library/gemma4:e2b" class="group">gemma4:e2b</a>
               <span>7fbdbf8f5e45</span> - 7.2GB - 128K context window
               <a href="/library/gemma4:26b" class="group">gemma4:26b</a>
               <span>5571076f3d70</span> - 18GB - 256K context window"#,
            "gemma4",
            4,
        );
        assert_eq!(tags[0].name, "gemma4:e2b");
        assert_eq!(tags[0].size_gb, 7.2);
        assert_eq!(tags[1].name, "gemma4:26b");
        assert_eq!(tags[1].size_gb, 18.0);
    }

    #[test]
    fn estimates_size_when_tag_page_has_no_footprint() {
        assert_eq!(estimate_model_size_gb("qwen2.5:7b"), Some(4.3));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn parses_nvidia_smi_total_memory() {
        assert_eq!(parse_nvidia_smi_total_mb("8188\n4096\n"), Some(12284));
    }
}
