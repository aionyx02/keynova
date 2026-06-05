//! PRODUCT.1.D project command discovery (copy-only MVP).
//!
//! Reads workspace manifests and surfaces runnable commands as search rows whose
//! primary action is "copy the command string". fs reads only, best-effort: a
//! manifest that is missing or fails to parse is simply skipped. Execution /
//! terminal handoff and the high-risk confirmation gate are PRODUCT.1.E — this
//! module never runs anything.

use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectCommand {
    /// Ready-to-run command string, e.g. `npm run dev` / `cargo test`.
    pub command: String,
    /// Canonical intent (dev/test/build/lint/format/check/run/preview) or the raw name.
    pub intent: String,
    /// Manifest the command came from, e.g. `package.json`.
    pub source_file: String,
    /// True for state-changing/destructive names (deploy/publish/clean/…). Carried
    /// for the risk label + PRODUCT.1.E execution gating; copy itself is safe.
    pub risky: bool,
}

/// Destructive / state-changing name fragments. Conservative: only clear ones.
const DESTRUCTIVE: &[&str] = &[
    "deploy", "publish", "release", "clean", "prune", "reset", "drop", "migrate",
    "destroy", "force", "delete", "remove",
];

/// Map a script/target name to a canonical workflow intent, else the raw name.
pub fn normalize_intent(name: &str) -> String {
    let n = name.to_lowercase();
    const MAP: &[(&str, &str)] = &[
        ("clippy", "lint"),
        ("format", "format"),
        ("fmt", "format"),
        ("preview", "preview"),
        ("serve", "dev"),
        ("start", "dev"),
        ("dev", "dev"),
        ("test", "test"),
        ("build", "build"),
        ("lint", "lint"),
        ("check", "check"),
        ("run", "run"),
    ];
    for (needle, intent) in MAP {
        if &n == needle || n.contains(needle) {
            return (*intent).to_string();
        }
    }
    n
}

fn is_risky(name: &str) -> bool {
    let n = name.to_lowercase();
    DESTRUCTIVE.iter().any(|kw| n.contains(kw))
}

/// Discover runnable commands from the manifests in `root` (non-recursive).
pub fn discover(root: &Path) -> Vec<ProjectCommand> {
    let mut out = Vec::new();
    discover_package_json(root, &mut out);
    discover_cargo(root, &mut out);
    discover_makefile(root, &mut out);
    discover_justfile(root, &mut out);
    out
}

fn discover_package_json(root: &Path, out: &mut Vec<ProjectCommand>) {
    let Ok(text) = std::fs::read_to_string(root.join("package.json")) else {
        return;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return;
    };
    let Some(scripts) = json.get("scripts").and_then(|s| s.as_object()) else {
        return;
    };
    for name in scripts.keys() {
        out.push(ProjectCommand {
            command: format!("npm run {name}"),
            intent: normalize_intent(name),
            source_file: "package.json".into(),
            risky: is_risky(name),
        });
    }
}

fn discover_cargo(root: &Path, out: &mut Vec<ProjectCommand>) {
    if !root.join("Cargo.toml").is_file() {
        return;
    }
    // Presence-based: surface the standard cargo workflow intents. (Parsing
    // [[bin]]/aliases is a follow-up; these cover the daily paths.)
    for (command, intent) in [
        ("cargo build", "build"),
        ("cargo test", "test"),
        ("cargo run", "run"),
        ("cargo clippy", "lint"),
        ("cargo fmt", "format"),
    ] {
        out.push(ProjectCommand {
            command: command.into(),
            intent: intent.into(),
            source_file: "Cargo.toml".into(),
            risky: false,
        });
    }
}

fn discover_makefile(root: &Path, out: &mut Vec<ProjectCommand>) {
    let Ok(text) = std::fs::read_to_string(root.join("Makefile")) else {
        return;
    };
    for target in parse_make_targets(&text) {
        out.push(ProjectCommand {
            command: format!("make {target}"),
            intent: normalize_intent(&target),
            source_file: "Makefile".into(),
            risky: is_risky(&target),
        });
    }
}

/// `target:` at column 0, identifier chars only — skips recipe lines (indented),
/// pattern rules, and special targets (`.PHONY`, `$(VAR)`).
fn parse_make_targets(text: &str) -> Vec<String> {
    let mut targets = Vec::new();
    for line in text.lines() {
        if line.starts_with([' ', '\t']) {
            continue;
        }
        let Some(colon) = line.find(':') else {
            continue;
        };
        let name = line[..colon].trim();
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            continue;
        }
        if !targets.iter().any(|t| t == name) {
            targets.push(name.to_string());
        }
    }
    targets
}

fn discover_justfile(root: &Path, out: &mut Vec<ProjectCommand>) {
    let Some(path) = ["justfile", "Justfile", ".justfile"]
        .iter()
        .map(|name| root.join(name))
        .find(|p| p.is_file())
    else {
        return;
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    for recipe in parse_just_recipes(&text) {
        out.push(ProjectCommand {
            command: format!("just {recipe}"),
            intent: normalize_intent(&recipe),
            source_file: "justfile".into(),
            risky: is_risky(&recipe),
        });
    }
}

/// Recipe lines `name args:` at column 0 — skips comments, `@`-prefixed lines,
/// and `name := value` assignments.
fn parse_just_recipes(text: &str) -> Vec<String> {
    let mut recipes = Vec::new();
    for line in text.lines() {
        if line.starts_with([' ', '\t', '#', '@']) {
            continue;
        }
        if !line.contains(':') {
            continue;
        }
        let name: String = line
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        if name.is_empty() {
            continue;
        }
        // `name := value` is an assignment, not a recipe.
        if line[name.len()..].trim_start().starts_with(":=") {
            continue;
        }
        if !recipes.iter().any(|r| r == &name) {
            recipes.push(name);
        }
    }
    recipes
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;

    fn temp_root() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("keynova-projcmd-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create temp root");
        dir
    }

    #[test]
    fn normalize_and_risk() {
        assert_eq!(normalize_intent("clippy"), "lint");
        assert_eq!(normalize_intent("test:watch"), "test");
        assert_eq!(normalize_intent("whatever"), "whatever");
        assert!(is_risky("deploy:prod"));
        assert!(!is_risky("test"));
    }

    #[test]
    fn discovers_package_json_scripts() {
        let root = temp_root();
        fs::write(
            root.join("package.json"),
            r#"{ "scripts": { "dev": "vite", "build": "vite build", "deploy": "x" } }"#,
        )
        .unwrap();
        let cmds = discover(&root);
        assert!(cmds.iter().any(|c| c.command == "npm run dev" && c.intent == "dev"));
        assert!(cmds.iter().any(|c| c.command == "npm run build" && c.source_file == "package.json"));
        assert!(cmds.iter().any(|c| c.command == "npm run deploy" && c.risky));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn discovers_cargo_makefile_and_just() {
        let root = temp_root();
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        fs::write(root.join("Makefile"), "build:\n\tcc\n.PHONY: all\n  indented:\n").unwrap();
        fs::write(root.join("justfile"), "alias := 'x'\nrun args:\n  echo\n").unwrap();
        let cmds = discover(&root);
        assert!(cmds.iter().any(|c| c.command == "cargo test" && c.source_file == "Cargo.toml"));
        assert!(cmds.iter().any(|c| c.command == "make build"));
        // `.PHONY` (starts with '.') and the indented line are not targets.
        assert!(!cmds.iter().any(|c| c.command.contains("PHONY") || c.command.contains("indented")));
        assert!(cmds.iter().any(|c| c.command == "just run"));
        // `alias :=` is an assignment, not a recipe.
        assert!(!cmds.iter().any(|c| c.command == "just alias"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn missing_manifests_yield_nothing() {
        let root = temp_root();
        assert!(discover(&root).is_empty());
        let _ = fs::remove_dir_all(root);
    }
}
