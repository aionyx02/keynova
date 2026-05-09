use std::path::PathBuf;

use super::safety::contains_any;

pub(super) fn should_run_local_search(prompt: &str) -> bool {
    let q = prompt.to_lowercase();
    if is_capability_question(prompt) {
        return true;
    }
    contains_any(
        &q,
        &[
            "keynova",
            "search",
            "find",
            "model",
            "note",
            "history",
            "setting",
            "workspace",
            "搜尋",
            "搜索",
            "模型",
            "筆記",
            "历史",
        ],
    )
}

pub(super) fn wants_whole_computer_search(prompt: &str) -> bool {
    let q = prompt.to_lowercase();
    contains_any(
        &q,
        &[
            "whole computer",
            "entire computer",
            "all computer",
            "all drives",
            "全電腦",
            "全电脑",
            "整台",
            "所有磁碟",
            "所有硬碟",
            "全機",
        ],
    )
}

pub(super) fn system_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    #[cfg(target_os = "windows")]
    {
        for letter in b'A'..=b'Z' {
            let root = format!("{}:\\", letter as char);
            let path = PathBuf::from(root);
            if path.is_dir() {
                roots.push(path);
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        roots.push(PathBuf::from("/"));
        if let Ok(home) = std::env::var("HOME") {
            roots.push(PathBuf::from(home));
        }
    }
    roots
}

pub(super) fn is_capability_question(prompt: &str) -> bool {
    let q = prompt.to_lowercase();
    contains_any(
        &q,
        &[
            "what can you do",
            "capability",
            "capabilities",
            "help me",
            "你可以",
            "可以做",
            "可以做到",
            "能做",
            "能幫",
            "能帮",
            "功能",
            "能力",
            "做什麼",
            "做什么",
            "做到甚麼",
            "做到什麼",
        ],
    )
}

pub(super) fn is_time_question(prompt: &str) -> bool {
    let q = prompt.to_lowercase();
    contains_any(
        &q,
        &[
            "current time",
            "what time",
            "date",
            "time now",
            "現在時間",
            "目前時間",
            "詳細時間",
            "幾點",
            "几点",
            "日期",
            "時間",
        ],
    )
}

