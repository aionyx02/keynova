use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use uuid::Uuid;

use crate::models::action::{Action, ActionRef};

const DEFAULT_ACTION_TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone)]
pub struct ActionSession {
    pub session_id: String,
    pub generation: u64,
}

#[derive(Debug, Clone)]
struct StoredAction {
    session_id: Option<String>,
    generation: u64,
    expires_at: Instant,
    action: Action,
}

#[derive(Default)]
struct ActionArenaInner {
    actions: HashMap<String, StoredAction>,
}

/// Holds backend-only actions and exposes short-lived ActionRef values over IPC.
pub struct ActionArena {
    inner: Mutex<ActionArenaInner>,
    next_generation: AtomicU64,
    ttl: Duration,
}

impl Default for ActionArena {
    fn default() -> Self {
        Self::new(DEFAULT_ACTION_TTL)
    }
}

impl ActionArena {
    pub fn new(ttl: Duration) -> Self {
        Self {
            inner: Mutex::new(ActionArenaInner::default()),
            next_generation: AtomicU64::new(1),
            ttl,
        }
    }

    pub fn start_session(&self) -> ActionSession {
        ActionSession {
            session_id: Uuid::new_v4().to_string(),
            generation: self.next_generation.fetch_add(1, Ordering::Relaxed),
        }
    }

    pub fn insert(&self, session: &ActionSession, action: Action) -> Result<ActionRef, String> {
        self.insert_inner(Some(session.session_id.clone()), session.generation, action)
    }

    pub fn insert_stable(&self, action: Action) -> Result<ActionRef, String> {
        self.insert_inner(None, 0, action)
    }

    fn insert_inner(
        &self,
        session_id: Option<String>,
        generation: u64,
        action: Action,
    ) -> Result<ActionRef, String> {
        let action_id = action.id.clone();
        let action_ref = ActionRef::new(action_id.clone(), session_id.clone(), generation);
        let mut guard = self.inner.lock().map_err(|e| e.to_string())?;
        prune_expired(&mut guard);
        guard.actions.insert(
            action_id,
            StoredAction {
                session_id,
                generation,
                expires_at: Instant::now() + self.ttl,
                action,
            },
        );
        Ok(action_ref)
    }

    pub fn resolve(&self, action_ref: &ActionRef) -> Result<Action, String> {
        let mut guard = self.inner.lock().map_err(|e| e.to_string())?;
        prune_expired(&mut guard);
        let stored = guard
            .actions
            .get(&action_ref.id)
            .ok_or_else(|| format!("stale or unknown action ref '{}'", action_ref.id))?;
        if stored.session_id != action_ref.session_id || stored.generation != action_ref.generation
        {
            return Err(format!(
                "stale action ref '{}' (generation {})",
                action_ref.id, action_ref.generation
            ));
        }
        Ok(stored.action.clone())
    }

    pub fn list_secondary(&self, action_ref: &ActionRef) -> Result<Vec<Action>, String> {
        Ok(self.resolve(action_ref)?.secondary_actions)
    }
}

fn prune_expired(inner: &mut ActionArenaInner) {
    let now = Instant::now();
    inner.actions.retain(|_, action| action.expires_at > now);
}

/// Registry for durable built-in actions that can be referenced by stable id.
#[derive(Default)]
pub struct ActionRegistry {
    actions: Mutex<HashMap<String, Action>>,
}

impl ActionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, action: Action) -> Result<(), String> {
        let mut guard = self.actions.lock().map_err(|e| e.to_string())?;
        guard.insert(action.id.clone(), action);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<Option<Action>, String> {
        let guard = self.actions.lock().map_err(|e| e.to_string())?;
        Ok(guard.get(id).cloned())
    }

    pub fn list(&self) -> Result<Vec<Action>, String> {
        let guard = self.actions.lock().map_err(|e| e.to_string())?;
        let mut actions: Vec<Action> = guard.values().cloned().collect();
        actions.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(actions)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::json;

    use super::*;
    use crate::models::action::Action;

    #[test]
    fn resolves_action_inside_session() {
        let arena = ActionArena::new(Duration::from_secs(60));
        let session = arena.start_session();
        let action = Action::command_route("cmd:help", "Help", "cmd.run", json!({"name":"help"}));
        let action_ref = arena.insert(&session, action).unwrap();
        assert_eq!(arena.resolve(&action_ref).unwrap().label, "Help");
    }

    #[test]
    fn rejects_stale_generation() {
        let arena = ActionArena::new(Duration::from_secs(60));
        let session = arena.start_session();
        let action_ref = arena
            .insert(&session, Action::launch_path("C:/tmp/app.exe"))
            .unwrap();
        let stale = ActionRef::new(
            action_ref.id,
            action_ref.session_id,
            action_ref.generation + 1,
        );
        assert!(arena.resolve(&stale).is_err());
    }
}
