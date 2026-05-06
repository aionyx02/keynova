use std::sync::{Arc, Mutex};

use crate::managers::app_manager::AppManager;
use crate::models::search_result::{ResultKind, SearchResult};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SearchBackend {
    Everything,
    AppCache,
}

/// 統一搜尋管理器：自動偵測 Everything，否則回退至 App 快取模糊搜尋。
pub struct SearchManager {
    app_manager: Arc<Mutex<AppManager>>,
    pub backend: SearchBackend,
}

impl SearchManager {
    pub fn new(app_manager: Arc<Mutex<AppManager>>) -> Self {
        let backend = Self::detect_backend();
        Self {
            app_manager,
            backend,
        }
    }

    fn detect_backend() -> SearchBackend {
        #[cfg(target_os = "windows")]
        {
            if crate::platform::windows::check_everything() {
                return SearchBackend::Everything;
            }
        }
        SearchBackend::AppCache
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        if query.trim().is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();

        // App fuzzy search (always available, instant from cache)
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

        // File/folder search via Everything IPC (Windows, if available)
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

        // Fallback file/folder search from in-memory cache (no disk I/O at query time)
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
        match self.backend {
            SearchBackend::Everything => "everything",
            SearchBackend::AppCache => "app_cache",
        }
    }
}
