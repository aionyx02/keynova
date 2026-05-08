#![allow(dead_code)]

use std::sync::{Arc, RwLock};

use crate::models::action::UiSearchItem;

#[derive(Debug, Clone)]
pub struct SearchContext {
    pub query: String,
    pub limit: usize,
    pub generation: u64,
    pub workspace_id: Option<usize>,
}

/// Search provider contract for federated search sources.
pub trait SearchProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn search(&self, context: &SearchContext) -> Vec<UiSearchItem>;
}

/// Provider registry with deterministic merge and per-query generation support.
#[derive(Default)]
pub struct SearchRegistry {
    providers: RwLock<Vec<Arc<dyn SearchProvider>>>,
}

impl SearchRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, provider: Arc<dyn SearchProvider>) -> Result<(), String> {
        let mut guard = self.providers.write().map_err(|e| e.to_string())?;
        guard.push(provider);
        guard.sort_by_key(|provider| provider.id());
        Ok(())
    }

    pub fn provider_ids(&self) -> Result<Vec<&'static str>, String> {
        let guard = self.providers.read().map_err(|e| e.to_string())?;
        Ok(guard.iter().map(|provider| provider.id()).collect())
    }

    pub fn search(&self, context: &SearchContext) -> Result<Vec<UiSearchItem>, String> {
        let guard = self.providers.read().map_err(|e| e.to_string())?;
        let per_provider_limit = context.limit.max(1);
        let mut merged = Vec::new();
        for provider in guard.iter() {
            let mut provider_context = context.clone();
            provider_context.limit = per_provider_limit;
            merged.extend(provider.search(&provider_context));
        }
        merged.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.source.cmp(&right.source))
                .then_with(|| left.title.cmp(&right.title))
        });
        merged.truncate(context.limit);
        Ok(merged)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::models::action::{ActionRef, UiSearchItem};
    use crate::models::search_result::ResultKind;

    struct StaticProvider(&'static str, i64);

    impl SearchProvider for StaticProvider {
        fn id(&self) -> &'static str {
            self.0
        }

        fn search(&self, context: &SearchContext) -> Vec<UiSearchItem> {
            let action_ref = ActionRef::new(format!("{}:{}", self.0, context.query), None, 0);
            vec![UiSearchItem {
                item_ref: action_ref.clone(),
                title: self.0.into(),
                subtitle: json!({ "generation": context.generation }).to_string(),
                source: self.0.into(),
                score: self.1,
                icon_key: None,
                primary_action: action_ref,
                primary_action_label: "Open".into(),
                secondary_action_count: 0,
                kind: ResultKind::File,
                name: self.0.into(),
                path: self.0.into(),
            }]
        }
    }

    #[test]
    fn merges_by_score() {
        let registry = SearchRegistry::new();
        registry
            .register(Arc::new(StaticProvider("low", 1)))
            .unwrap();
        registry
            .register(Arc::new(StaticProvider("high", 5)))
            .unwrap();
        let results = registry
            .search(&SearchContext {
                query: "a".into(),
                limit: 2,
                generation: 1,
                workspace_id: None,
            })
            .unwrap();
        assert_eq!(results[0].source, "high");
    }
}
