use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::search_result::ResultKind;

/// A compact reference to an action held by the backend arena.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ActionRef {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub generation: u64,
}

impl ActionRef {
    pub fn new(id: impl Into<String>, session_id: Option<String>, generation: u64) -> Self {
        Self {
            id: id.into(),
            session_id,
            generation,
        }
    }
}

/// Risk level used by agent, automation, and plugin approval gates.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionRisk {
    Low,
    Medium,
    High,
}

/// Backend-only action payload stored in ActionArena instead of crossing IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    pub label: String,
    pub risk: ActionRisk,
    pub kind: ActionKind,
    #[serde(default)]
    pub secondary_actions: Vec<Action>,
}

impl Action {
    pub fn launch_path(path: impl Into<String>) -> Self {
        let path = path.into();
        Self {
            id: format!("launch:{path}"),
            label: "Open".to_string(),
            risk: ActionRisk::Low,
            kind: ActionKind::LaunchPath { path },
            secondary_actions: Vec::new(),
        }
    }

    pub fn command_route(
        id: impl Into<String>,
        label: impl Into<String>,
        route: impl Into<String>,
        payload: Value,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            risk: ActionRisk::Low,
            kind: ActionKind::CommandRoute {
                route: route.into(),
                payload,
            },
            secondary_actions: Vec::new(),
        }
    }

    pub fn open_panel(
        id: impl Into<String>,
        label: impl Into<String>,
        panel: impl Into<String>,
        initial_args: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            risk: ActionRisk::Low,
            kind: ActionKind::OpenPanel {
                panel: panel.into(),
                initial_args: initial_args.into(),
            },
            secondary_actions: Vec::new(),
        }
    }
}

/// Executable backend action payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionKind {
    CommandRoute { route: String, payload: Value },
    LaunchPath { path: String },
    OpenPanel { panel: String, initial_args: String },
    Inline { text: String },
    Noop { reason: String },
}

/// Result returned by action.run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionResult {
    Inline { text: String },
    Panel { name: String, initial_args: String },
    Launched { path: String },
    Noop { reason: String },
}

/// Display-only search item sent over IPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSearchItem {
    pub item_ref: ActionRef,
    pub title: String,
    pub subtitle: String,
    pub source: String,
    pub score: i64,
    pub icon_key: Option<String>,
    pub primary_action: ActionRef,
    pub primary_action_label: String,
    pub secondary_action_count: usize,

    // Backward-compatible fields used by the current React launcher.
    pub kind: ResultKind,
    pub name: String,
    pub path: String,
}
