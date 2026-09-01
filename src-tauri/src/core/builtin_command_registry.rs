//! Builtin command registry.
//!
//! Commands used to be a compile-time set: the map was keyed by `&'static str`
//! and every name was a literal, so a command could only exist if it had been
//! written into the binary. That is now one of two origins.
//!
//! A [`CommandOrigin::Plugin`] command is registered and removed while the app
//! is running, which is what "pluggable" means here and why the key had to
//! become an owned `String`. One rule keeps that from turning into a hijacking
//! surface: **a plugin may not take a name a builtin already owns.** `/setting`
//! opening something other than settings is not an extension, and refusing at
//! registration is the only place that check is cheap and total — after
//! registration nothing downstream can tell the two apart.
//!
//! Where plugin definitions come from is deliberately not decided here. This
//! module is the mechanism only; the source (and the trust boundary that comes
//! with it) is a separate decision — see docs/state.md, Planned.

use crate::models::builtin_command::BuiltinCommandResult;
use serde::Serialize;
use std::collections::HashMap;

/// 可擴展的內建指令介面；每個指令實作此 trait 並向 BuiltinCommandRegistry 注冊。
///
/// The three metadata methods return `&str` rather than `&'static str` so an
/// implementation may own its strings — a command defined at runtime has no
/// static lifetime to give.
pub trait BuiltinCommand: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    /// 參數語法提示，例如 `"[key] [value]"`；無參數的指令回傳 None。
    fn args_hint(&self) -> Option<&str> {
        None
    }
    fn execute(&self, args: &str) -> BuiltinCommandResult;
}

/// Where a registered command came from. Not cosmetic: it is what lets a reload
/// pull out everything that was plugged in without touching what was compiled
/// into the binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandOrigin {
    /// Compiled in and registered during assembly. Permanent for the process.
    Builtin,
    /// Registered at runtime. Removable, and never allowed to shadow a builtin.
    Plugin,
}

/// 指令元資料，用於 cmd.list 序列化。
#[derive(Debug, Clone, Serialize)]
pub struct CommandMeta {
    pub name: String,
    pub description: String,
    pub args_hint: Option<String>,
    /// Lets a consumer tell a shipped command from a plugged-in one without a
    /// second lookup. Additive: existing consumers ignore it.
    pub origin: CommandOrigin,
}

struct Registered {
    origin: CommandOrigin,
    command: Box<dyn BuiltinCommand>,
}

/// 集中管理所有已注冊的內建指令。
#[derive(Default)]
pub struct BuiltinCommandRegistry {
    commands: HashMap<String, Registered>,
}

impl BuiltinCommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注冊一個內建指令；同名時覆蓋舊指令。
    ///
    /// Assembly-time only. Overwriting stays allowed because assembly is a
    /// single ordered pass the developer controls; it is runtime registration
    /// that needs a guard, not this.
    pub fn register(&mut self, cmd: Box<dyn BuiltinCommand>) {
        self.insert(CommandOrigin::Builtin, cmd);
    }

    /// Registers a command at runtime.
    ///
    /// Fails rather than shadowing a builtin — see the module note. Replacing
    /// one plugin command with another of the same name is fine, and is how a
    /// reload updates a definition in place.
    pub fn register_plugin(&mut self, cmd: Box<dyn BuiltinCommand>) -> Result<(), String> {
        let name = cmd.name().trim().to_string();
        if name.is_empty() {
            return Err("plugin command has an empty name".to_string());
        }
        if self.origin_of(&name) == Some(CommandOrigin::Builtin) {
            return Err(format!(
                "'{name}' is a builtin command and cannot be replaced"
            ));
        }
        self.insert(CommandOrigin::Plugin, cmd);
        Ok(())
    }

    /// Removes one plugin command. `false` means there was nothing to remove, or
    /// the name belongs to a builtin — either way the registry is unchanged.
    pub fn unregister_plugin(&mut self, name: &str) -> bool {
        if self.origin_of(name) != Some(CommandOrigin::Plugin) {
            return false;
        }
        self.commands.remove(name).is_some()
    }

    /// Removes every plugin command, returning how many went. This is the reload
    /// path: drop the lot, then register the new set.
    pub fn unregister_all_plugins(&mut self) -> usize {
        let before = self.commands.len();
        self.commands
            .retain(|_, entry| entry.origin == CommandOrigin::Builtin);
        before - self.commands.len()
    }

    /// 執行指定指令；指令不存在時回傳 None。
    pub fn run(&self, name: &str, args: &str) -> Option<BuiltinCommandResult> {
        self.commands.get(name).map(|c| c.command.execute(args))
    }

    pub fn origin_of(&self, name: &str) -> Option<CommandOrigin> {
        self.commands.get(name).map(|entry| entry.origin)
    }

    /// 回傳所有指令的元資料，依名稱排序。
    pub fn list(&self) -> Vec<CommandMeta> {
        let mut metas: Vec<CommandMeta> = self
            .commands
            .values()
            .map(|entry| CommandMeta {
                name: entry.command.name().to_string(),
                description: entry.command.description().to_string(),
                args_hint: entry.command.args_hint().map(str::to_string),
                origin: entry.origin,
            })
            .collect();
        metas.sort_by(|a, b| a.name.cmp(&b.name));
        metas
    }

    fn insert(&mut self, origin: CommandOrigin, command: Box<dyn BuiltinCommand>) {
        self.commands.insert(
            command.name().trim().to_string(),
            Registered { origin, command },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::builtin_command::{BuiltinCommandResult, CommandUiType};

    struct EchoCmd;
    impl BuiltinCommand for EchoCmd {
        fn name(&self) -> &str {
            "echo"
        }
        fn description(&self) -> &str {
            "echo args"
        }
        fn execute(&self, args: &str) -> BuiltinCommandResult {
            BuiltinCommandResult {
                text: args.to_string(),
                ui_type: CommandUiType::Inline,
            }
        }
    }

    /// Stands in for a command defined at runtime: it owns its strings, which is
    /// the whole reason the trait no longer promises `'static`.
    struct OwnedCmd {
        name: String,
        reply: String,
    }

    impl OwnedCmd {
        fn new(name: &str, reply: &str) -> Self {
            Self {
                name: name.to_string(),
                reply: reply.to_string(),
            }
        }
    }

    impl BuiltinCommand for OwnedCmd {
        fn name(&self) -> &str {
            &self.name
        }
        fn description(&self) -> &str {
            "runtime command"
        }
        fn execute(&self, _args: &str) -> BuiltinCommandResult {
            BuiltinCommandResult {
                text: self.reply.clone(),
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
        assert_eq!(list[0].origin, CommandOrigin::Builtin);
    }

    #[test]
    fn plugin_command_registers_and_runs_with_an_owned_name() {
        let mut reg = BuiltinCommandRegistry::new();
        reg.register_plugin(Box::new(OwnedCmd::new("deploy", "shipped")))
            .expect("a free name should register");
        assert_eq!(reg.origin_of("deploy"), Some(CommandOrigin::Plugin));
        assert_eq!(reg.run("deploy", "").unwrap().text, "shipped");
    }

    #[test]
    fn plugin_cannot_shadow_a_builtin() {
        let mut reg = BuiltinCommandRegistry::new();
        reg.register(Box::new(EchoCmd));

        let error = reg
            .register_plugin(Box::new(OwnedCmd::new("echo", "hijacked")))
            .expect_err("a builtin name must be refused");
        assert!(
            error.contains("echo"),
            "error should name the command: {error}"
        );

        // The builtin is untouched: it still resolves, and still runs its own code.
        assert_eq!(reg.run("echo", "hello").unwrap().text, "hello");
        assert_eq!(reg.origin_of("echo"), Some(CommandOrigin::Builtin));
    }

    #[test]
    fn plugin_replaces_its_own_earlier_definition() {
        let mut reg = BuiltinCommandRegistry::new();
        reg.register_plugin(Box::new(OwnedCmd::new("deploy", "v1")))
            .unwrap();
        reg.register_plugin(Box::new(OwnedCmd::new("deploy", "v2")))
            .expect("re-registering a plugin name is how a reload updates it");
        assert_eq!(reg.run("deploy", "").unwrap().text, "v2");
    }

    #[test]
    fn unregister_plugin_removes_only_plugins() {
        let mut reg = BuiltinCommandRegistry::new();
        reg.register(Box::new(EchoCmd));
        reg.register_plugin(Box::new(OwnedCmd::new("deploy", "shipped")))
            .unwrap();

        assert!(reg.unregister_plugin("deploy"));
        assert!(reg.run("deploy", "").is_none());

        // A builtin is not removable through the plugin door.
        assert!(!reg.unregister_plugin("echo"));
        assert!(reg.run("echo", "hi").is_some());
    }

    #[test]
    fn unregister_all_plugins_leaves_builtins_standing() {
        let mut reg = BuiltinCommandRegistry::new();
        reg.register(Box::new(EchoCmd));
        reg.register_plugin(Box::new(OwnedCmd::new("a", "1"))).unwrap();
        reg.register_plugin(Box::new(OwnedCmd::new("b", "2"))).unwrap();

        assert_eq!(reg.unregister_all_plugins(), 2);
        assert_eq!(reg.list().len(), 1);
        assert_eq!(reg.list()[0].name, "echo");
        // Idempotent: a second reload with nothing plugged in removes nothing.
        assert_eq!(reg.unregister_all_plugins(), 0);
    }

    #[test]
    fn empty_plugin_name_is_refused() {
        let mut reg = BuiltinCommandRegistry::new();
        assert!(reg
            .register_plugin(Box::new(OwnedCmd::new("   ", "x")))
            .is_err());
        assert!(reg.list().is_empty());
    }
}
