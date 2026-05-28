//! Agent lifecycle methods for `AgentHandler`.
//!
//! Owns: `start_run` (router), `should_use_react_loop`, `start_react_run`
//! (LLM-driven), `start_heuristic_run` (offline fallback), `approve_run`,
//! `reject_run`, `memory_refs`. Split out of `handlers/agent/mod.rs` in REF.3
//! so lifecycle stays separate from tool dispatch / local context / dev
//! command running / intent answer helpers.

use std::sync::Arc;

use serde_json::json;

use crate::core::agent_runtime::ReactLoopConfig;
use crate::core::AgentMemoryEntry;
use crate::managers::ai_manager::{
    provider_supports_tool_calls, resolve_ai_runtime_config, ToolCallProvider,
};
use crate::models::agent::{
    AgentMemoryRef, AgentMemoryScope, AgentRun, AgentRunStatus, AgentStep, ContextVisibility,
};

use super::formatting::{
    build_plan, build_prompt_audit, describe_execution, describe_run, parse_visibility, truncate,
};
use super::safety::long_term_memory_opt_in;
use super::{AgentHandler, LONG_TERM_MEMORY_LIMIT, PROMPT_BUDGET_CHARS, SESSION_MEMORY_LIMIT};

impl AgentHandler {
    pub(super) fn start_run(&self, prompt: String) -> Result<AgentRun, String> {
        if self.should_use_react_loop() {
            self.start_react_run(prompt)
        } else {
            self.start_heuristic_run(prompt)
        }
    }

    /// Returns true when the current AI provider supports function/tool calling
    /// and `agent.mode` has not been explicitly set to `"offline"`.
    fn should_use_react_loop(&self) -> bool {
        let Ok(config) = self.config.lock() else {
            return false;
        };
        if config.get("agent.mode").as_deref() == Some("offline") {
            return false;
        }
        match resolve_ai_runtime_config(|key| config.get(key)) {
            Ok(rt) => provider_supports_tool_calls(&rt.provider),
            Err(_) => false,
        }
    }

    /// Normal mode: insert a Running run and spawn the ReAct loop.
    /// The LLM drives tool selection; local heuristics are not involved.
    fn start_react_run(&self, prompt: String) -> Result<AgentRun, String> {
        let run_id = self.runtime.next_run_id();
        let memory_refs = self.memory_refs()?;
        // Pre-populate prompt_audit and context_bundle with initial local context so
        // the UI can show which sources were considered before the first tool call.
        let (initial_sources, _) = self.sources_for_prompt(&prompt).unwrap_or_default();
        let prompt_audit = build_prompt_audit(&prompt, &initial_sources, PROMPT_BUDGET_CHARS);
        let context_bundle = self.build_context_bundle(&prompt, initial_sources.clone());
        let run = AgentRun {
            id: run_id.clone(),
            prompt: prompt.clone(),
            status: AgentRunStatus::Running,
            plan: vec![
                "Classify context by visibility.".into(),
                "LLM selects tools via ReAct loop.".into(),
                "Return grounded final answer.".into(),
            ],
            steps: vec![AgentStep {
                id: format!("{run_id}:react"),
                title: "ReAct loop".into(),
                status: "running".into(),
                tool_calls: Vec::new(),
            }],
            approvals: Vec::new(),
            memory_refs,
            sources: initial_sources,
            prompt_audit: Some(prompt_audit),
            context_bundle: Some(context_bundle),
            command_result: None,
            output: None,
            error: None,
        };
        self.log_audit(
            &run_id,
            "run_started",
            "ok",
            "ReAct loop initiated",
            Some(json!({ "prompt_chars": prompt.chars().count() })),
        );
        let rt_config = {
            let config = self.config.lock().map_err(|e| e.to_string())?;
            resolve_ai_runtime_config(|key| config.get(key))?
        };
        let provider: Arc<dyn ToolCallProvider> = Arc::new(rt_config.provider);
        let tools = self.runtime.list_tools();
        let dispatch = self.build_react_dispatch();
        let knowledge_store = self.knowledge_store.clone();
        let approval_timeout_secs = {
            let cfg = self.config.lock().map_err(|e| e.to_string())?;
            cfg.get("agent.approval_timeout_secs")
                .and_then(|v| v.parse::<u64>().ok())
                .filter(|n| *n > 0)
                .unwrap_or(300)
        };
        let loop_config = ReactLoopConfig {
            approval_timeout_secs,
            audit_log: Some(Arc::new(move |entry| {
                knowledge_store.try_log_agent_audit(entry);
            })),
            ..ReactLoopConfig::default()
        };
        let inserted = self.runtime.insert_run(run)?;
        self.runtime
            .spawn_react_loop(run_id, provider, tools, loop_config, dispatch);
        Ok(inserted)
    }

    /// Offline fallback: resolve sources and planned actions with local heuristics.
    /// Used when the provider does not support tool calls or `agent.mode = "offline"`.
    fn start_heuristic_run(&self, prompt: String) -> Result<AgentRun, String> {
        let (sources, tool_calls) = self.sources_for_prompt(&prompt)?;
        let memory_refs = self.memory_refs()?;
        let prompt_audit = build_prompt_audit(&prompt, &sources, PROMPT_BUDGET_CHARS);
        let context_bundle = self.build_context_bundle(&prompt, sources.clone());
        let approvals = self.plan_approvals(&prompt)?;
        let direct_answer = self.direct_local_answer(&prompt);
        let status = if approvals.is_empty() {
            AgentRunStatus::Completed
        } else {
            AgentRunStatus::WaitingApproval
        };
        let plan = build_plan(
            &prompt,
            approvals
                .first()
                .and_then(|approval| approval.planned_action.as_ref()),
            direct_answer.is_some(),
        );
        let run_id = self.runtime.next_run_id();
        let run = AgentRun {
            id: run_id.clone(),
            prompt: prompt.clone(),
            status,
            plan,
            steps: vec![
                AgentStep {
                    id: format!("{run_id}:prompt"),
                    title: "Build filtered prompt".into(),
                    status: "completed".into(),
                    tool_calls,
                },
                AgentStep {
                    id: format!("{run_id}:approval"),
                    title: if approvals.is_empty() {
                        "No approval required".into()
                    } else {
                        "Waiting for approval".into()
                    },
                    status: if approvals.is_empty() {
                        "completed".into()
                    } else {
                        "pending".into()
                    },
                    tool_calls: Vec::new(),
                },
            ],
            approvals,
            memory_refs,
            sources,
            prompt_audit: Some(prompt_audit.clone()),
            context_bundle: Some(context_bundle),
            command_result: None,
            output: Some(direct_answer.unwrap_or_else(|| describe_run(&prompt, &prompt_audit))),
            error: None,
        };

        self.log_audit(
            &run_id,
            "run_started",
            "ok",
            "Agent run prepared",
            Some(json!({
                "prompt_chars": prompt.chars().count(),
                "included_sources": prompt_audit.included_sources.len(),
                "filtered_sources": prompt_audit.filtered_sources.len(),
                "approval_count": run.approvals.len(),
            })),
        );
        if let Some(approval) = run.approvals.first() {
            self.log_audit(
                &run_id,
                "approval_required",
                "pending",
                &approval.summary,
                approval.planned_action.as_ref().map(|action| {
                    json!({
                        "action_id": action.id,
                        "kind": action.kind,
                        "risk": action.risk,
                    })
                }),
            );
        }
        self.runtime.insert_run(run)
    }

    pub(super) fn approve_run(
        &self,
        run_id: &str,
        approval_id: &str,
        remember: bool,
    ) -> Result<AgentRun, String> {
        let mut run = self
            .runtime
            .get(run_id)?
            .ok_or_else(|| format!("agent run '{run_id}' not found"))?;
        let approval_index = run
            .approvals
            .iter()
            .position(|approval| approval.id == approval_id)
            .ok_or_else(|| format!("approval '{approval_id}' not found"))?;
        if run.approvals[approval_index].status != "pending" {
            return Err(format!("approval '{approval_id}' is not pending"));
        }
        run.approvals[approval_index].remember_for_run = remember;
        match run.approvals[approval_index].planned_action.clone() {
            None => {
                // ReAct gate approval — mark approved, restore Running; loop resumes.
                run.approvals[approval_index].status = "approved".into();
                run.status = AgentRunStatus::Running;
                self.log_audit(
                    run_id,
                    "approval_approved",
                    "ok",
                    &run.approvals[approval_index].summary,
                    None,
                );
                self.runtime.update_run(run, "agent.run.updated")
            }
            Some(action) => {
                // Heuristic flow — execute planned action and complete the run.
                let command_result = self.execute_planned_action(&action)?;
                run.approvals[approval_index].status = "approved".into();
                run.status = AgentRunStatus::Completed;
                run.command_result = Some(command_result.clone());
                run.output = Some(describe_execution(&action, &command_result));
                if let Some(step) = run.steps.get_mut(1) {
                    step.status = "completed".into();
                    step.title = format!("Approved: {}", action.label);
                }
                self.log_audit(
                    run_id,
                    "approval_approved",
                    "ok",
                    &action.summary,
                    Some(json!({
                        "action_id": action.id,
                        "kind": action.kind,
                        "risk": action.risk,
                    })),
                );
                if long_term_memory_opt_in(&self.config) {
                    let workspace_id = self.workspace_manager.lock().ok().map(|ws| ws.current().id);
                    self.knowledge_store
                        .try_store_agent_memory(AgentMemoryEntry {
                            id: format!("run:{run_id}"),
                            scope: "long_term".into(),
                            workspace_id,
                            title: truncate(&run.prompt, 80),
                            content: run.output.clone().unwrap_or_default(),
                            visibility: "user_private".into(),
                        });
                }
                self.runtime.update_run(run, "agent.run.completed")
            }
        }
    }

    pub(super) fn reject_run(&self, run_id: &str, approval_id: &str) -> Result<AgentRun, String> {
        let mut run = self
            .runtime
            .get(run_id)?
            .ok_or_else(|| format!("agent run '{run_id}' not found"))?;
        let approval_index = run
            .approvals
            .iter()
            .position(|approval| approval.id == approval_id)
            .ok_or_else(|| format!("approval '{approval_id}' not found"))?;
        run.approvals[approval_index].status = "rejected".into();
        let summary = run.approvals[approval_index].summary.clone();
        if run.approvals[approval_index].planned_action.is_none() {
            // ReAct gate rejection — mark rejected, restore Running; loop continues.
            run.status = AgentRunStatus::Running;
            self.log_audit(run_id, "approval_rejected", "cancelled", &summary, None);
            return self.runtime.update_run(run, "agent.run.updated");
        }
        // Heuristic flow — cancel the run.
        run.status = AgentRunStatus::Cancelled;
        run.command_result = None;
        run.output = Some(format!("Approval rejected. {summary}"));
        if let Some(step) = run.steps.get_mut(1) {
            step.status = "cancelled".into();
            step.title = "Approval rejected".into();
        }
        self.log_audit(run_id, "approval_rejected", "cancelled", &summary, None);
        self.runtime.update_run(run, "agent.run.failed")
    }

    fn memory_refs(&self) -> Result<Vec<AgentMemoryRef>, String> {
        let mut refs = Vec::new();
        for run in self.runtime.recent_runs(SESSION_MEMORY_LIMIT)? {
            if run.output.as_deref().unwrap_or("").is_empty() {
                continue;
            }
            refs.push(AgentMemoryRef {
                id: format!("session:{}", run.id),
                scope: AgentMemoryScope::Session,
                visibility: ContextVisibility::UserPrivate,
                summary: truncate(
                    &format!(
                        "Recent run: {} -> {}",
                        run.prompt,
                        run.output.unwrap_or_default()
                    ),
                    140,
                ),
            });
        }

        if let Ok(workspace) = self.workspace_manager.lock() {
            let current = workspace.current();
            refs.push(AgentMemoryRef {
                id: format!("workspace:{}", current.id),
                scope: AgentMemoryScope::Workspace,
                visibility: ContextVisibility::PublicContext,
                summary: format!(
                    "Workspace {} with {} recent files, {} notes, and {} recent actions.",
                    current.name,
                    current.recent_files.len(),
                    current.note_ids.len(),
                    current.recent_actions.len()
                ),
            });
            if long_term_memory_opt_in(&self.config) {
                for memory in self.knowledge_store.agent_memories_blocking(
                    Some("long_term".into()),
                    Some(current.id),
                    LONG_TERM_MEMORY_LIMIT,
                )? {
                    refs.push(AgentMemoryRef {
                        id: memory.id,
                        scope: AgentMemoryScope::LongTerm,
                        visibility: parse_visibility(&memory.visibility),
                        summary: truncate(&format!("{}: {}", memory.title, memory.content), 140),
                    });
                }
            }
        }

        Ok(refs)
    }
}
