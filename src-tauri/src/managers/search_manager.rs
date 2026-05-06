use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};

use serde::Serialize;

use crate::managers::app_manager::AppManager;
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
    active_generation: AtomicU64,
    pub backend: SearchBackend,
}

impl SearchManager {
    pub fn new_with_config(
        app_manager: Arc<Mutex<AppManager>>,
        configured_backend: Option<&str>,
    ) -> Self {
        let preference = SearchBackendPreference::from_config(configured_backend);
        let backend = Self::detect_backend(preference);
        Self {
            app_manager,
            preference,
            active_generation: AtomicU64::new(0),
            backend,
        }
    }

    pub fn set_configured_backend(&mut self, configured_backend: Option<&str>) {
        self.preference = SearchBackendPreference::from_config(configured_backend);
        self.backend = Self::detect_backend(self.preference);
    }

    fn detect_backend(preference: SearchBackendPreference) -> SearchBackend {
        Self::select_backend(preference, everything_available(), tantivy_available())
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
        SearchBackendInfo {
            configured: self.preference.as_str(),
            active: self.backend.as_str(),
            everything_available: everything_available(),
            tantivy_available: tantivy_available(),
            file_cache_entries: file_cache_entries(),
            rebuild_supported: rebuild_supported(),
        }
    }

    pub fn rebuild_index(&self) -> SearchIndexRebuildStatus {
        #[cfg(target_os = "windows")]
        {
            std::thread::spawn(|| {
                let count = crate::platform::windows::build_file_index();
                eprintln!("[keynova] search file index rebuilt: {count} entries");
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
        if let Ok(mgr) = self.app_manager.lock() {
            let apps = mgr.search_apps(query);
            for app in apps.into_iter().take(app_slots) {
                results.push(SearchResult {
                    kind: ResultKind::App,
                    name: app.name,
                    path: app.path,
                    score: 100,
                });
            }
        }

        #[cfg(target_os = "windows")]
        if self.backend == SearchBackend::Everything {
            let file_results =
                crate::platform::windows::everything_search(query, (limit / 2) as u32);
            for (name, path, is_folder) in file_results {
                results.push(SearchResult {
                    kind: if is_folder {
                        ResultKind::Folder
                    } else {
                        ResultKind::File
                    },
                    name,
                    path,
                    score: 80,
                });
            }
        }

        #[cfg(target_os = "windows")]
        if self.backend == SearchBackend::AppCache {
            let file_slots = limit.saturating_sub(results.len());
            if file_slots > 0 {
                let file_results =
                    crate::platform::windows::scan_files_from_cache(query, file_slots);
                for (name, path, is_folder) in file_results {
                    results.push(SearchResult {
                        kind: if is_folder {
                            ResultKind::Folder
                        } else {
                            ResultKind::File
                        },
                        name,
                        path,
                        score: 70,
                    });
                }
            }
        }

        results.truncate(limit);
        results
    }

    pub fn active_backend_name(&self) -> &'static str {
        self.backend.as_str()
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

fn tantivy_available() -> bool {
    false
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
        let manager = SearchManager::new_with_config(app_manager, Some("app_cache"));
        let first = manager.begin_generation();
        assert!(manager.is_current_generation(first));
        let second = manager.cancel_generation();
        assert!(!manager.is_current_generation(first));
        assert!(manager.is_current_generation(second));
    }
}
