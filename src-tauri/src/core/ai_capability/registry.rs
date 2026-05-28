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
}

impl CapabilityId {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "explain" => Some(Self::Explain),
            "summarize" => Some(Self::Summarize),
            "fix_error" => Some(Self::FixError),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Explain => "explain",
            Self::Summarize => "summarize",
            Self::FixError => "fix_error",
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
    }

    #[test]
    fn audit_matches_adr_0030() {
        assert!(meta(CapabilityId::Explain).audit);
        assert!(!meta(CapabilityId::Summarize).audit);
        assert!(meta(CapabilityId::FixError).audit);
    }
}
