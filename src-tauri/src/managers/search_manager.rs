use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::managers::{
    app_manager::AppManager,
    tantivy_index::{self, TantivyFileEntry},
};
use crate::models::search_result::{ResultKind, SearchResult};

pub const STARTUP_INDEX_MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);

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
    pub indexing: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchIndexWarmupReason {
    Empty,
    Stale,
}

impl SearchIndexWarmupReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Stale => "stale",
        }
    }
}

pub struct SearchManager {
    app_manager: Arc<Mutex<AppManager>>,
    preference: SearchBackendPreference,
    tantivy_index_dir: std::path::PathBuf,
    rank_memory: Mutex<HashMap<String, SearchRankMemoryEntry>>,
    active_generation: AtomicU64,
    indexing: AtomicBool,
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
            indexing: AtomicBool::new(false),
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

    pub fn refresh_backend(&mut self) {
        self.backend = Self::detect_backend(self.preference, &self.tantivy_index_dir);
    }

    pub fn active_backend(&self) -> SearchBackend {
        Self::detect_backend(self.preference, &self.tantivy_index_dir)
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
        let active = self.active_backend();
        SearchBackendInfo {
            configured: self.preference.as_str(),
            active: active.as_str(),
            everything_available: everything_available(),
            tantivy_available: tantivy_index_entries > 0,
            indexing: self.indexing(),
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
                backend: self.active_backend().as_str(),
                message: "Search index rebuild started in the background".into(),
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            SearchIndexRebuildStatus {
                started: false,
                backend: self.active_backend().as_str(),
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

    /// Returns `(recency_boost, frequency_boost)` for the given key.
    ///
    /// The UI surfaces these as `score_breakdown` so the user can see "why this
    /// rank". Session-only values reset on app restart.
    pub fn rank_boost_breakdown(&self, source: &str, path: &str) -> (i64, i64) {
        let key = rank_key(source, path);
        let Ok(memory) = self.rank_memory.lock() else {
            return (0, 0);
        };
        let Some(entry) = memory.get(&key) else {
            return (0, 0);
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
        let frequency = entry.count.min(10) as i64 * 4;
        (recency, frequency)
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
                self.active_backend(),
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
        let fetch_limit = existing_path_fetch_limit(limit);

        #[cfg(target_os = "windows")]
        {
            match backend {
                SearchBackend::Everything => {
                    let results =
                        crate::platform::windows::everything_search(query, fetch_limit as u32)
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
                            .collect();
                    filter_existing_file_results(results, limit)
                }
                SearchBackend::AppCache | SearchBackend::Tantivy => {
                    if backend == SearchBackend::Tantivy {
                        if let Some(index_dir) = tantivy_index_dir {
                            if let Ok(results) =
                                tantivy_index::search(index_dir, query, fetch_limit)
                            {
                                let results = filter_existing_file_results(results, limit);
                                if !results.is_empty() {
                                    return results;
                                }
                            }
                        }
                    }
                    let results =
                        crate::platform::windows::scan_files_from_cache(query, fetch_limit)
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
                            .collect();
                    filter_existing_file_results(results, limit)
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            if backend == SearchBackend::Tantivy {
                if let Some(index_dir) = tantivy_index_dir {
                    if let Ok(results) = tantivy_index::search(index_dir, query, fetch_limit) {
                        let results = filter_existing_file_results(results, limit);
                        if !results.is_empty() {
                            return results;
                        }
                    }
                }
            }
            // Use native system indexer (mdfind / plocate / locate / ignore_walk)
            let results = crate::managers::system_indexer::search_system_index(
                query,
                &[],
                fetch_limit,
                tantivy_index_dir,
            )
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
            .collect();
            filter_existing_file_results(results, limit)
        }
    }

    pub fn active_backend_name(&self) -> &'static str {
        self.active_backend().as_str()
    }

    pub fn tantivy_index_dir(&self) -> std::path::PathBuf {
        self.tantivy_index_dir.clone()
    }

    pub fn indexing(&self) -> bool {
        self.indexing.load(Ordering::SeqCst)
    }

    pub fn set_indexing(&self, indexing: bool) {
        self.indexing.store(indexing, Ordering::SeqCst);
    }
}

pub fn startup_index_warmup_reason(
    index_dir: &std::path::Path,
    max_age: Duration,
) -> Option<SearchIndexWarmupReason> {
    let entries = tantivy_index::indexed_entries(index_dir);
    let age = tantivy_index::index_age(index_dir);
    index_warmup_reason_for_state(entries, age, max_age)
}

fn index_warmup_reason_for_state(
    entries: usize,
    age: Option<Duration>,
    max_age: Duration,
) -> Option<SearchIndexWarmupReason> {
    if entries == 0 {
        return Some(SearchIndexWarmupReason::Empty);
    }
    if age.is_some_and(|age| age > max_age) {
        return Some(SearchIndexWarmupReason::Stale);
    }
    None
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

fn existing_path_fetch_limit(limit: usize) -> usize {
    limit.saturating_mul(4).min(1_000).max(limit)
}

fn filter_existing_file_results(results: Vec<SearchResult>, limit: usize) -> Vec<SearchResult> {
    results
        .into_iter()
        .filter(search_result_path_exists)
        .take(limit)
        .collect()
}

fn search_result_path_exists(result: &SearchResult) -> bool {
    match &result.kind {
        ResultKind::File | ResultKind::Folder => {
            path_matches_result_kind(&result.path, &result.kind)
        }
        _ => true,
    }
}

fn path_matches_result_kind(path: &str, kind: &ResultKind) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    match kind {
        ResultKind::File => metadata.is_file(),
        ResultKind::Folder => metadata.is_dir(),
        _ => true,
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
        let (r0, f0) = manager.rank_boost_breakdown("file", "C:/tmp/a.txt");
        assert_eq!((r0, f0), (0, 0));
        manager.record_selection("file", "C:/tmp/a.txt");
        let (r1, f1) = manager.rank_boost_breakdown("file", "C:/tmp/a.txt");
        assert!(r1 + f1 > 0);
    }

    #[test]
    fn rank_boost_breakdown_missing_entry_returns_zero() {
        let app_manager = Arc::new(Mutex::new(AppManager::new()));
        let manager = SearchManager::new_with_config(app_manager, Some("app_cache"), None);
        assert_eq!(
            manager.rank_boost_breakdown("file", "C:/tmp/missing.txt"),
            (0, 0)
        );
    }

    #[test]
    fn rank_boost_breakdown_fresh_entry_yields_recency_and_frequency() {
        let app_manager = Arc::new(Mutex::new(AppManager::new()));
        let manager = SearchManager::new_with_config(app_manager, Some("app_cache"), None);
        for _ in 0..3 {
            manager.record_selection("file", "C:/tmp/a.txt");
        }
        let (recency, frequency) = manager.rank_boost_breakdown("file", "C:/tmp/a.txt");
        assert_eq!(recency, 25, "fresh selection should give max recency");
        assert_eq!(frequency, 12, "count=3 should give frequency = 3*4 = 12");
    }

    #[test]
    fn index_warmup_reason_empty_index_rebuilds() {
        assert_eq!(
            index_warmup_reason_for_state(0, None, STARTUP_INDEX_MAX_AGE),
            Some(SearchIndexWarmupReason::Empty)
        );
    }

    #[test]
    fn index_warmup_reason_stale_index_rebuilds() {
        assert_eq!(
            index_warmup_reason_for_state(
                10,
                Some(STARTUP_INDEX_MAX_AGE + Duration::from_secs(1)),
                STARTUP_INDEX_MAX_AGE,
            ),
            Some(SearchIndexWarmupReason::Stale)
        );
    }

    #[test]
    fn index_warmup_reason_fresh_index_is_ready() {
        assert_eq!(
            index_warmup_reason_for_state(
                10,
                Some(STARTUP_INDEX_MAX_AGE - Duration::from_secs(1)),
                STARTUP_INDEX_MAX_AGE,
            ),
            None
        );
    }

    #[test]
    fn indexing_state_is_reported_in_backend_info() {
        let app_manager = Arc::new(Mutex::new(AppManager::new()));
        let manager = SearchManager::new_with_config(app_manager, Some("app_cache"), None);
        assert!(!manager.backend_info().indexing);

        manager.set_indexing(true);
        assert!(manager.backend_info().indexing);
    }

    #[test]
    fn rank_boost_breakdown_frequency_capped_at_ten() {
        let app_manager = Arc::new(Mutex::new(AppManager::new()));
        let manager = SearchManager::new_with_config(app_manager, Some("app_cache"), None);
        for _ in 0..15 {
            manager.record_selection("file", "C:/tmp/a.txt");
        }
        let (_, frequency) = manager.rank_boost_breakdown("file", "C:/tmp/a.txt");
        assert_eq!(
            frequency, 40,
            "count.min(10) * 4 = 40 even with 15 selections"
        );
    }

    #[test]
    fn rank_boost_breakdown_decays_after_recency_windows() {
        // Reaching into the rank_memory directly to forge an aged timestamp keeps
        // this test deterministic without sleeping for hours of wall-clock time.
        let app_manager = Arc::new(Mutex::new(AppManager::new()));
        let manager = SearchManager::new_with_config(app_manager, Some("app_cache"), None);
        manager.record_selection("file", "C:/tmp/a.txt");
        {
            let mut mem = manager.rank_memory.lock().unwrap();
            let entry = mem.get_mut("file:C:/tmp/a.txt").unwrap();
            entry.last_seen_secs = entry.last_seen_secs.saturating_sub(8 * 24 * 60 * 60);
        }
        let (recency, frequency) = manager.rank_boost_breakdown("file", "C:/tmp/a.txt");
        assert_eq!(recency, 0, "8-day-old selection should give 0 recency");
        assert_eq!(
            frequency, 4,
            "one selection should give frequency = 1*4 = 4"
        );
    }

    #[test]
    fn filter_existing_file_results_drops_missing_paths() {
        let dir = tempfile::TempDir::new().unwrap();
        let live_file = dir.path().join("live.md");
        let live_folder = dir.path().join("folder");
        std::fs::write(&live_file, "ok").unwrap();
        std::fs::create_dir_all(&live_folder).unwrap();

        let results = vec![
            SearchResult {
                kind: ResultKind::File,
                name: "deleted.md".into(),
                path: dir.path().join("deleted.md").display().to_string(),
                score: 99,
            },
            SearchResult {
                kind: ResultKind::File,
                name: "live.md".into(),
                path: live_file.display().to_string(),
                score: 98,
            },
            SearchResult {
                kind: ResultKind::Folder,
                name: "folder".into(),
                path: live_folder.display().to_string(),
                score: 97,
            },
            SearchResult {
                kind: ResultKind::Folder,
                name: "wrong-kind".into(),
                path: live_file.display().to_string(),
                score: 96,
            },
        ];

        let filtered = filter_existing_file_results(results, 10);
        let names = filtered
            .iter()
            .map(|result| result.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["live.md", "folder"]);
    }

    #[test]
    fn filter_existing_file_results_applies_limit_after_stale_drop() {
        let dir = tempfile::TempDir::new().unwrap();
        let live_file = dir.path().join("live.md");
        std::fs::write(&live_file, "ok").unwrap();

        let results = vec![
            SearchResult {
                kind: ResultKind::File,
                name: "deleted.md".into(),
                path: dir.path().join("deleted.md").display().to_string(),
                score: 99,
            },
            SearchResult {
                kind: ResultKind::File,
                name: "live.md".into(),
                path: live_file.display().to_string(),
                score: 98,
            },
        ];

        let filtered = filter_existing_file_results(results, 1);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "live.md");
    }
}
