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
        self.handlers.insert(handler.namespace().to_string(), handler);
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
