use serde::{Deserialize, Serialize};

/// Classification of a discovered local file or directory.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MaterialClass {
    Project,
    Note,
    Report,
    Presentation,
    Certificate,
    Unknown,
}

impl std::fmt::Display for MaterialClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MaterialClass::Project => write!(f, "Project"),
            MaterialClass::Note => write!(f, "Note"),
            MaterialClass::Report => write!(f, "Report"),
            MaterialClass::Presentation => write!(f, "Presentation"),
            MaterialClass::Certificate => write!(f, "Certificate"),
            MaterialClass::Unknown => write!(f, "Unknown"),
        }
    }
}

/// A single file or directory identified as a learning material candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialCandidate {
    pub path: String,
    pub name: String,
    pub class: MaterialClass,
    pub size_bytes: u64,
    pub modified_secs: u64,
}

/// Aggregate stats from a single scan pass.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanStats {
    /// Total filesystem entries examined.
    pub scanned_count: usize,
    /// Entries that matched a non-Unknown class.
    pub candidate_count: usize,
    /// Entries removed by the denylist.
    pub filtered_count: usize,
    /// Entries rejected due to symlink escape or permission error.
    pub denied_count: usize,
}

/// Output of a learning-material scan over one or more roots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewReport {
    pub roots: Vec<String>,
    pub candidates: Vec<MaterialCandidate>,
    pub stats: ScanStats,
}

impl ReviewReport {
    /// Render the report as a Markdown string suitable for a note or export.
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# Learning Material Review\n\n");
        out.push_str(&format!(
            "**Scanned roots:** {}\n\n",
            if self.roots.is_empty() {
                "(none)".to_string()
            } else {
                self.roots.join(", ")
            }
        ));
        out.push_str(&format!(
            "**Stats:** {} scanned · {} candidates · {} filtered · {} denied\n\n",
            self.stats.scanned_count,
            self.stats.candidate_count,
            self.stats.filtered_count,
            self.stats.denied_count
        ));

        for class in [
            MaterialClass::Project,
            MaterialClass::Note,
            MaterialClass::Report,
            MaterialClass::Presentation,
            MaterialClass::Certificate,
            MaterialClass::Unknown,
        ] {
            let items: Vec<_> = self.candidates.iter().filter(|c| c.class == class).collect();
            if items.is_empty() {
                continue;
            }
            out.push_str(&format!("## {class}\n\n"));
            for item in items {
                let size_kb = item.size_bytes / 1024;
                out.push_str(&format!("- **{}** — `{}` ({} KB)\n", item.name, item.path, size_kb));
            }
            out.push('\n');
        }
        out
    }
}