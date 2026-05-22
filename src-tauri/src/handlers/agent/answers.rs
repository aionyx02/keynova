//! Chat-first direct-answer helpers for `AgentHandler` (heuristic offline path).
//!
//! Owns: `direct_local_answer` (the master router that fans out across all
//! direct-answer specialisations) and the six `answer_*` specialisations
//! (filesystem search / file read / project type summary / GitHub trending /
//! web search). Each one is the legacy "Agent answers in natural language"
//! surface that ADR-0029 retires.
//!
//! Deprecation: these helpers only support the chat-first behaviour gated by
//! `ai.legacy_agent`. They are deleted in REF.8 once the stateless capability
//! layer (REF.4) is the default surface.

use std::path::PathBuf;

use super::filesystem::{
    answer_directory_listing, direct_local_answer as direct_local_answer_fallback,
    extract_file_read_target, extract_filesystem_search_query, format_project_type_summary,
    format_system_index_search_answer, is_project_type_summary_prompt, read_file_answer,
    scan_project_types,
};
use super::web::{
    answer_workflow_plan, extract_web_search_query, fetch_github_trending,
    format_github_trending_answer, format_web_search_answer, is_github_trending_prompt,
};
use super::AgentHandler;
use crate::managers::system_indexer::search_system_index;

impl AgentHandler {
    pub(super) fn direct_local_answer(&self, prompt: &str) -> Option<String> {
        let roots = self.filesystem_search_roots_for_prompt(prompt);
        answer_directory_listing(prompt, &roots)
            .or_else(|| self.answer_file_read(prompt, &roots))
            .or_else(|| self.answer_project_type_summary(prompt, &roots))
            .or_else(|| self.answer_github_trending(prompt))
            .or_else(|| self.answer_filesystem_search(prompt, &roots))
            .or_else(|| self.answer_web_search(prompt))
            .or_else(|| answer_workflow_plan(prompt))
            .or_else(|| direct_local_answer_fallback(prompt))
    }

    fn answer_filesystem_search(&self, prompt: &str, roots: &[PathBuf]) -> Option<String> {
        let query = extract_filesystem_search_query(prompt)?;
        let outcome = search_system_index(&query, roots, 20, Some(&self.tantivy_index_dir));
        Some(format_system_index_search_answer(&query, &outcome))
    }

    fn answer_file_read(&self, prompt: &str, roots: &[PathBuf]) -> Option<String> {
        let target = extract_file_read_target(prompt)?;
        Some(read_file_answer(&target, roots))
    }

    fn answer_project_type_summary(&self, prompt: &str, roots: &[PathBuf]) -> Option<String> {
        if !is_project_type_summary_prompt(prompt) {
            return None;
        }
        Some(format_project_type_summary(&scan_project_types(roots)))
    }

    fn answer_github_trending(&self, prompt: &str) -> Option<String> {
        if !is_github_trending_prompt(prompt) {
            return None;
        }
        Some(match fetch_github_trending(10) {
            Ok(repos) if repos.is_empty() => {
                "我查詢了 GitHub Trending daily，但沒有解析到熱門專案。".into()
            }
            Ok(repos) => format_github_trending_answer(&repos),
            Err(error) => format!(
                "我目前無法查詢 GitHub Trending：{error}\n\n你也可以先用 web.search 查詢 `GitHub trending repositories today`。"
            ),
        })
    }

    fn answer_web_search(&self, prompt: &str) -> Option<String> {
        let query = extract_web_search_query(prompt)?;
        Some(match self.web_search(&query, 5) {
            Ok(sources) if sources.is_empty() => {
                format!("我查了網路，但沒有找到 `{query}` 的可用結果。")
            }
            Ok(sources) => format_web_search_answer(&query, &sources),
            Err(error) => format!("我目前無法完成網路查詢 `{query}`：{error}"),
        })
    }
}