// REF.1 — Unified result/action contract.
//
// All result producers (search backends, builtin commands, file/action paths) emit
// or convert to `UnifiedResult`. REF.6 will switch `CommandPalette` consumption to
// this shape; until then, legacy types (`SearchResult`, `BuiltinCommandResult`,
// `UiSearchItem`) remain as compatibility surfaces, with conversion shims provided
// here.
//
// `RiskTag` realizes ADR-0030's minimal `{requires_confirmation, reason}` contract
// and is exposed under the REF.1 alias `ConfirmRequirement`.
//
// Allowed dead_code at module level: REF.1 lands the contract; REF.4–REF.6 wire
// the consumers. Remove this allow once REF.6 consumes the types in palette.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use crate::models::action::{ActionRef, ScoreBreakdown, UiSearchItem};
use crate::models::builtin_command::{BuiltinCommandResult, CommandUiType};
use crate::models::search_result::{ResultKind, SearchResult};

/// ADR-0030 minimal risk tag. `reason` is audit-only (UI must not render it).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RiskTag {
    pub requires_confirmation: bool,
    #[serde(default)]
    pub reason: String,
}

/// Maximum audit-only `reason` length, per ADR-0030 §4. Capability authors must
/// truncate at this UTF-8 char boundary before returning.
pub const RISK_TAG_REASON_MAX_BYTES: usize = 256;

impl RiskTag {
    pub fn none() -> Self {
        Self {
            requires_confirmation: false,
            reason: String::new(),
        }
    }

    pub fn confirm(reason: impl Into<String>) -> Self {
        Self {
            requires_confirmation: true,
            reason: truncate_reason(reason.into()),
        }
    }
}

/// Truncate `reason` to `RISK_TAG_REASON_MAX_BYTES` at a UTF-8 char boundary, appending `…`.
/// Capability authors are expected to call this (or equivalent) before constructing a `RiskTag`.
pub fn truncate_reason(s: String) -> String {
    if s.len() <= RISK_TAG_REASON_MAX_BYTES {
        return s;
    }
    // Find the largest char boundary <= MAX - 3 (room for the U+2026 "…", 3 bytes UTF-8).
    let target = RISK_TAG_REASON_MAX_BYTES.saturating_sub(3);
    let mut end = target;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    let mut out = String::with_capacity(end + 3);
    out.push_str(&s[..end]);
    out.push('…');
    out
}

/// REF.1 alias: `ConfirmRequirement` is the consumer-facing name for `RiskTag`.
pub type ConfirmRequirement = RiskTag;

/// Where a result originated.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResultSource {
    /// Filesystem-indexed entries (apps, files, folders, notes, history, models).
    File { kind: ResultKind, path: String },
    /// Builtin command (calculator, navigation, etc.).
    BuiltinCommand { ui: BuiltinUi },
    /// Extension point for sources not yet first-class (workflow memory, plugin, etc.).
    Other { name: String },
}

/// Mirrors `CommandUiType` but flattens `Terminal(spec)` to its tag string so
/// `UnifiedResult` can stay JSON-friendly without dragging in `TerminalLaunchSpec`.
/// REF.6 may revisit if richer payload is needed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BuiltinUi {
    Inline,
    Panel { name: String },
    Terminal,
}

impl From<CommandUiType> for BuiltinUi {
    fn from(value: CommandUiType) -> Self {
        match value {
            CommandUiType::Inline => BuiltinUi::Inline,
            CommandUiType::Panel(name) => BuiltinUi::Panel { name },
            CommandUiType::Terminal(_) => BuiltinUi::Terminal,
        }
    }
}

/// An action exposed inline on a result row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionChip {
    pub id: String,
    pub label: String,
    pub action_ref: ActionRef,
    pub confirm: ConfirmRequirement,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hotkey_hint: Option<String>,
    /// True = visible on the row; false = inside the secondary menu.
    #[serde(default = "default_true")]
    pub primary: bool,
}

fn default_true() -> bool {
    true
}

/// Preview payload, rendered on user expand gesture.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PreviewPayload {
    #[default]
    None,
    Text {
        content: String,
        truncated: bool,
    },
    Image {
        data_url: String,
    },
    Binary {
        size_bytes: u64,
    },
}

/// Ranking signals. `score` is the canonical display score; `breakdown` retains
/// the existing rank decomposition; `workflow_boost` is reserved for workflow
/// memory and is zero until that integration wires it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RankSignals {
    pub score: i64,
    #[serde(default)]
    pub breakdown: ScoreBreakdown,
    #[serde(default)]
    pub workflow_boost: i64,
}

/// Forward-compat metadata bag. New optional fields are added here without
/// breaking older consumers; never put required semantics in `SourceMetadata`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceMetadata {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    /// Preserves the legacy `UiSearchItem.secondary_action_count`
    /// signal ("press Tab to see more") on the palette row without
    /// requiring backend to materialise every hidden action as an
    /// `ActionChip`. Additive per ADR-0030 §4 evolution rule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_action_count: Option<u32>,
}

/// Unified result/action contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedResult {
    /// Stable identifier within a generation (used by React keying, etc.).
    pub id: String,
    pub source: ResultSource,
    pub title: String,
    pub subtitle: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_key: Option<String>,
    pub actions: Vec<ActionChip>,
    #[serde(default)]
    pub preview: PreviewPayload,
    #[serde(default)]
    pub rank: RankSignals,
    /// Optional workflow-memory context hash. Not consumed by initial
    /// capabilities; capability schema treats this as an optional input.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_hash: Option<String>,
    #[serde(default)]
    pub source_metadata: SourceMetadata,
}

// ---------------------------------------------------------------------------
// Conversion shims (REF.1 §Scope)
// ---------------------------------------------------------------------------

/// Convert a legacy raw `SearchResult` (from search backends) into a `UnifiedResult`.
///
/// The legacy struct lacks `ActionRef` / action metadata, so the resulting
/// `UnifiedResult.actions` is empty. Producers that already attach actions
/// should convert via `UiSearchItem` instead.
impl From<SearchResult> for UnifiedResult {
    fn from(r: SearchResult) -> Self {
        let kind = r.kind.clone();
        UnifiedResult {
            id: format!("file:{}", r.path),
            source: ResultSource::File {
                kind,
                path: r.path.clone(),
            },
            title: r.name.clone(),
            subtitle: r.path.clone(),
            icon_key: None,
            actions: Vec::new(),
            preview: PreviewPayload::None,
            rank: RankSignals {
                score: r.score,
                breakdown: ScoreBreakdown::default(),
                workflow_boost: 0,
            },
            context_hash: None,
            source_metadata: SourceMetadata::default(),
        }
    }
}

/// Convert a builtin command result (post-execution) into a `UnifiedResult`.
///
/// Builtin commands produce a UI-typed text payload rather than a search-shaped
/// entry; this shim adapts the produced text into an inline-readable result
/// without inventing actions. Builtin commands that need action chips can
/// emit a `UnifiedResult` directly in REF.6.
impl From<BuiltinCommandResult> for UnifiedResult {
    fn from(r: BuiltinCommandResult) -> Self {
        let ui: BuiltinUi = r.ui_type.into();
        let preview = PreviewPayload::Text {
            content: r.text.clone(),
            truncated: false,
        };
        UnifiedResult {
            id: format!("builtin:{}", short_hash(&r.text)),
            source: ResultSource::BuiltinCommand { ui },
            title: first_line(&r.text),
            subtitle: String::new(),
            icon_key: None,
            actions: Vec::new(),
            preview,
            rank: RankSignals::default(),
            context_hash: None,
            source_metadata: SourceMetadata::default(),
        }
    }
}

/// Convert the current IPC-crossing `UiSearchItem` into a `UnifiedResult`.
/// REF.6.A: this is now the production conversion path used by
/// `SearchHandler` before emitting search responses; legacy `UiSearchItem`
/// stays internal to `base_results_to_ui_items`.
impl From<UiSearchItem> for UnifiedResult {
    fn from(item: UiSearchItem) -> Self {
        let primary_action_chip = ActionChip {
            id: format!("{}:primary", item.item_ref.id),
            label: item.primary_action_label.clone(),
            action_ref: item.primary_action.clone(),
            confirm: ConfirmRequirement::none(),
            hotkey_hint: None,
            primary: true,
        };

        let secondary_action_count = if item.secondary_action_count > 0 {
            Some(item.secondary_action_count as u32)
        } else {
            None
        };

        UnifiedResult {
            id: item.item_ref.id.clone(),
            source: ResultSource::File {
                kind: item.kind.clone(),
                path: item.path.clone(),
            },
            title: item.title.clone(),
            subtitle: item.subtitle.clone(),
            icon_key: item.icon_key.clone(),
            actions: vec![primary_action_chip],
            preview: PreviewPayload::None,
            rank: RankSignals {
                score: item.score,
                breakdown: item.score_breakdown.clone(),
                workflow_boost: 0,
            },
            context_hash: None,
            source_metadata: SourceMetadata {
                secondary_action_count,
                ..SourceMetadata::default()
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").to_string()
}

/// Lightweight deterministic id suffix for non-pathed sources. Uses Rust's
/// default hasher; collision in display ids is acceptable since the id only
/// has to be stable within a single generation, not globally unique.
fn short_hash(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    format!("{:x}", h.finish())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::action::{ActionRef, ScoreBreakdown};
    use crate::models::search_result::ResultKind;

    #[test]
    fn risk_tag_none_is_no_confirm() {
        let t = RiskTag::none();
        assert!(!t.requires_confirmation);
        assert!(t.reason.is_empty());
    }

    #[test]
    fn risk_tag_confirm_truncates_long_reason() {
        let long = "a".repeat(400);
        let t = RiskTag::confirm(long);
        assert!(t.requires_confirmation);
        assert!(t.reason.len() <= RISK_TAG_REASON_MAX_BYTES);
        assert!(t.reason.ends_with('…'));
    }

    #[test]
    fn truncate_reason_respects_char_boundary() {
        // 4-byte char (e.g. 𝄞 U+1D11E) repeated. Each char is 4 bytes UTF-8.
        let s: String = std::iter::repeat('𝄞').take(80).collect();
        assert_eq!(s.len(), 320); // > 256
        let truncated = truncate_reason(s);
        // Must end on a char boundary and within max bytes.
        assert!(truncated.len() <= RISK_TAG_REASON_MAX_BYTES);
        assert!(truncated.ends_with('…'));
        // Last char before the ellipsis must be a full 𝄞, never a partial byte sequence.
        let without_ellipsis = truncated.trim_end_matches('…');
        assert!(without_ellipsis.chars().all(|c| c == '𝄞'));
    }

    #[test]
    fn risk_tag_serde_roundtrip_with_missing_reason() {
        let json = r#"{"requires_confirmation":true}"#;
        let t: RiskTag = serde_json::from_str(json).expect("parse");
        assert!(t.requires_confirmation);
        assert!(t.reason.is_empty()); // serde(default) kicks in
    }

    #[test]
    fn search_result_shim_preserves_score_and_path() {
        let sr = SearchResult {
            kind: ResultKind::File,
            name: "foo.rs".to_string(),
            path: "/tmp/foo.rs".to_string(),
            score: 42,
        };
        let u: UnifiedResult = sr.into();
        assert_eq!(u.title, "foo.rs");
        assert_eq!(u.subtitle, "/tmp/foo.rs");
        assert_eq!(u.rank.score, 42);
        assert!(matches!(
            u.source,
            ResultSource::File { ref path, .. } if path == "/tmp/foo.rs"
        ));
        assert!(u.actions.is_empty());
    }

    #[test]
    fn builtin_command_shim_wraps_text_as_preview() {
        let br = BuiltinCommandResult {
            text: "= 42".to_string(),
            ui_type: CommandUiType::Inline,
        };
        let u: UnifiedResult = br.into();
        assert!(matches!(u.source, ResultSource::BuiltinCommand { .. }));
        match u.preview {
            PreviewPayload::Text { content, .. } => assert_eq!(content, "= 42"),
            _ => panic!("expected Text preview"),
        }
    }

    #[test]
    fn ui_search_item_shim_keeps_primary_action() {
        let item = UiSearchItem {
            item_ref: ActionRef::new("res-1", None, 1),
            title: "Foo App".to_string(),
            subtitle: "C:/Foo.exe".to_string(),
            source: "app".to_string(),
            score: 100,
            icon_key: Some("app/foo".to_string()),
            primary_action: ActionRef::new("act-1", None, 1),
            primary_action_label: "Launch".to_string(),
            secondary_action_count: 0,
            kind: ResultKind::App,
            name: "Foo App".to_string(),
            path: "C:/Foo.exe".to_string(),
            score_breakdown: ScoreBreakdown {
                base: 80,
                workspace_boost: 0,
                config_boost: 0,
                recency_boost: 10,
                frequency_boost: 10,
            },
        };
        let u: UnifiedResult = item.into();
        assert_eq!(u.id, "res-1");
        assert_eq!(u.actions.len(), 1);
        let chip = &u.actions[0];
        assert_eq!(chip.label, "Launch");
        assert_eq!(chip.action_ref.id, "act-1");
        assert!(chip.primary);
        assert!(!chip.confirm.requires_confirmation);
        assert_eq!(u.rank.score, 100);
        assert_eq!(u.rank.breakdown.base, 80);
        // REF.6.A: secondary_action_count == 0 must serialize as None so
        // skip_serializing_if drops the field from the wire format.
        assert_eq!(u.source_metadata.secondary_action_count, None);
    }

    #[test]
    fn ui_search_item_shim_covers_every_source_kind() {
        // PRODUCT.1.B contract audit: every result-source kind must map into a
        // UnifiedResult with stable id, preserved kind/path/score, exactly one
        // primary action chip, and no row-level confirm (risk lives on actions /
        // capabilities, not search rows). One representative per kind.
        let kinds = [
            (ResultKind::App, "app://foo"),
            (ResultKind::File, "/tmp/foo.txt"),
            (ResultKind::Folder, "/tmp/dir"),
            (ResultKind::Command, "command://help"),
            (ResultKind::Note, "note://todo"),
            (ResultKind::History, "history://1"),
            (ResultKind::Model, "model://qwen"),
            (ResultKind::Memory, "memory://1"),
        ];
        for (kind, path) in kinds {
            let item = UiSearchItem {
                item_ref: ActionRef::new("res-k", None, 1),
                title: "T".to_string(),
                subtitle: path.to_string(),
                source: "x".to_string(),
                score: 42,
                icon_key: None,
                primary_action: ActionRef::new("act-k", None, 1),
                primary_action_label: "Go".to_string(),
                secondary_action_count: 0,
                kind: kind.clone(),
                name: "T".to_string(),
                path: path.to_string(),
                score_breakdown: ScoreBreakdown::default(),
            };
            let u: UnifiedResult = item.into();
            assert_eq!(u.id, "res-k", "id stable for {kind:?}");
            assert_eq!(u.rank.score, 42, "score preserved for {kind:?}");
            assert_eq!(u.actions.len(), 1, "one primary action for {kind:?}");
            assert!(u.actions[0].primary);
            assert!(
                !u.actions[0].confirm.requires_confirmation,
                "search rows carry no row-level confirm for {kind:?}"
            );
            match u.source {
                ResultSource::File { kind: k, path: p } => {
                    assert_eq!(k, kind, "kind preserved for {kind:?}");
                    assert_eq!(p, path, "path preserved for {kind:?}");
                }
                other => panic!("expected ResultSource::File for {kind:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn ui_search_item_shim_populates_secondary_action_count() {
        let item = UiSearchItem {
            item_ref: ActionRef::new("res-2", None, 1),
            title: "Foo.txt".to_string(),
            subtitle: "/tmp/foo.txt".to_string(),
            source: "file".to_string(),
            score: 50,
            icon_key: None,
            primary_action: ActionRef::new("act-2", None, 1),
            primary_action_label: "Open".to_string(),
            secondary_action_count: 3,
            kind: ResultKind::File,
            name: "Foo.txt".to_string(),
            path: "/tmp/foo.txt".to_string(),
            score_breakdown: ScoreBreakdown::default(),
        };
        let u: UnifiedResult = item.into();
        assert_eq!(u.source_metadata.secondary_action_count, Some(3));
    }

    #[test]
    fn unified_result_roundtrip_via_json() {
        let item = UiSearchItem {
            item_ref: ActionRef::new("res-2", None, 1),
            title: "Bar".to_string(),
            subtitle: "/tmp/bar".to_string(),
            source: "file".to_string(),
            score: 50,
            icon_key: None,
            primary_action: ActionRef::new("act-2", None, 1),
            primary_action_label: "Open".to_string(),
            secondary_action_count: 0,
            kind: ResultKind::File,
            name: "Bar".to_string(),
            path: "/tmp/bar".to_string(),
            score_breakdown: ScoreBreakdown::default(),
        };
        let u: UnifiedResult = item.into();
        let json = serde_json::to_string(&u).expect("serialize");
        let parsed: UnifiedResult = serde_json::from_str(&json).expect("parse");
        assert_eq!(parsed.id, u.id);
        assert_eq!(parsed.actions.len(), 1);
        assert_eq!(parsed.rank.score, 50);
    }

    #[test]
    fn preview_payload_defaults_to_none_when_absent() {
        let minimal = serde_json::json!({
            "id": "x",
            "source": {"type": "other", "name": "test"},
            "title": "t",
            "subtitle": "s",
            "actions": []
        });
        let parsed: UnifiedResult =
            serde_json::from_value(minimal).expect("parse minimal unified result");
        assert!(matches!(parsed.preview, PreviewPayload::None));
        assert_eq!(parsed.rank.score, 0);
        assert!(parsed.context_hash.is_none());
    }
}
