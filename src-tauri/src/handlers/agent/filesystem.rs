use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::managers::system_indexer::SystemSearchOutcome;

use super::formatting::{
    capability_answer, extract_backticked, extract_path_like, extract_quoted, truncate,
};
use super::intent::{is_capability_question, is_time_question};
#[cfg(test)]
use super::intent::wants_whole_computer_search;
use super::safety::contains_any;

pub(super) fn direct_local_answer(prompt: &str) -> Option<String> {
    if is_capability_question(prompt) {
        return Some(capability_answer());
    }
    if is_time_question(prompt) {
        let now = chrono::Local::now();
        return Some(format!(
            "目前時間是 {}。\n\n如果你想要我每次都用更完整格式顯示，可以問「顯示目前的詳細時間」；如果要開啟/執行本機動作，我會先列出 approval。",
            now.format("%Y-%m-%d %H:%M:%S %:z")
        ));
    }
    None
}

pub(super) fn answer_directory_listing(prompt: &str, roots: &[PathBuf]) -> Option<String> {
    let target = extract_directory_list_target(prompt)?;
    let (found, checked) = resolve_directory_target(&target, roots);
    let Some(path) = found else {
        let checked_text = if checked.is_empty() {
            "沒有可用的搜尋根目錄。".to_string()
        } else {
            checked
                .iter()
                .take(8)
                .map(|path| format!("- {}", path.display()))
                .collect::<Vec<_>>()
                .join("\n")
        };
        return Some(format!(
            "我找不到 `{target}` 資料夾。\n\n我已檢查：\n{checked_text}\n\n如果它在其他位置，可以用完整路徑再問一次，例如「列出 `C:\\path\\to\\hw` 裡的資料夾」。"
        ));
    };

    let mut directories = std::fs::read_dir(&path)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|entry| {
                    let file_type = entry.file_type().ok()?;
                    if !file_type.is_dir() {
                        return None;
                    }
                    Some(entry.file_name().to_string_lossy().to_string())
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    directories.sort_by_key(|name| name.to_lowercase());

    if directories.is_empty() {
        return Some(format!(
            "我找到了 `{target}` 資料夾：\n{}\n\n它目前沒有子資料夾。",
            path.display()
        ));
    }

    let list = directories
        .iter()
        .map(|name| format!("- {name}"))
        .collect::<Vec<_>>()
        .join("\n");
    Some(format!(
        "我找到了 `{target}` 資料夾：\n{}\n\n裡面有 {} 個子資料夾：\n{}",
        path.display(),
        directories.len(),
        list
    ))
}

#[derive(Debug, Clone)]
#[cfg(test)]
#[allow(dead_code)]
pub(super) struct FileSearchHit {
    pub(super) path: PathBuf,
    pub(super) is_dir: bool,
}

#[derive(Debug, Clone)]
#[cfg(test)]
#[allow(dead_code)]
pub(super) struct FileSearchOutcome {
    pub(super) hits: Vec<FileSearchHit>,
    pub(super) checked_roots: Vec<PathBuf>,
    pub(super) visited: usize,
    pub(super) stopped_early: bool,
}

#[cfg(test)]
pub(super) fn search_filesystem(query: &str, roots: &[PathBuf], limit: usize) -> FileSearchOutcome {
    let normalized = query.to_lowercase();
    let started = Instant::now();
    let mut hits = Vec::new();
    let mut visited = 0usize;
    let mut stopped_early = false;
    let max_results = limit.max(1);
    let max_visited = if wants_whole_computer_search(query) {
        40_000
    } else {
        12_000
    };

    for root in roots {
        let mut stack = vec![root.clone()];
        while let Some(path) = stack.pop() {
            if visited >= max_visited || started.elapsed() > Duration::from_millis(3500) {
                stopped_early = true;
                break;
            }
            visited += 1;

            let Ok(metadata) = std::fs::metadata(&path) else {
                continue;
            };
            let is_dir = metadata.is_dir();
            if path_matches_query(&path, &normalized) {
                hits.push(FileSearchHit {
                    path: path.clone(),
                    is_dir,
                });
                if hits.len() >= max_results {
                    stopped_early = true;
                    break;
                }
            }

            if !is_dir || should_skip_directory(&path) {
                continue;
            }
            let Ok(entries) = std::fs::read_dir(&path) else {
                continue;
            };
            for entry in entries.filter_map(Result::ok) {
                stack.push(entry.path());
            }
        }
        if stopped_early || hits.len() >= max_results {
            break;
        }
    }

    FileSearchOutcome {
        hits,
        checked_roots: roots.to_vec(),
        visited,
        stopped_early,
    }
}

#[cfg(test)]
pub(super) fn path_matches_query(path: &Path, query: &str) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.to_lowercase().contains(query))
}

#[cfg(test)]
#[allow(dead_code)]
pub(super) fn format_filesystem_search_answer(query: &str, outcome: &FileSearchOutcome) -> String {
    let roots = if outcome.checked_roots.is_empty() {
        "- 沒有可用搜尋根目錄".to_string()
    } else {
        outcome
            .checked_roots
            .iter()
            .take(8)
            .map(|root| format!("- {}", root.display()))
            .collect::<Vec<_>>()
            .join("\n")
    };

    if outcome.hits.is_empty() {
        return format!(
            "我沒有找到符合 `{query}` 的檔案或資料夾。\n\n已檢查 {} 個項目，搜尋根目錄：\n{}",
            outcome.visited, roots
        );
    }

    let list = outcome
        .hits
        .iter()
        .map(|hit| {
            format!(
                "- [{}] {}",
                if hit.is_dir { "folder" } else { "file" },
                hit.path.display()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let suffix = if outcome.stopped_early {
        "\n\n結果已達上限或時間上限，這是 bounded read-only 搜尋；你可以提供更精準關鍵字縮小範圍。"
    } else {
        ""
    };
    format!(
        "我找到 {} 個符合 `{query}` 的檔案/資料夾：\n{}\n\n已檢查 {} 個項目。{}",
        outcome.hits.len(),
        list,
        outcome.visited,
        suffix
    )
}

pub(super) fn format_system_index_search_answer(query: &str, outcome: &SystemSearchOutcome) -> String {
    let diagnostics = &outcome.diagnostics;
    let mut diagnostic_parts = vec![format!("provider={}", diagnostics.provider)];
    if let Some(reason) = diagnostics.fallback_reason.as_deref() {
        diagnostic_parts.push(format!("fallback={reason}"));
    }
    if diagnostics.visited > 0 {
        diagnostic_parts.push(format!("visited={}", diagnostics.visited));
    }
    if diagnostics.permission_denied > 0 {
        diagnostic_parts.push(format!(
            "permission_denied={}",
            diagnostics.permission_denied
        ));
    }
    if let Some(age) = diagnostics.index_age_secs {
        diagnostic_parts.push(format!("index_age_secs={age}"));
    }
    if diagnostics.timed_out {
        diagnostic_parts.push("timed_out=true".into());
    }
    if let Some(message) = diagnostics.message.as_deref() {
        diagnostic_parts.push(format!("message={message}"));
    }
    let diagnostics = diagnostic_parts.join(", ");

    if outcome.hits.is_empty() {
        return format!("No filesystem matches found for `{query}`.\n\nDiagnostics: {diagnostics}");
    }

    let list = outcome
        .hits
        .iter()
        .map(|hit| {
            format!(
                "- [{}] {}",
                if hit.is_dir { "folder" } else { "file" },
                hit.path
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Found {} filesystem matches for `{query}`.\n{}\n\nDiagnostics: {}",
        outcome.hits.len(),
        list,
        diagnostics
    )
}

pub(super) fn read_file_answer(target: &str, roots: &[PathBuf]) -> String {
    let (found, checked) = resolve_file_target(target, roots);
    let Some(path) = found else {
        let checked_text = checked
            .iter()
            .take(8)
            .map(|path| format!("- {}", path.display()))
            .collect::<Vec<_>>()
            .join("\n");
        return format!(
            "我找不到 `{target}` 這個檔案。\n\n我已檢查：\n{}",
            if checked_text.is_empty() {
                "- 沒有可用搜尋根目錄"
            } else {
                checked_text.as_str()
            }
        );
    };

    match read_text_preview(&path, 12_000) {
        Ok(preview) => format!(
            "我讀取了：\n{}\n\n內容預覽：\n```text\n{}\n```",
            path.display(),
            preview
        ),
        Err(error) => format!("我找到了 `{}`，但無法讀取文字內容：{error}", path.display()),
    }
}

pub(super) fn read_text_preview(path: &Path, max_chars: usize) -> Result<String, String> {
    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err("目標不是檔案".into());
    }
    const MAX_BYTES: u64 = 512 * 1024;
    if metadata.len() > MAX_BYTES {
        return Err(format!(
            "檔案大小 {} bytes 超過 read-only 預覽上限 {} bytes",
            metadata.len(),
            MAX_BYTES
        ));
    }
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    Ok(truncate(&content, max_chars))
}

#[derive(Debug, Clone)]
pub(super) struct ProjectTypeCount {
    pub(super) name: &'static str,
    pub(super) count: usize,
    pub(super) samples: Vec<PathBuf>,
}

pub(super) fn is_project_type_summary_prompt(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    lower.contains("project type")
        || lower.contains("project types")
        || lower.contains("repo type")
        || lower.contains("repo types")
        || lower.contains("專案類型")
        || lower.contains("项目类型")
        || (lower.contains("專案") && lower.contains("最多"))
        || (lower.contains("项目") && lower.contains("最多"))
}

pub(super) fn scan_project_types(roots: &[PathBuf]) -> Vec<ProjectTypeCount> {
    let started = Instant::now();
    let mut counts = project_type_markers()
        .iter()
        .map(|(name, _)| ProjectTypeCount {
            name,
            count: 0,
            samples: Vec::new(),
        })
        .collect::<Vec<_>>();
    let mut visited = 0usize;
    let max_visited = 60_000usize;

    for root in roots {
        let mut stack = vec![root.clone()];
        while let Some(path) = stack.pop() {
            if visited >= max_visited || started.elapsed() > Duration::from_millis(4500) {
                break;
            }
            visited += 1;
            if should_skip_directory(&path) {
                continue;
            }
            let Ok(entries) = std::fs::read_dir(&path) else {
                continue;
            };
            let mut child_dirs = Vec::new();
            let mut files = Vec::new();
            for entry in entries.filter_map(Result::ok) {
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };
                if file_type.is_dir() {
                    child_dirs.push(entry.path());
                } else if file_type.is_file() {
                    files.push(entry.file_name().to_string_lossy().to_lowercase());
                }
            }

            for (index, (_, markers)) in project_type_markers().iter().enumerate() {
                if markers.iter().any(|marker| {
                    if marker.starts_with('.') {
                        files.iter().any(|file| file.ends_with(marker))
                    } else {
                        files.iter().any(|file| file == marker)
                    }
                }) {
                    counts[index].count += 1;
                    if counts[index].samples.len() < 3 {
                        counts[index].samples.push(path.clone());
                    }
                }
            }
            stack.extend(child_dirs);
        }
    }

    counts.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.name.cmp(right.name))
    });
    counts
}

pub(super) fn project_type_markers() -> &'static [(&'static str, &'static [&'static str])] {
    &[
        ("JavaScript/TypeScript", &["package.json"]),
        ("Rust", &["cargo.toml"]),
        (
            "Python",
            &["pyproject.toml", "requirements.txt", "setup.py"],
        ),
        ("Go", &["go.mod"]),
        (
            "Java/Kotlin",
            &["pom.xml", "build.gradle", "build.gradle.kts"],
        ),
        ("C#/.NET", &[".sln", ".csproj"]),
        ("PHP", &["composer.json"]),
        ("Ruby", &["gemfile"]),
        ("Dart/Flutter", &["pubspec.yaml"]),
        ("C/C++", &["cmakelists.txt", "makefile"]),
    ]
}

pub(super) fn format_project_type_summary(counts: &[ProjectTypeCount]) -> String {
    let non_zero = counts
        .iter()
        .filter(|item| item.count > 0)
        .collect::<Vec<_>>();
    if non_zero.is_empty() {
        return "我做了 bounded read-only 專案類型掃描，但沒有找到常見專案 marker，例如 package.json、Cargo.toml、pyproject.toml、go.mod。".into();
    }
    let list = non_zero
        .iter()
        .take(8)
        .map(|item| {
            let samples = item
                .samples
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join("; ");
            format!(
                "- {}: {} 個{}",
                item.name,
                item.count,
                if samples.is_empty() {
                    String::new()
                } else {
                    format!("，例：{samples}")
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let top = non_zero[0];
    format!(
        "目前 bounded read-only 掃描結果顯示，最多的是 {} 專案，共 {} 個。\n\n統計：\n{}\n\n判斷依據是常見 marker 檔，例如 package.json、Cargo.toml、pyproject.toml、go.mod；不會修改任何檔案。",
        top.name,
        top.count,
        list
    )
}

pub(super) fn extract_directory_list_target(prompt: &str) -> Option<String> {
    let lower = prompt.to_lowercase();
    if !contains_any(
        &lower,
        &[
            "folder",
            "folders",
            "directory",
            "directories",
            "資料夾",
            "文件夾",
        ],
    ) {
        return None;
    }
    if !contains_any(
        &lower,
        &[
            "list",
            "find",
            "search",
            "what",
            "which",
            "under",
            "inside",
            "有哪些",
            "哪些",
            "列出",
            "搜尋",
            "搜索",
            "找",
            "裡面",
            "里面",
            "中有",
        ],
    ) {
        return None;
    }

    if let Some(value) = extract_backticked(prompt).or_else(|| extract_quoted(prompt)) {
        return Some(value);
    }
    if let Some(path) = extract_path_like(prompt) {
        return Some(path);
    }

    for marker in ["資料夾", "文件夾", " folder", " directory"] {
        if let Some(index) = lower.find(marker) {
            if let Some(target) = last_target_token(&prompt[..index]) {
                return Some(target);
            }
        }
    }

    for marker in [" in ", " under ", " inside "] {
        if let Some(index) = lower.find(marker) {
            let rest = &prompt[index + marker.len()..];
            if let Some(target) = first_target_token(rest) {
                return Some(target);
            }
        }
    }

    None
}

pub(super) fn extract_filesystem_search_query(prompt: &str) -> Option<String> {
    let lower = prompt.to_lowercase();
    if !contains_any(
        &lower,
        &[
            "filesystem",
            "file system",
            "whole computer",
            "entire computer",
            "all computer",
            "all drives",
            "全電腦",
            "全电脑",
            "整台",
            "所有磁碟",
            "所有硬碟",
            "檔案",
            "文件",
            "資料",
        ],
    ) || !contains_any(
        &lower,
        &[
            "search", "find", "look for", "搜尋", "搜索", "尋找", "查找", "找",
        ],
    ) {
        return None;
    }

    if let Some(value) = extract_backticked(prompt).or_else(|| extract_quoted(prompt)) {
        return Some(value);
    }

    let mut best = None;
    for token in prompt
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '，' | '。' | '、' | ':' | '：'))
        .map(clean_target_token)
    {
        let lower = token.to_lowercase();
        if token.len() >= 2
            && !matches!(
                lower.as_str(),
                "幫我"
                    | "帮我"
                    | "搜尋"
                    | "搜索"
                    | "尋找"
                    | "查找"
                    | "找"
                    | "全電腦"
                    | "全电脑"
                    | "整台"
                    | "資料"
                    | "檔案"
                    | "文件"
                    | "file"
                    | "files"
                    | "data"
                    | "computer"
                    | "whole"
                    | "entire"
            )
        {
            best = Some(token);
        }
    }
    best
}

pub(super) fn extract_file_read_target(prompt: &str) -> Option<String> {
    let lower = prompt.to_lowercase();
    if !contains_any(
        &lower,
        &[
            "read", "open", "show", "cat", "讀取", "读取", "打開", "打开", "顯示", "显示",
        ],
    ) {
        return None;
    }
    extract_backticked(prompt)
        .or_else(|| extract_quoted(prompt))
        .or_else(|| extract_path_like(prompt))
}

pub(super) fn resolve_directory_target(target: &str, roots: &[PathBuf]) -> (Option<PathBuf>, Vec<PathBuf>) {
    let target_path = PathBuf::from(target);
    let mut checked = Vec::new();
    if target_path.is_absolute() {
        checked.push(target_path.clone());
        return (target_path.is_dir().then_some(target_path), checked);
    }

    for root in roots {
        let candidate = root.join(target);
        checked.push(candidate.clone());
        if candidate.is_dir() {
            return (Some(candidate), checked);
        }
    }

    if target_path.components().count() == 1 {
        for root in roots {
            if let Some(found) = find_directory_by_name(root, target, 4, &mut checked) {
                return (Some(found), checked);
            }
        }
    }

    (None, checked)
}

pub(super) fn resolve_file_target(target: &str, roots: &[PathBuf]) -> (Option<PathBuf>, Vec<PathBuf>) {
    let target_path = PathBuf::from(target);
    let mut checked = Vec::new();
    if target_path.is_absolute() {
        checked.push(target_path.clone());
        return (target_path.is_file().then_some(target_path), checked);
    }

    for root in roots {
        let candidate = root.join(target);
        checked.push(candidate.clone());
        if candidate.is_file() {
            return (Some(candidate), checked);
        }
    }

    if target_path.components().count() == 1 {
        for root in roots {
            if let Some(found) = find_file_by_name(root, target, 4, &mut checked) {
                return (Some(found), checked);
            }
        }
    }

    (None, checked)
}

pub(super) fn find_directory_by_name(
    root: &Path,
    target: &str,
    max_depth: usize,
    checked: &mut Vec<PathBuf>,
) -> Option<PathBuf> {
    if max_depth == 0 || should_skip_directory(root) {
        return None;
    }
    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        checked.push(path.clone());
        let name = entry.file_name().to_string_lossy().to_string();
        if name.eq_ignore_ascii_case(target) {
            return Some(path);
        }
        if let Some(found) = find_directory_by_name(&path, target, max_depth - 1, checked) {
            return Some(found);
        }
    }
    None
}

pub(super) fn find_file_by_name(
    root: &Path,
    target: &str,
    max_depth: usize,
    checked: &mut Vec<PathBuf>,
) -> Option<PathBuf> {
    if max_depth == 0 || should_skip_directory(root) {
        return None;
    }
    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        checked.push(path.clone());
        let name = entry.file_name().to_string_lossy().to_string();
        if file_type.is_file() && name.eq_ignore_ascii_case(target) {
            return Some(path);
        }
        if file_type.is_dir() {
            if let Some(found) = find_file_by_name(&path, target, max_depth - 1, checked) {
                return Some(found);
            }
        }
    }
    None
}

pub(super) fn should_skip_directory(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            matches!(
                name.to_ascii_lowercase().as_str(),
                ".git" | ".idea" | "node_modules" | "target" | "dist"
            )
        })
}

pub(super) fn last_target_token(value: &str) -> Option<String> {
    value
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '，' | '。' | '、' | ':' | '：'))
        .rev()
        .map(clean_target_token)
        .find(|token| is_meaningful_target(token))
}

pub(super) fn first_target_token(value: &str) -> Option<String> {
    value
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '，' | '。' | '、' | ':' | '：'))
        .map(clean_target_token)
        .find(|token| is_meaningful_target(token))
}

pub(super) fn clean_target_token(value: &str) -> String {
    value
        .trim_matches(|ch| {
            matches!(
                ch,
                '"' | '\'' | '`' | ',' | '.' | '?' | '!' | '，' | '。' | '？' | '！' | '「' | '」'
            )
        })
        .trim_end_matches("裡")
        .trim_end_matches("里")
        .trim_end_matches("中")
        .to_string()
}

pub(super) fn is_meaningful_target(value: &str) -> bool {
    let lower = value.to_lowercase();
    !value.trim().is_empty()
        && !matches!(
            lower.as_str(),
            "幫我"
                | "帮我"
                | "搜尋"
                | "搜索"
                | "查詢"
                | "查询"
                | "找"
                | "列出"
                | "有哪些"
                | "哪些"
                | "folder"
                | "folders"
                | "directory"
                | "directories"
                | "in"
                | "under"
                | "inside"
        )
}

