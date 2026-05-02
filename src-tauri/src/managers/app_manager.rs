use crate::models::app::AppInfo;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

/// 管理已安裝應用程式的掃描、搜尋與啟動。
pub struct AppManager {
    cache: Vec<AppInfo>,
    matcher: SkimMatcherV2,
}

impl AppManager {
    pub fn new() -> Self {
        Self {
            cache: Vec::new(),
            matcher: SkimMatcherV2::default(),
        }
    }

    /// 掃描系統應用並更新快取。
    pub fn scan_applications(&mut self) {
        #[cfg(target_os = "windows")]
        {
            self.cache = crate::platform::windows::scan_applications();
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.cache = Vec::new();
        }
    }

    /// 回傳快取中的全部應用列表（快取為空時先掃描）。
    pub fn list_all(&mut self) -> Vec<AppInfo> {
        if self.cache.is_empty() {
            self.scan_applications();
        }
        self.cache.clone()
    }

    /// 模糊搜尋應用程式，依匹配分數排序。
    pub fn search_apps(&self, query: &str) -> Vec<AppInfo> {
        if query.is_empty() {
            return self.cache.clone();
        }
        let mut scored: Vec<(i64, &AppInfo)> = self
            .cache
            .iter()
            .filter_map(|app| {
                self.matcher
                    .fuzzy_match(&app.name, query)
                    .map(|score| (score, app))
            })
            .collect();
        scored.sort_by_key(|b| std::cmp::Reverse(b.0));
        scored.into_iter().map(|(_, app)| app.clone()).collect()
    }

    /// 以路徑啟動應用程式，並累加啟動次數。
    pub fn launch_app(&mut self, path: &str) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            crate::platform::windows::launch_app(path)?;
        }
        #[cfg(not(target_os = "windows"))]
        {
            std::process::Command::new(path)
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        if let Some(app) = self.cache.iter_mut().find(|a| a.path == path) {
            app.launch_count += 1;
        }
        Ok(())
    }
}
