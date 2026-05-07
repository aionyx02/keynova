use std::path::{Path, PathBuf};
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

use ignore::{WalkBuilder, WalkState};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[allow(dead_code)]
#[serde(rename_all = "snake_case")]
pub enum SystemIndexerProvider {
    Everything,
    Tantivy,
    Mdfind,
    Plocate,
    Locate,
    IgnoreWalk,
    Unavailable,
}

impl SystemIndexerProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Everything => "everything",
            Self::Tantivy => "tantivy",
            Self::Mdfind => "mdfind",
            Self::Plocate => "plocate",
            Self::Locate => "locate",
            Self::IgnoreWalk => "ignore_walk",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemSearchHit {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub score: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemIndexerDiagnostics {
    pub provider: &'static str,
    pub available: bool,
    pub fallback_reason: Option<String>,
    pub visited: usize,
    pub permission_denied: usize,
    pub timed_out: bool,
    pub index_age_secs: Option<u64>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemSearchOutcome {
    pub hits: Vec<SystemSearchHit>,
    pub diagnostics: SystemIndexerDiagnostics,
}

pub trait SystemIndexer {
    #[allow(dead_code)]
    fn provider(&self) -> SystemIndexerProvider;
    #[allow(dead_code)]
    fn status(&self) -> SystemIndexerDiagnostics;
    fn search(&self, query: &str, limit: usize) -> SystemSearchOutcome;
}

pub fn search_system_index(query: &str, roots: &[PathBuf], limit: usize) -> SystemSearchOutcome {
    if query.trim().is_empty() || limit == 0 {
        return unavailable("empty query");
    }

    let native = native_index_search(query, limit);
    if native.diagnostics.available && (!native.hits.is_empty() || roots.is_empty()) {
        return native;
    }

    let fallback_reason = if native.diagnostics.available {
        format!(
            "{} returned no hits; checked bounded fallback roots",
            native.diagnostics.provider
        )
    } else {
        native
            .diagnostics
            .message
            .clone()
            .unwrap_or_else(|| "native indexer unavailable".into())
    };
    let fallback = IgnoreWalkIndexer::new(roots.to_vec()).search(query, limit);
    with_fallback_reason(fallback, fallback_reason)
}

pub struct IgnoreWalkIndexer {
    roots: Vec<PathBuf>,
    max_visited: usize,
    timeout: Duration,
}

impl IgnoreWalkIndexer {
    pub fn new(roots: Vec<PathBuf>) -> Self {
        Self {
            roots,
            max_visited: 40_000,
            timeout: Duration::from_millis(3500),
        }
    }

    #[cfg(test)]
    fn with_limits(roots: Vec<PathBuf>, max_visited: usize, timeout: Duration) -> Self {
        Self {
            roots,
            max_visited,
            timeout,
        }
    }
}

impl SystemIndexer for IgnoreWalkIndexer {
    fn provider(&self) -> SystemIndexerProvider {
        SystemIndexerProvider::IgnoreWalk
    }

    fn status(&self) -> SystemIndexerDiagnostics {
        SystemIndexerDiagnostics {
            provider: self.provider().as_str(),
            available: !self.roots.is_empty(),
            fallback_reason: None,
            visited: 0,
            permission_denied: 0,
            timed_out: false,
            index_age_secs: None,
            message: None,
        }
    }

    fn search(&self, query: &str, limit: usize) -> SystemSearchOutcome {
        if self.roots.is_empty() {
            return unavailable("no fallback roots configured");
        }

        let normalized = query.to_lowercase();
        let limit = limit.max(1);
        let started = Instant::now();
        let hits = Arc::new(Mutex::new(Vec::<SystemSearchHit>::new()));
        let visited = Arc::new(AtomicUsize::new(0));
        let denied = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let timed_out = Arc::new(AtomicBool::new(false));

        for root in &self.roots {
            if stop.load(Ordering::SeqCst) {
                break;
            }
            if !root.exists() {
                continue;
            }

            let mut builder = WalkBuilder::new(root);
            builder
                .standard_filters(true)
                .hidden(true)
                .parents(true)
                .ignore(true)
                .git_ignore(true)
                .git_global(true)
                .git_exclude(true)
                .follow_links(false)
                .threads(4);
            let walker = builder.build_parallel();
            let max_visited = self.max_visited;
            let timeout = self.timeout;

            walker.run(|| {
                let hits = Arc::clone(&hits);
                let visited = Arc::clone(&visited);
                let denied = Arc::clone(&denied);
                let stop = Arc::clone(&stop);
                let timed_out = Arc::clone(&timed_out);
                let normalized = normalized.clone();
                Box::new(move |entry| {
                    if stop.load(Ordering::SeqCst) {
                        return WalkState::Quit;
                    }
                    if started.elapsed() > timeout {
                        timed_out.store(true, Ordering::SeqCst);
                        stop.store(true, Ordering::SeqCst);
                        return WalkState::Quit;
                    }
                    let seen = visited.fetch_add(1, Ordering::SeqCst) + 1;
                    if seen >= max_visited {
                        stop.store(true, Ordering::SeqCst);
                        return WalkState::Quit;
                    }

                    let entry = match entry {
                        Ok(entry) => entry,
                        Err(error) => {
                            if error
                                .io_error()
                                .is_some_and(|io| io.kind() == std::io::ErrorKind::PermissionDenied)
                            {
                                denied.fetch_add(1, Ordering::SeqCst);
                            }
                            return WalkState::Continue;
                        }
                    };
                    let path = entry.path();
                    if !path_matches(path, &normalized) {
                        return WalkState::Continue;
                    }
                    let is_dir = entry.file_type().is_some_and(|kind| kind.is_dir());
                    let name = path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(ToOwned::to_owned)
                        .unwrap_or_else(|| path.to_string_lossy().into_owned());
                    let mut guard = hits.lock().unwrap_or_else(|e| e.into_inner());
                    guard.push(SystemSearchHit {
                        name,
                        path: path.display().to_string(),
                        is_dir,
                        score: 55,
                    });
                    if guard.len() >= limit {
                        stop.store(true, Ordering::SeqCst);
                        return WalkState::Quit;
                    }
                    WalkState::Continue
                })
            });
        }

        let mut hits = hits.lock().unwrap_or_else(|e| e.into_inner()).clone();
        hits.truncate(limit);
        SystemSearchOutcome {
            hits,
            diagnostics: SystemIndexerDiagnostics {
                provider: SystemIndexerProvider::IgnoreWalk.as_str(),
                available: true,
                fallback_reason: None,
                visited: visited.load(Ordering::SeqCst),
                permission_denied: denied.load(Ordering::SeqCst),
                timed_out: timed_out.load(Ordering::SeqCst),
                index_age_secs: None,
                message: None,
            },
        }
    }
}

fn native_index_search(query: &str, limit: usize) -> SystemSearchOutcome {
    #[cfg(target_os = "windows")]
    {
        if crate::platform::windows::check_everything() {
            let hits = crate::platform::windows::everything_search(query, limit as u32)
                .into_iter()
                .map(|(name, path, is_dir)| SystemSearchHit {
                    name,
                    path,
                    is_dir,
                    score: 95,
                })
                .collect();
            return native_outcome(SystemIndexerProvider::Everything, hits, None, None);
        }

        let index_dir = crate::managers::tantivy_index::resolve_index_dir(None);
        let index_count = crate::managers::tantivy_index::indexed_entries(&index_dir);
        if index_count > 0 {
            match crate::managers::tantivy_index::search(&index_dir, query, limit) {
                Ok(results) => {
                    let hits = results
                        .into_iter()
                        .map(|result| SystemSearchHit {
                            name: result.name,
                            path: result.path,
                            is_dir: matches!(
                                result.kind,
                                crate::models::search_result::ResultKind::Folder
                            ),
                            score: result.score,
                        })
                        .collect();
                    return native_outcome(
                        SystemIndexerProvider::Tantivy,
                        hits,
                        None,
                        Some(format!("{index_count} indexed entries")),
                    );
                }
                Err(error) => {
                    return unavailable(format!("tantivy index search failed: {error}"));
                }
            }
        }

        unavailable("Windows native indexers unavailable or empty")
    }

    #[cfg(target_os = "macos")]
    {
        run_cli_indexer(
            SystemIndexerProvider::Mdfind,
            "mdfind",
            &["-name".into(), query.into()],
            limit,
            None,
        )
    }

    #[cfg(target_os = "linux")]
    {
        let age = linux_locate_index_age_secs();
        let plocate = run_cli_indexer(
            SystemIndexerProvider::Plocate,
            "plocate",
            &["--limit".into(), limit.max(1).to_string(), query.into()],
            limit,
            age,
        );
        if plocate.diagnostics.available && !plocate.hits.is_empty() {
            return plocate;
        }
        let locate = run_cli_indexer(
            SystemIndexerProvider::Locate,
            "locate",
            &["-l".into(), limit.max(1).to_string(), query.into()],
            limit,
            age,
        );
        if locate.diagnostics.available {
            locate
        } else {
            plocate
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        unavailable("no native indexer configured for this platform")
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn run_cli_indexer(
    provider: SystemIndexerProvider,
    binary: &str,
    args: &[String],
    limit: usize,
    index_age_secs: Option<u64>,
) -> SystemSearchOutcome {
    let output = run_command_with_timeout(binary, args, Duration::from_millis(2500));
    let output = match output {
        Ok(output) => output,
        Err(error) => {
            return SystemSearchOutcome {
                hits: Vec::new(),
                diagnostics: SystemIndexerDiagnostics {
                    provider: provider.as_str(),
                    available: false,
                    fallback_reason: None,
                    visited: 0,
                    permission_denied: 0,
                    timed_out: error == "timeout",
                    index_age_secs,
                    message: Some(format!("{binary} unavailable or failed: {error}")),
                },
            }
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return SystemSearchOutcome {
            hits: Vec::new(),
            diagnostics: SystemIndexerDiagnostics {
                provider: provider.as_str(),
                available: true,
                fallback_reason: None,
                visited: 0,
                permission_denied: 0,
                timed_out: false,
                index_age_secs,
                message: Some(if stderr.is_empty() {
                    format!("{binary} exited with {}", output.status)
                } else {
                    stderr
                }),
            },
        };
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let hits = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(limit.max(1))
        .map(path_hit)
        .collect::<Vec<_>>();
    native_outcome(provider, hits, index_age_secs, None)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn run_command_with_timeout(
    binary: &str,
    args: &[String],
    timeout: Duration,
) -> Result<std::process::Output, String> {
    let mut child = Command::new(binary)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let started = Instant::now();
    loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(_) => return child.wait_with_output().map_err(|e| e.to_string()),
            None if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("timeout".into());
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn path_hit(path: &str) -> SystemSearchHit {
    let path_buf = PathBuf::from(path);
    let name = path_buf
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string();
    let is_dir = std::fs::metadata(&path_buf)
        .map(|metadata| metadata.is_dir())
        .unwrap_or(false);
    SystemSearchHit {
        name,
        path: path.to_string(),
        is_dir,
        score: 85,
    }
}

fn native_outcome(
    provider: SystemIndexerProvider,
    hits: Vec<SystemSearchHit>,
    index_age_secs: Option<u64>,
    message: Option<String>,
) -> SystemSearchOutcome {
    SystemSearchOutcome {
        hits,
        diagnostics: SystemIndexerDiagnostics {
            provider: provider.as_str(),
            available: true,
            fallback_reason: None,
            visited: 0,
            permission_denied: 0,
            timed_out: false,
            index_age_secs,
            message,
        },
    }
}

fn unavailable(message: impl Into<String>) -> SystemSearchOutcome {
    SystemSearchOutcome {
        hits: Vec::new(),
        diagnostics: SystemIndexerDiagnostics {
            provider: SystemIndexerProvider::Unavailable.as_str(),
            available: false,
            fallback_reason: None,
            visited: 0,
            permission_denied: 0,
            timed_out: false,
            index_age_secs: None,
            message: Some(message.into()),
        },
    }
}

fn with_fallback_reason(
    mut outcome: SystemSearchOutcome,
    fallback_reason: String,
) -> SystemSearchOutcome {
    outcome.diagnostics.fallback_reason = Some(fallback_reason);
    outcome
}

fn path_matches(path: &Path, query: &str) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.to_lowercase().contains(query))
}

#[cfg(target_os = "linux")]
fn linux_locate_index_age_secs() -> Option<u64> {
    ["/var/lib/plocate/plocate.db", "/var/lib/mlocate/mlocate.db"]
        .iter()
        .filter_map(|path| std::fs::metadata(path).ok())
        .filter_map(|metadata| metadata.modified().ok())
        .filter_map(|modified| std::time::SystemTime::now().duration_since(modified).ok())
        .map(|age| age.as_secs())
        .min()
}

#[cfg(not(target_os = "linux"))]
#[allow(dead_code)]
fn linux_locate_index_age_secs() -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn ignore_walk_fallback_finds_matching_file() {
        let root = std::env::temp_dir().join(format!("keynova-indexer-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join("nested")).expect("create nested");
        std::fs::write(root.join("nested").join("agent-plan.json"), "{}").expect("write file");

        let indexer =
            IgnoreWalkIndexer::with_limits(vec![root.clone()], 10_000, Duration::from_secs(2));
        let outcome = indexer.search("agent-plan", 5);

        assert_eq!(outcome.diagnostics.provider, "ignore_walk");
        assert_eq!(outcome.hits.len(), 1);
        assert!(outcome.hits[0].path.ends_with("agent-plan.json"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn empty_query_returns_unavailable() {
        let outcome = search_system_index("", &[], 10);
        assert!(!outcome.diagnostics.available);
        assert!(outcome.hits.is_empty());
    }
}
