use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;

pub type CommandResult = Result<Value, String>;

pub trait CommandHandler: Send + Sync {
    fn namespace(&self) -> &'static str;
    fn execute(&self, command: &str, payload: Value) -> CommandResult;
}

#[derive(Default)]
pub struct CommandRouter {
    handlers: HashMap<String, Arc<dyn CommandHandler>>,
}

impl CommandRouter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, handler: Arc<dyn CommandHandler>) {
        self.handlers
            .insert(handler.namespace().to_string(), handler);
    }

    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }

    pub fn dispatch(&self, route: &str, payload: Value) -> CommandResult {
        let (namespace, command) = route
            .split_once('.')
            .ok_or_else(|| format!("invalid route '{route}', expected 'namespace.command'"))?;

        let handler = self
            .handlers
            .get(namespace)
            .ok_or_else(|| format!("no handler registered for namespace '{namespace}'"))?;

        handler.execute(command, payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct EchoHandler {
        ns: &'static str,
    }

    impl CommandHandler for EchoHandler {
        fn namespace(&self) -> &'static str {
            self.ns
        }
        fn execute(&self, command: &str, payload: Value) -> CommandResult {
            Ok(json!({ "ns": self.ns, "command": command, "payload": payload }))
        }
    }

    #[test]
    fn dispatch_routes_to_registered_handler() {
        let mut router = CommandRouter::new();
        router.register(Arc::new(EchoHandler { ns: "search" }));
        let result = router.dispatch("search.query", json!({ "q": "test" })).unwrap();
        assert_eq!(result["ns"], "search");
        assert_eq!(result["command"], "query");
    }

    #[test]
    fn dispatch_unknown_namespace_returns_error() {
        let router = CommandRouter::new();
        let err = router.dispatch("unknown.cmd", json!(null)).unwrap_err();
        assert!(err.contains("unknown"), "error should mention the namespace: {err}");
    }

    #[test]
    fn dispatch_route_without_dot_returns_error() {
        let router = CommandRouter::new();
        let err = router.dispatch("nodotroute", json!(null)).unwrap_err();
        assert!(err.contains("invalid route"), "error should say invalid route: {err}");
    }

    #[test]
    fn dispatch_empty_route_returns_error() {
        let router = CommandRouter::new();
        assert!(router.dispatch("", json!(null)).is_err());
    }

    #[test]
    fn dispatch_multi_dot_route_splits_on_first_dot() {
        let mut router = CommandRouter::new();
        router.register(Arc::new(EchoHandler { ns: "agent" }));
        let result = router.dispatch("agent.run.start", json!(null)).unwrap();
        assert_eq!(result["ns"], "agent");
        assert_eq!(result["command"], "run.start");
    }

    #[test]
    fn handler_count_reflects_registered_handlers() {
        let mut router = CommandRouter::new();
        assert_eq!(router.handler_count(), 0);
        router.register(Arc::new(EchoHandler { ns: "a" }));
        router.register(Arc::new(EchoHandler { ns: "b" }));
        assert_eq!(router.handler_count(), 2);
    }
}
