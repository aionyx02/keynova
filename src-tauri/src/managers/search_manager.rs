use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::managers::{
    app_manager::AppManager,
    tantivy_index::{self, TantivyFileEntry},
};
use crate::models::search_result::{ResultKind, SearchResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchBackend {
    Everything,
    AppCache,
    Tantivy,
}

impl SearchBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            SearchBackend::Everything => "everything",
            SearchBackend::AppCache => "app_cache",
            SearchBackend::Tantivy => "tantivy",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchBackendPreference {
    Auto,
    Everything,
    AppCache,
    Tantivy,
}

impl SearchBackendPreference {
    pub fn from_config(value: Option<&str>) -> Self {
        match value.unwrap_or("auto").trim().to_lowercase().as_str() {
            "everything" => Self::Everything,
            "app_cache" | "app-cache" | "cache" => Self::AppCache,
            "tantivy" => Self::Tantivy,
            _ => Self::Auto,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Everything => "everything",
            Self::AppCache => "app_cache",
            Self::Tantivy => "tantivy",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchBackendInfo {
    pub configured: &'static str,
    pub active: &'static str,
    pub everything_available: bool,
    pub tantivy_available: bool,
    pub file_cache_entries: usize,
    pub tantivy_index_entries: usize,
    pub tantivy_index_dir: String,
    pub rebuild_supported: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchIndexRebuildStatus {
    pub started: bool,
    pub backend: &'static str,
    pub message: String,
}

pub struct SearchManager {
    app_manager: Arc<Mutex<AppManager>>,
    preference: SearchBackendPreference,
    tantivy_index_dir: std::path::PathBuf,
    rank_memory: Mutex<HashMap<String, SearchRankMemoryEntry>>,
    active_generation: AtomicU64,
    pub backend: SearchBackend,
}

#[derive(Debug, Clone)]
struct SearchRankMemoryEntry {
    count: u32,
    last_seen_secs: u64,
}

impl SearchManager {
    pub fn new_with_config(
        app_manager: Arc<Mutex<AppManager>>,
        configured_backend: Option<&str>,
        configured_index_dir: Option<&str>,
    ) -> Self {
        let preference = SearchBackendPreference::from_config(configured_backend);
        let tantivy_index_dir = tantivy_index::resolve_index_dir(configured_index_dir);
        let backend = Self::detect_backend(preference, &tantivy_index_dir);
        Self {
            app_manager,
            preference,
            tantivy_index_dir,
            rank_memory: Mutex::new(HashMap::new()),
            active_generation: AtomicU64::new(0),
            backend,
        }
    }

    pub fn set_configured_backend(
        &mut self,
        configured_backend: Option<&str>,
        configured_index_dir: Option<&str>,
    ) {
        self.preference = SearchBackendPreference::from_config(configured_backend);
        self.tantivy_index_dir = tantivy_index::resolve_index_dir(configured_index_dir);
        self.backend = Self::detect_backend(self.preference, &self.tantivy_index_dir);
    }

    fn detect_backend(
        preference: SearchBackendPreference,
        tantivy_index_dir: &std::path::Path,
    ) -> SearchBackend {
        Self::select_backend(
            preference,
            everything_available(),
            tantivy_available(tantivy_index_dir),
        )
    }

    pub fn select_backend(
        preference: SearchBackendPreference,
        everything_available: bool,
        tantivy_available: bool,
    ) -> SearchBackend {
        match preference {
            SearchBackendPreference::Everything if everything_available => {
                SearchBackend::Everything
            }
            SearchBackendPreference::Tantivy if tantivy_available => SearchBackend::Tantivy,
            SearchBackendPreference::AppCache => SearchBackend::AppCache,
            SearchBackendPreference::Auto => {
                if everything_available {
                    SearchBackend::Everything
                } else if tantivy_available {
                    SearchBackend::Tantivy
                } else {
                    SearchBackend::AppCache
                }
            }
            SearchBackendPreference::Everything | SearchBackendPreference::Tantivy => {
                SearchBackend::AppCache
            }
        }
    }

    pub fn backend_info(&self) -> SearchBackendInfo {
        let tantivy_index_entries = tantivy_index::indexed_entries(&self.tantivy_index_dir);
        SearchBackendInfo {
            configured: self.preference.as_str(),
            active: self.backend.as_str(),
            everything_available: everything_available(),
            tantivy_available: tantivy_index_entries > 0,
            file_cache_entries: file_cache_entries(),
            tantivy_index_entries,
            tantivy_index_dir: self.tantivy_index_dir.display().to_string(),
            rebuild_supported: rebuild_supported(),
        }
    }

    pub fn rebuild_index(&self) -> SearchIndexRebuildStatus {
        #[cfg(target_os = "windows")]
        {
            let index_dir = self.tantivy_index_dir.clone();
            std::thread::spawn(move || {
                let count = crate::platform::windows::build_file_index();
                let entries = crate::platform::windows::file_index_snapshot()
                    .into_iter()
                    .map(|(name, path, is_folder)| TantivyFileEntry {
                        name,
                        path,
                        is_folder,
                    })
                    .collect::<Vec<_>>();
                match tantivy_index::rebuild(&index_dir, &entries) {
                    Ok(indexed) => eprintln!(
                        "[keynova] search file index rebuilt: {count} cache entries, {indexed} tantivy docs"
                    ),
                    Err(error) => eprintln!(
                        "[keynova] tantivy index rebuild failed after cache rebuild ({count} entries): {error}"
                    ),
                }
            });
            SearchIndexRebuildStatus {
                started: true,
                backend: self.backend.as_str(),
                message: "Search index rebuild started in the background".into(),
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            SearchIndexRebuildStatus {
                started: false,
                backend: self.backend.as_str(),
                message: "Search index rebuild is only available on Windows right now".into(),
            }
        }
    }

    pub fn begin_generation(&self) -> u64 {
        self.active_generation.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn record_selection(&self, source: &str, path: &str) {
        let key = rank_key(source, path);
        if let Ok(mut memory) = self.rank_memory.lock() {
            let entry = memory.entry(key).or_insert(SearchRankMemoryEntry {
                count: 0,
                last_seen_secs: 0,
            });
            entry.count = entry.count.saturating_add(1);
            entry.last_seen_secs = now_secs();
            if memory.len() > 512 {
                trim_rank_memory(&mut memory);
            }
        }
    }

    pub fn rank_boost(&self, source: &str, path: &str) -> i64 {
        let key = rank_key(source, path);
        let Ok(memory) = self.rank_memory.lock() else {
            return 0;
        };
        let Some(entry) = memory.get(&key) else {
            return 0;
        };
        let age_secs = now_secs().saturating_sub(entry.last_seen_secs);
        let recency = if age_secs < 60 * 60 {
            25
        } else if age_secs < 24 * 60 * 60 {
            15
        } else if age_secs < 7 * 24 * 60 * 60 {
            8
        } else {
            0
        };
        recency + (entry.count.min(10) as i64 * 4)
    }

    pub fn cancel_generation(&self) -> u64 {
        self.active_generation.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn is_current_generation(&self, generation: u64) -> bool {
        self.active_generation.load(Ordering::SeqCst) == generation
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        if query.trim().is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();

        let app_slots = limit / 2;
        results.extend(self.app_results(query, app_slots));

        let file_slots = limit.saturating_sub(results.len());
        if file_slots > 0 {
            results.extend(Self::file_results_for_backend(
                self.backend,
                query,
                file_slots,
                Some(&self.tantivy_index_dir),
            ));
        }

        results.truncate(limit);
        results
    }

    pub fn app_results(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        if query.trim().is_empty() || limit == 0 {
            return Vec::new();
        }
        let Ok(mgr) = self.app_manager.lock() else {
            return Vec::new();
        };
        let q = query.to_lowercase();
        mgr.search_apps(query)
            .into_iter()
            .take(limit)
            .map(|app| {
                let score = app_name_match_score(&app.name, &q);
                SearchResult {
                    kind: ResultKind::App,
                    name: app.name,
                    path: app.path,
                    score,
                }
            })
            .collect()
    }

    pub fn file_results_for_backend(
        backend: SearchBackend,
        query: &str,
        limit: usize,
        tantivy_index_dir: Option<&std::path::Path>,
    ) -> Vec<SearchResult> {
        if query.trim().is_empty() || limit == 0 {
            return Vec::new();
        }
        let q = query.to_lowercase();

        #[cfg(target_os = "windows")]
        {
            match backend {
                SearchBackend::Everything => {
                    crate::platform::windows::everything_search(query, limit as u32)
                        .into_iter()
                        .map(|(name, path, is_folder)| SearchResult {
                            kind: if is_folder {
                                ResultKind::Folder
                            } else {
                                ResultKind::File
                            },
                            score: file_name_match_score(&name, &q),
                            name,
                            path,
                        })
                        .collect()
                }
                SearchBackend::AppCache | SearchBackend::Tantivy => {
                    if backend == SearchBackend::Tantivy {
                        if let Some(index_dir) = tantivy_index_dir {
                            if let Ok(results) = tantivy_index::search(index_dir, query, limit) {
                                if !results.is_empty() {
                                    return results;
                                }
                            }
                        }
                    }
                    crate::platform::windows::scan_files_from_cache(query, limit)
                        .into_iter()
                        .map(|(name, path, is_folder)| SearchResult {
                            kind: if is_folder {
                                ResultKind::Folder
                            } else {
                                ResultKind::File
                            },
                            score: file_name_match_score(&name, &q),
                            name,
                            path,
                        })
                        .collect()
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            if backend == SearchBackend::Tantivy {
                if let Some(index_dir) = tantivy_index_dir {
                    if let Ok(results) = tantivy_index::search(index_dir, query, limit) {
                        if !results.is_empty() {
                            return results;
                        }
                    }
                }
            }
            // Use native system indexer (mdfind / plocate / locate / ignore_walk)
            crate::managers::system_indexer::search_system_index(query, &[], limit, tantivy_index_dir)
                .hits
                .into_iter()
                .map(|hit| SearchResult {
                    kind: if hit.is_dir {
                        ResultKind::Folder
                    } else {
                        ResultKind::File
                    },
                    score: hit.score,
                    name: hit.name,
                    path: hit.path,
                })
                .collect()
        }
    }

    pub fn active_backend_name(&self) -> &'static str {
        self.backend.as_str()
    }

    pub fn tantivy_index_dir(&self) -> std::path::PathBuf {
        self.tantivy_index_dir.clone()
    }
}

/// Score an app match by name quality.
/// Exact match → 100, prefix → 90, substring → 78.
/// Substring is kept below file scores (80–95) so that a weak app match
/// does not crowd out files that match the query more precisely.
fn app_name_match_score(name: &str, query_lower: &str) -> i64 {
    let name_lower = name.to_lowercase();
    if name_lower == query_lower {
        100
    } else if name_lower.starts_with(query_lower) {
        90
    } else {
        78
    }
}

/// Score a file/folder match by name quality.
/// Exact match → 95, prefix → 88, substring → 80.
/// Exact and prefix beat app prefix (90) so relevant files surface above
/// apps that only partially match the query.
fn file_name_match_score(name: &str, query_lower: &str) -> i64 {
    let name_lower = name.to_lowercase();
    if name_lower == query_lower {
        95
    } else if name_lower.starts_with(query_lower) {
        88
    } else {
        80
    }
}

fn rank_key(source: &str, path: &str) -> String {
    format!("{source}:{path}")
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn trim_rank_memory(memory: &mut HashMap<String, SearchRankMemoryEntry>) {
    let mut entries = memory
        .iter()
        .map(|(key, entry)| (key.clone(), entry.last_seen_secs))
        .collect::<Vec<_>>();
    entries.sort_by_key(|(_, last_seen_secs)| *last_seen_secs);
    for (key, _) in entries.into_iter().take(memory.len().saturating_sub(512)) {
        memory.remove(&key);
    }
}

fn everything_available() -> bool {
    #[cfg(target_os = "windows")]
    {
        crate::platform::windows::check_everything()
    }

    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

fn tantivy_available(index_dir: &std::path::Path) -> bool {
    tantivy_index::indexed_entries(index_dir) > 0
}

fn file_cache_entries() -> usize {
    #[cfg(target_os = "windows")]
    {
        crate::platform::windows::file_index_len()
    }

    #[cfg(not(target_os = "windows"))]
    {
        0
    }
}

fn rebuild_supported() -> bool {
    cfg!(target_os = "windows")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_selection_auto_prefers_everything() {
        let backend = SearchManager::select_backend(SearchBackendPreference::Auto, true, false);
        assert_eq!(backend, SearchBackend::Everything);
    }

    #[test]
    fn test_backend_selection_auto_uses_tantivy_before_cache() {
        let backend = SearchManager::select_backend(SearchBackendPreference::Auto, false, true);
        assert_eq!(backend, SearchBackend::Tantivy);
    }

    #[test]
    fn test_backend_selection_forced_unavailable_falls_back_to_cache() {
        let backend =
            SearchManager::select_backend(SearchBackendPreference::Everything, false, false);
        assert_eq!(backend, SearchBackend::AppCache);
    }

    #[test]
    fn test_backend_selection_config_aliases() {
        assert_eq!(
            SearchBackendPreference::from_config(Some("app-cache")),
            SearchBackendPreference::AppCache
        );
        assert_eq!(
            SearchBackendPreference::from_config(Some("unknown")),
            SearchBackendPreference::Auto
        );
    }

    #[test]
    fn generation_cancels_stale_searches() {
        let app_manager = Arc::new(Mutex::new(AppManager::new()));
        let manager = SearchManager::new_with_config(app_manager, Some("app_cache"), None);
        let first = manager.begin_generation();
        assert!(manager.is_current_generation(first));
        let second = manager.cancel_generation();
        assert!(!manager.is_current_generation(first));
        assert!(manager.is_current_generation(second));
    }

    #[test]
    fn rank_memory_boosts_recent_selection() {
        let app_manager = Arc::new(Mutex::new(AppManager::new()));
        let manager = SearchManager::new_with_config(app_manager, Some("app_cache"), None);
        assert_eq!(manager.rank_boost("file", "C:/tmp/a.txt"), 0);
        manager.record_selection("file", "C:/tmp/a.txt");
        assert!(manager.rank_boost("file", "C:/tmp/a.txt") > 0);
    }
}
