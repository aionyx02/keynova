//! Non-file result providers: command / note / history / model search sources
//! appended to the result set.
//!
//! Extracted from `handlers/search.rs` (REF.9.E) as a pure structural move;
//! behavior unchanged. These remain inherent methods on `SearchHandler`; only
//! `append_non_file_results` is reachable from the parent module.

use serde_json::json;

use crate::core::action_registry::ActionSession;
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
        self.append_command_results(query, plan.command_limit, session, out)?;
        self.append_note_results(query, plan.note_limit, session, out)?;
        self.append_history_results(query, plan.history_limit, session, out)?;
        self.append_model_results(query, plan.model_limit, session, out)?;
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
        for (meta, score) in registry
            .list()
            .into_iter()
            .filter_map(|meta| {
                let score = command_match_score(meta.name, meta.description, &q)?;
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
