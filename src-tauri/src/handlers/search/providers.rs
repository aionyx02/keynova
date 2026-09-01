//! Non-file result providers: command / note / history / model search sources
//! appended to the result set.
//!
//! These remain inherent methods on `SearchHandler`; only
//! `append_non_file_results` is reachable from the parent module.

use serde_json::json;

use crate::core::action_registry::ActionSession;
use crate::core::ai_capability::memory::{term_score, PERSONAL_MEMORY_SCOPE};
use crate::handlers::builtin_cmd::COMMAND_FEATURE_GUARDS;
use crate::managers::model_manager::HardwareInfo;
use crate::models::action::{Action, ScoreBreakdown, UiSearchItem};
use crate::models::search_result::ResultKind;

use super::ranking::command_match_score;
use super::{SearchHandler, SearchPlan};

impl SearchHandler {
    pub(super) fn append_non_file_results(
        &self,
        query: &str,
        plan: &SearchPlan,
        session: &ActionSession,
        out: &mut Vec<UiSearchItem>,
    ) -> Result<(), String> {
        // Declarative provider chain (DECOUP.6 / ADR-0044): `(provider, limit,
        // gate)`. Per ADR-0044 §2 `search` stays a cross-cutting consumer — its
        // providers share the handler's managers rather than being owned by each
        // feature — but the chain order + feature gating now live in one list, so
        // a gated provider is a one-line change. `command` self-filters per
        // command; `model` is ungated (AI-config bootstrap). The gate flags match
        // the dispatch-guard / `COMMAND_FEATURE_GUARDS` source of truth.
        type Provider =
            fn(&SearchHandler, &str, usize, &ActionSession, &mut Vec<UiSearchItem>) -> Result<(), String>;
        let chain: [(Provider, usize, Option<&str>); 6] = [
            (SearchHandler::append_command_results, plan.command_limit, None),
            (SearchHandler::append_project_command_results, plan.command_limit, None),
            (SearchHandler::append_note_results, plan.note_limit, Some("features.notes")),
            (SearchHandler::append_history_results, plan.history_limit, Some("features.history")),
            (SearchHandler::append_memory_results, plan.memory_limit, Some("features.ai")),
            (SearchHandler::append_model_results, plan.model_limit, None),
        ];
        for (run, limit, gate) in chain {
            if gate.is_none_or(|flag| self.feature_enabled(flag)) {
                run(self, query, limit, session, out)?;
            }
        }
        Ok(())
    }

    fn append_command_results(
        &self,
        query: &str,
        limit: usize,
        session: &ActionSession,
        out: &mut Vec<UiSearchItem>,
    ) -> Result<(), String> {
        let q = query.to_lowercase();
        let registry = self.builtin_registry.lock().map_err(|e| e.to_string())?;
        if let Some((name, args)) = parse_direct_utility_query(query) {
            if let Some(meta) = registry.list().into_iter().find(|meta| meta.name == name) {
                let display = if args.is_empty() {
                    format!("/{name}")
                } else {
                    format!("/{name} {args}")
                };
                let action = Action::command_route(
                    format!("cmd:{name}:direct:{args}"),
                    display.clone(),
                    "cmd.run",
                    json!({ "name": name, "args": args }),
                );
                let action_ref = self.action_arena.insert(session, action)?;
                let hint = meta.args_hint.as_deref().unwrap_or("");
                let subtitle = if args.is_empty() && !hint.is_empty() {
                    format!("{} · {}", meta.description, hint)
                } else {
                    format!("{} · inline result", meta.description)
                };
                let mut item = UiSearchItem {
                    item_ref: action_ref.clone(),
                    title: display,
                    subtitle,
                    source: "command".into(),
                    score: 110,
                    icon_key: Some("command".into()),
                    primary_action: action_ref,
                    primary_action_label: "Use".into(),
                    secondary_action_count: 0,
                    kind: ResultKind::Command,
                    name: name.to_string(),
                    path: format!("command://{name}:direct:{args}"),
                    score_breakdown: ScoreBreakdown::default(),
                };
                self.apply_rank_boost(&mut item);
                out.push(item);
                return Ok(());
            }
        }
        for (meta, score) in registry
            .list()
            .into_iter()
            .filter_map(|meta| {
                // FEAT.GATE fix: hide a disabled feature's command from search so
                // its visibility matches executability (the builtin handler also
                // refuses it). Same source of truth as the handler guard.
                if COMMAND_FEATURE_GUARDS
                    .iter()
                    .any(|&(name, flag)| name == meta.name && !self.feature_enabled(flag))
                {
                    return None;
                }
                let score = command_match_score(&meta.name, &meta.description, &q)?;
                Some((meta, score))
            })
            .take(limit)
        {
            let action = Action::command_route(
                format!("cmd:{}", meta.name),
                format!("/{}", meta.name),
                "cmd.run",
                json!({ "name": meta.name, "args": "" }),
            );
            let action_ref = self.action_arena.insert(session, action)?;
            let mut item = UiSearchItem {
                item_ref: action_ref.clone(),
                title: format!("/{}", meta.name),
                subtitle: meta.description.to_string(),
                source: "command".into(),
                score,
                icon_key: Some("command".into()),
                primary_action: action_ref,
                primary_action_label: "Run".into(),
                secondary_action_count: 0,
                kind: ResultKind::Command,
                name: meta.name.to_string(),
                path: format!("command://{}", meta.name),
                score_breakdown: ScoreBreakdown::default(),
            };
            self.apply_rank_boost(&mut item);
            out.push(item);
        }
        Ok(())
    }

    /// PRODUCT.1.D — discover runnable commands from the active workspace's
    /// manifests (package.json / Cargo.toml / Makefile / justfile) and surface
    /// them as copy-only rows. No project root ⇒ no rows. Copy-only: the primary
    /// action is intercepted on the frontend (via the `projectcmd://` path) to
    /// write the command to the clipboard; execution is PRODUCT.1.E.
    fn append_project_command_results(
        &self,
        query: &str,
        limit: usize,
        session: &ActionSession,
        out: &mut Vec<UiSearchItem>,
    ) -> Result<(), String> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return Ok(());
        }
        let Some(root) = self
            .workspace_manager
            .lock()
            .ok()
            .and_then(|ws| ws.current().project_root.clone())
            .filter(|root| !root.trim().is_empty())
        else {
            return Ok(());
        };
        let root_path = std::path::Path::new(&root);
        let cwd = root_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(root.as_str())
            .to_string();

        for cmd in crate::core::project_commands::discover(root_path)
            .into_iter()
            .filter(|cmd| cmd.command.to_lowercase().contains(&q) || cmd.intent.contains(&q))
            .take(limit)
        {
            // Benign action: the frontend short-circuits `projectcmd://` rows to a
            // clipboard copy, so this never dispatches in the happy path.
            let action = Action::command_route(
                format!("projectcmd:{}", cmd.command),
                format!("Copy {}", cmd.command),
                "search.record_selection",
                json!({ "source": "command", "path": format!("projectcmd://{}", cmd.command) }),
            );
            let action_ref = self.action_arena.insert(session, action)?;
            let mut item = UiSearchItem {
                item_ref: action_ref.clone(),
                title: cmd.command.clone(),
                subtitle: format!("{} · {}", cmd.source_file, cwd),
                source: "command".into(),
                score: 72,
                icon_key: Some("command".into()),
                primary_action: action_ref,
                primary_action_label: "Copy".into(),
                secondary_action_count: 0,
                kind: ResultKind::Command,
                name: cmd.command.clone(),
                path: format!("projectcmd://{}:{}", cmd.source_file, cmd.command),
                score_breakdown: ScoreBreakdown::default(),
            };
            self.apply_rank_boost(&mut item);
            out.push(item);
        }
        Ok(())
    }

    fn append_note_results(
        &self,
        query: &str,
        limit: usize,
        session: &ActionSession,
        out: &mut Vec<UiSearchItem>,
    ) -> Result<(), String> {
        let q = query.to_lowercase();
        let notes = self.note_manager.lock().map_err(|e| e.to_string())?.list();
        for note in notes
            .into_iter()
            .filter(|note| note.name.to_lowercase().contains(&q))
            .take(limit)
        {
            let action = Action::open_panel(
                format!("note:{}", note.name),
                "Open note",
                "note",
                note.name.clone(),
            );
            let action_ref = self.action_arena.insert(session, action)?;
            let mut item = UiSearchItem {
                item_ref: action_ref.clone(),
                title: note.name.clone(),
                subtitle: format!("{} bytes", note.size_bytes),
                source: "note".into(),
                score: 75,
                icon_key: Some("note".into()),
                primary_action: action_ref,
                primary_action_label: "Open note".into(),
                secondary_action_count: 0,
                kind: ResultKind::Note,
                name: note.name.clone(),
                path: format!("note://{}", note.name),
                score_breakdown: ScoreBreakdown::default(),
            };
            self.apply_rank_boost(&mut item);
            out.push(item);
        }
        Ok(())
    }

    fn append_history_results(
        &self,
        query: &str,
        limit: usize,
        session: &ActionSession,
        out: &mut Vec<UiSearchItem>,
    ) -> Result<(), String> {
        let workspace_id = self
            .workspace_manager
            .lock()
            .ok()
            .map(|workspace| workspace.current().id);
        let history = self.history_manager.lock().map_err(|e| e.to_string())?;
        for entry in history
            .search_ranked(query, workspace_id)
            .into_iter()
            .take(limit)
        {
            let action = Action::open_panel(
                format!("history:{}", entry.id),
                "Open history",
                "history",
                entry.id.clone(),
            );
            let action_ref = self.action_arena.insert(session, action)?;
            let snippet = entry.content.chars().take(80).collect::<String>();
            let mut score = if entry.pinned { 70 } else { 65 };
            if workspace_id.is_some() && entry.workspace_id == workspace_id {
                score += 10;
            }
            let mut item = UiSearchItem {
                item_ref: action_ref.clone(),
                title: snippet.clone(),
                subtitle: entry.content_type.clone(),
                source: "history".into(),
                score,
                icon_key: Some("history".into()),
                primary_action: action_ref,
                primary_action_label: "Open history".into(),
                secondary_action_count: 0,
                kind: ResultKind::History,
                name: snippet,
                path: format!("history://{}", entry.id),
                score_breakdown: ScoreBreakdown::default(),
            };
            self.apply_rank_boost(&mut item);
            out.push(item);
        }
        Ok(())
    }

    /// MEM.1.C — surface stored personal memories (scope=`personal`) as search
    /// rows. Gated by `features.ai` at the call site. Ranking reuses the
    /// capability layer's `term_score` so a memory ranks here the same way it
    /// would under the `recall` card. Best-effort: store errors yield no rows.
    fn append_memory_results(
        &self,
        query: &str,
        limit: usize,
        session: &ActionSession,
        out: &mut Vec<UiSearchItem>,
    ) -> Result<(), String> {
        // Personal memories are stored workspace-agnostic (`workspace_id = None`),
        // mirroring `recall` and the grounding reader.
        let rows = self
            .knowledge_store
            .agent_memories_blocking(Some(PERSONAL_MEMORY_SCOPE.to_string()), None, 50)
            .unwrap_or_default();

        let q = query.trim().to_lowercase();
        let mut scored: Vec<(f32, crate::core::knowledge_store::AgentMemoryEntry)> = rows
            .into_iter()
            .map(|row| (term_score(&q, &row.title, &row.content), row))
            .collect();
        if !q.is_empty() {
            scored.retain(|(s, _)| *s > 0.0);
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        }

        for (_, row) in scored.into_iter().take(limit) {
            // One-line content carried in `subtitle` for both display and the
            // frontend "paste into query" action (memory rows are short).
            let content_line = row.content.replace('\n', " ");
            // The frontend short-circuits memory rows to paste `subtitle` into the
            // query; this benign route only runs if that path ever regresses.
            let action = Action::command_route(
                format!("memory:{}", row.id),
                "Paste",
                "search.record_selection",
                json!({ "source": "memory", "path": format!("memory://{}", row.id) }),
            );
            let action_ref = self.action_arena.insert(session, action)?;
            let mut item = UiSearchItem {
                item_ref: action_ref.clone(),
                title: row.title.clone(),
                subtitle: content_line,
                source: "memory".into(),
                score: 68,
                icon_key: Some("memory".into()),
                primary_action: action_ref,
                primary_action_label: "Paste".into(),
                secondary_action_count: 0,
                kind: ResultKind::Memory,
                name: row.title,
                path: format!("memory://{}", row.id),
                score_breakdown: ScoreBreakdown::default(),
            };
            self.apply_rank_boost(&mut item);
            out.push(item);
        }
        Ok(())
    }

    fn append_model_results(
        &self,
        query: &str,
        limit: usize,
        session: &ActionSession,
        out: &mut Vec<UiSearchItem>,
    ) -> Result<(), String> {
        let q = query.to_lowercase();
        let hardware = HardwareInfo {
            ram_mb: 0,
            vram_mb: 0,
        };
        for model in self
            .model_manager
            .catalog_fast(&hardware)
            .into_iter()
            .filter(|model| model.name.to_lowercase().contains(&q))
            .take(limit)
        {
            let action = Action::open_panel(
                format!("model:{}", model.name),
                "Open model",
                "model",
                model.name.clone(),
            );
            let action_ref = self.action_arena.insert(session, action)?;
            let mut item = UiSearchItem {
                item_ref: action_ref.clone(),
                title: model.name.clone(),
                subtitle: model.rating.clone(),
                source: "model".into(),
                score: 60,
                icon_key: Some("model".into()),
                primary_action: action_ref,
                primary_action_label: "Manage model".into(),
                secondary_action_count: 0,
                kind: ResultKind::Model,
                name: model.name.clone(),
                path: format!("model://{}", model.name),
                score_breakdown: ScoreBreakdown::default(),
            };
            self.apply_rank_boost(&mut item);
            out.push(item);
        }
        Ok(())
    }
}

fn parse_direct_utility_query(query: &str) -> Option<(&'static str, String)> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let head = parts.next()?;
    let args = parts.next().unwrap_or("").trim().to_string();
    direct_utility_name(&head.to_lowercase()).map(|name| (name, args))
}

fn direct_utility_name(name: &str) -> Option<&'static str> {
    match name {
        "uuid" => Some("uuid"),
        "nanoid" => Some("nanoid"),
        "pw" | "password" => Some("pw"),
        "hash" => Some("hash"),
        "b64enc" | "base64" | "base64enc" => Some("b64enc"),
        "b64dec" | "base64dec" => Some("b64dec"),
        "urlenc" | "urlencode" => Some("urlenc"),
        "urldec" | "urldecode" => Some("urldec"),
        "json" => Some("json"),
        "jsonm" | "jsonmin" | "jsonminify" => Some("jsonm"),
        "regex" => Some("regex"),
        "jwt" => Some("jwt"),
        "color" => Some("color"),
        "cron" => Some("cron"),
        "killport" => Some("killport"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{direct_utility_name, parse_direct_utility_query};

    #[test]
    fn direct_utility_query_accepts_slashless_args() {
        assert_eq!(
            parse_direct_utility_query(r#"json {"a":1}"#),
            Some(("json", r#"{"a":1}"#.to_string()))
        );
        assert_eq!(
            parse_direct_utility_query("jwt header.payload.sig"),
            Some(("jwt", "header.payload.sig".to_string()))
        );
    }

    #[test]
    fn direct_utility_query_supports_common_aliases() {
        assert_eq!(direct_utility_name("base64"), Some("b64enc"));
        assert_eq!(direct_utility_name("password"), Some("pw"));
        assert_eq!(direct_utility_name("jsonminify"), Some("jsonm"));
    }

    #[test]
    fn direct_utility_query_ignores_regular_search_text() {
        assert_eq!(parse_direct_utility_query("readme"), None);
        assert_eq!(parse_direct_utility_query("terminal"), None);
    }
}
