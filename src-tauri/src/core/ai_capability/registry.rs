//! Compile-time capability registry. Initial three capabilities per
//! ADR-0029 §4 / REF.4 scope. New entries are append-only here; runtime
//! `dyn Capability` registration is intentionally absent.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityId {
    Explain,
    Summarize,
    FixError,
    GenCommand,
    SuggestNext,
    Remember,
    Recall,
}

impl CapabilityId {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "explain" => Some(Self::Explain),
            "summarize" => Some(Self::Summarize),
            "fix_error" => Some(Self::FixError),
            "gen_command" => Some(Self::GenCommand),
            "suggest_next" => Some(Self::SuggestNext),
            "remember" => Some(Self::Remember),
            "recall" => Some(Self::Recall),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Explain => "explain",
            Self::Summarize => "summarize",
            Self::FixError => "fix_error",
            Self::GenCommand => "gen_command",
            Self::SuggestNext => "suggest_next",
            Self::Remember => "remember",
            Self::Recall => "recall",
        }
    }
}

/// Registration-time metadata. `audit` and `accepts_context_hash` are fixed
/// per capability per ADR-0030 §4 — not part of the per-call return.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct CapabilityMeta {
    pub id: CapabilityId,
    pub audit: bool,
    pub accepts_context_hash: bool,
}

const META: &[CapabilityMeta] = &[
    CapabilityMeta {
        id: CapabilityId::Explain,
        audit: true,
        accepts_context_hash: true,
    },
    CapabilityMeta {
        id: CapabilityId::Summarize,
        audit: false,
        accepts_context_hash: false,
    },
    CapabilityMeta {
        id: CapabilityId::FixError,
        audit: true,
        accepts_context_hash: true,
    },
    CapabilityMeta {
        id: CapabilityId::GenCommand,
        audit: true,
        accepts_context_hash: true,
    },
    CapabilityMeta {
        id: CapabilityId::SuggestNext,
        audit: false,
        accepts_context_hash: true,
    },
    CapabilityMeta {
        id: CapabilityId::Remember,
        audit: true,
        accepts_context_hash: false,
    },
    CapabilityMeta {
        id: CapabilityId::Recall,
        audit: false,
        accepts_context_hash: false,
    },
];

pub fn meta(id: CapabilityId) -> CapabilityMeta {
    META.iter()
        .copied()
        .find(|m| m.id == id)
        .expect("CapabilityId variants must have registry entries")
}

pub fn all() -> &'static [CapabilityMeta] {
    META
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_roundtrip() {
        for id in [
            CapabilityId::Explain,
            CapabilityId::Summarize,
            CapabilityId::FixError,
            CapabilityId::GenCommand,
            CapabilityId::SuggestNext,
            CapabilityId::Remember,
            CapabilityId::Recall,
        ] {
            assert_eq!(CapabilityId::parse(id.as_str()), Some(id));
        }
        assert_eq!(CapabilityId::parse("nope"), None);
    }

    #[test]
    fn all_covers_every_variant() {
        let ids: Vec<_> = all().iter().map(|m| m.id).collect();
        assert!(ids.contains(&CapabilityId::Explain));
        assert!(ids.contains(&CapabilityId::Summarize));
        assert!(ids.contains(&CapabilityId::FixError));
        assert!(ids.contains(&CapabilityId::GenCommand));
        assert!(ids.contains(&CapabilityId::SuggestNext));
        assert!(ids.contains(&CapabilityId::Remember));
        assert!(ids.contains(&CapabilityId::Recall));
    }

    #[test]
    fn audit_matches_adr_0030() {
        assert!(meta(CapabilityId::Explain).audit);
        assert!(!meta(CapabilityId::Summarize).audit);
        assert!(meta(CapabilityId::FixError).audit);
        assert!(meta(CapabilityId::GenCommand).audit);
        assert!(!meta(CapabilityId::SuggestNext).audit);
        assert!(meta(CapabilityId::Remember).audit);
        assert!(!meta(CapabilityId::Recall).audit);
    }
}
