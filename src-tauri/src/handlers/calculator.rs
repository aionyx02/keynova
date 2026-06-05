use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::app::feature_registry::{AssemblyCtx, FeatureRegistrar, FeatureSpec};
use crate::core::{CommandHandler, CommandResult};
use crate::managers::calculator_manager::CalculatorManager;

/// 處理 `calculator.*` 指令：eval、history。
pub struct CalculatorHandler {
    manager: Arc<Mutex<CalculatorManager>>,
}

impl CalculatorHandler {
    pub fn new(manager: Arc<Mutex<CalculatorManager>>) -> Self {
        Self { manager }
    }
}

/// DECOUP.1 (ADR-0044): self-register the calculator feature. Calculator is a
/// leaf — it builds its own manager and needs nothing from `ctx`. Removing the
/// feature is now: delete this module + its `REGISTRARS` entry.
pub fn register(reg: &mut FeatureRegistrar, _ctx: &AssemblyCtx) {
    let manager = Arc::new(Mutex::new(CalculatorManager::new()));
    reg.handler(Arc::new(CalculatorHandler::new(manager)));
    reg.spec(FeatureSpec {
        namespace: "calculator",
        flag_key: Some("features.calculator"),
    });
}

impl CommandHandler for CalculatorHandler {
    fn namespace(&self) -> &'static str {
        "calculator"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "eval" => {
                let expr = payload
                    .get("expr")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing 'expr'".to_string())?;
                let mut mgr = self.manager.lock().map_err(|e| e.to_string())?;
                let result = mgr.eval(expr)?;
                Ok(json!({ "result": result, "expr": expr }))
            }
            "history" => {
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                let history: Vec<_> = mgr
                    .history()
                    .iter()
                    .map(|e| json!({ "expr": e.expr, "result": e.result }))
                    .collect();
                Ok(json!(history))
            }
            _ => Err(format!("unknown calculator command '{command}'")),
        }
    }
}
