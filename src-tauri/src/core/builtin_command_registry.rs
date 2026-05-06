use crate::models::builtin_command::BuiltinCommandResult;
use serde::Serialize;
use std::collections::HashMap;

/// 可擴展的內建指令介面；每個指令實作此 trait 並向 BuiltinCommandRegistry 注冊。
pub trait BuiltinCommand: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    /// 參數語法提示，例如 `"[key] [value]"`；無參數的指令回傳 None。
    fn args_hint(&self) -> Option<&'static str> {
        None
    }
    fn execute(&self, args: &str) -> BuiltinCommandResult;
}

/// 指令元資料，用於 cmd.list 序列化。
#[derive(Debug, Clone, Serialize)]
pub struct CommandMeta {
    pub name: &'static str,
    pub description: &'static str,
    pub args_hint: Option<&'static str>,
}

/// 集中管理所有已注冊的內建指令。
#[derive(Default)]
pub struct BuiltinCommandRegistry {
    commands: HashMap<&'static str, Box<dyn BuiltinCommand>>,
}

impl BuiltinCommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注冊一個新的內建指令；同名時覆蓋舊指令。
    pub fn register(&mut self, cmd: Box<dyn BuiltinCommand>) {
        self.commands.insert(cmd.name(), cmd);
    }

    /// 執行指定指令；指令不存在時回傳 None。
    pub fn run(&self, name: &str, args: &str) -> Option<BuiltinCommandResult> {
        self.commands.get(name).map(|c| c.execute(args))
    }

    /// 回傳所有指令的元資料，依名稱排序。
    pub fn list(&self) -> Vec<CommandMeta> {
        let mut metas: Vec<CommandMeta> = self
            .commands
            .values()
            .map(|c| CommandMeta {
                name: c.name(),
                description: c.description(),
                args_hint: c.args_hint(),
            })
            .collect();
        metas.sort_by_key(|m| m.name);
        metas
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::builtin_command::{BuiltinCommandResult, CommandUiType};

    struct EchoCmd;
    impl BuiltinCommand for EchoCmd {
        fn name(&self) -> &'static str {
            "echo"
        }
        fn description(&self) -> &'static str {
            "echo args"
        }
        fn execute(&self, args: &str) -> BuiltinCommandResult {
            BuiltinCommandResult {
                text: args.to_string(),
                ui_type: CommandUiType::Inline,
            }
        }
    }

    #[test]
    fn register_and_run() {
        let mut reg = BuiltinCommandRegistry::new();
        reg.register(Box::new(EchoCmd));
        let result = reg.run("echo", "hello");
        assert!(result.is_some());
        assert_eq!(result.unwrap().text, "hello");
    }

    #[test]
    fn run_unknown_returns_none() {
        let reg = BuiltinCommandRegistry::new();
        assert!(reg.run("nope", "").is_none());
    }

    #[test]
    fn list_sorted() {
        let mut reg = BuiltinCommandRegistry::new();
        reg.register(Box::new(EchoCmd));
        let list = reg.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "echo");
    }
}
