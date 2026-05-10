use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use uuid::Uuid;

use crate::core::builtin_command_registry::BuiltinCommand;
use crate::core::config_manager::ConfigManager;
use crate::core::{BuiltinCommandRegistry, CommandHandler, CommandResult};
use crate::managers::{
    model_manager::ModelManager, note_manager::NoteManager, search_manager::SearchManager,
};
use crate::models::builtin_command::{BuiltinCommandResult, CommandUiType};
use crate::models::terminal::{TerminalEnvVar, TerminalLaunchSpec};

pub struct HelpCommand;

impl BuiltinCommand for HelpCommand {
    fn name(&self) -> &'static str {
        "help"
    }

    fn description(&self) -> &'static str {
        "Show available commands"
    }

    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: String::new(),
            ui_type: CommandUiType::Inline,
        }
    }
}

pub struct SettingCommand;

impl BuiltinCommand for SettingCommand {
    fn name(&self) -> &'static str {
        "setting"
    }

    fn description(&self) -> &'static str {
        "Open or edit settings"
    }

    fn args_hint(&self) -> Option<&'static str> {
        Some("[key] [value]")
    }

    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: String::new(),
            ui_type: CommandUiType::Panel("setting".into()),
        }
    }
}

pub struct ReloadCommand;

impl BuiltinCommand for ReloadCommand {
    fn name(&self) -> &'static str {
        "reload"
    }

    fn description(&self) -> &'static str {
        "Reload config from disk"
    }

    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: "Reload requested".into(),
            ui_type: CommandUiType::Inline,
        }
    }
}

// ─── Phase 3 builtin commands ────────────────────────────────────────────────

pub struct TrCommand;

impl BuiltinCommand for TrCommand {
    fn name(&self) -> &'static str {
        "tr"
    }
    fn description(&self) -> &'static str {
        "Translate text"
    }
    fn args_hint(&self) -> Option<&'static str> {
        Some("<src> <dst> <text>  or  default <text>")
    }
    fn execute(&self, args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: args.to_string(),
            ui_type: CommandUiType::Panel("translation".into()),
        }
    }
}

pub struct AiCommand;

impl BuiltinCommand for AiCommand {
    fn name(&self) -> &'static str {
        "ai"
    }
    fn description(&self) -> &'static str {
        "Ask AI assistant"
    }
    fn args_hint(&self) -> Option<&'static str> {
        Some("<question>")
    }
    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: String::new(),
            ui_type: CommandUiType::Panel("ai".into()),
        }
    }
}

pub struct ModelDownloadCommand;

impl BuiltinCommand for ModelDownloadCommand {
    fn name(&self) -> &'static str {
        "model_download"
    }
    fn description(&self) -> &'static str {
        "Download or enable a local AI model"
    }
    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: String::new(),
            ui_type: CommandUiType::Panel("model_download".into()),
        }
    }
}

pub struct ModelListCommand;

impl BuiltinCommand for ModelListCommand {
    fn name(&self) -> &'static str {
        "model_list"
    }
    fn description(&self) -> &'static str {
        "List and switch AI models"
    }
    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: String::new(),
            ui_type: CommandUiType::Panel("model_list".into()),
        }
    }
}

pub struct ModelRemoveCommand {
    manager: Arc<ModelManager>,
    config: Arc<Mutex<ConfigManager>>,
}

impl ModelRemoveCommand {
    pub fn new(manager: Arc<ModelManager>, config: Arc<Mutex<ConfigManager>>) -> Self {
        Self { manager, config }
    }

    fn ollama_url(&self) -> String {
        self.config
            .lock()
            .ok()
            .and_then(|c| c.get("ollama.base_url"))
            .unwrap_or_else(|| "http://localhost:11434".into())
    }
}

impl BuiltinCommand for ModelRemoveCommand {
    fn name(&self) -> &'static str {
        "model_remove"
    }
    fn description(&self) -> &'static str {
        "Remove a local Ollama model"
    }
    fn execute(&self, args: &str) -> BuiltinCommandResult {
        let name = args.trim();
        if name.is_empty() {
            return BuiltinCommandResult {
                text: String::new(),
                ui_type: CommandUiType::Panel("model_remove".into()),
            };
        }
        let base_url = self.ollama_url();
        match self.manager.delete(&base_url, name) {
            Ok(()) => BuiltinCommandResult {
                text: format!("已刪除模型 {name}"),
                ui_type: CommandUiType::Inline,
            },
            Err(e) => BuiltinCommandResult {
                text: format!("刪除失敗: {e}"),
                ui_type: CommandUiType::Inline,
            },
        }
    }
}

pub struct NoteCommand {
    manager: Arc<Mutex<NoteManager>>,
    config: Arc<Mutex<ConfigManager>>,
}

impl NoteCommand {
    pub fn new(manager: Arc<Mutex<NoteManager>>, config: Arc<Mutex<ConfigManager>>) -> Self {
        Self { manager, config }
    }
}

impl BuiltinCommand for NoteCommand {
    fn name(&self) -> &'static str {
        "note"
    }
    fn description(&self) -> &'static str {
        "Quick notes"
    }
    fn args_hint(&self) -> Option<&'static str> {
        Some("[lazyvim [note-name] | lazyvim --path <path>]")
    }
    fn execute(&self, args: &str) -> BuiltinCommandResult {
        let (configured_command, configured_config_dir) = self
            .config
            .lock()
            .ok()
            .map(|config| {
                (
                    config.get("notes.lazyvim_command"),
                    config.get("notes.lazyvim_config_dir"),
                )
            })
            .unwrap_or((None, None));
        match self.manager.lock() {
            Ok(manager) => run_note_command(
                args,
                &manager,
                configured_command.as_deref(),
                configured_config_dir.as_deref(),
                &find_command_in_path,
            ),
            Err(error) => inline_result(format!("Note manager unavailable: {error}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NoteCommandMode {
    BuiltinPanel,
    LazyVim(NoteLaunchTarget),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NoteLaunchTarget {
    NotesRoot,
    Named(String),
    Path(String),
}

fn run_note_command(
    args: &str,
    manager: &NoteManager,
    configured_command: Option<&str>,
    configured_config_dir: Option<&str>,
    command_finder: &dyn Fn(&str) -> Option<PathBuf>,
) -> BuiltinCommandResult {
    let mode = match parse_note_args(args) {
        Ok(mode) => mode,
        Err(error) => return inline_result(error),
    };

    let NoteCommandMode::LazyVim(target) = mode else {
        return BuiltinCommandResult {
            text: String::new(),
            ui_type: CommandUiType::Panel("note".into()),
        };
    };

    let Some(program) = resolve_editor_command(configured_command, command_finder) else {
        return BuiltinCommandResult {
            text: String::new(),
            ui_type: CommandUiType::Panel("nvim_download".into()),
        };
    };

    let lazyvim_config = match resolve_lazyvim_config(configured_config_dir) {
        Ok(config) => config,
        Err(error) => return inline_result(error),
    };

    let (target_path, title, cwd) = match resolve_lazyvim_target(&target, manager) {
        Ok(target) => target,
        Err(error) => return inline_result(error),
    };

    BuiltinCommandResult {
        text: String::new(),
        ui_type: CommandUiType::Terminal(TerminalLaunchSpec {
            launch_id: Uuid::new_v4().to_string(),
            program: program.display().to_string(),
            args: vec![target_path.display().to_string()],
            cwd: Some(cwd.display().to_string()),
            title: Some(title),
            env: lazyvim_config.env,
            editor: true,
        }),
    }
}

struct LazyVimConfig {
    env: Vec<TerminalEnvVar>,
}

struct LazyVimLayout {
    config_parent: PathBuf,
    config_dir: PathBuf,
    data_home: PathBuf,
    state_home: PathBuf,
    cache_home: PathBuf,
}

fn inline_result(text: String) -> BuiltinCommandResult {
    BuiltinCommandResult {
        text,
        ui_type: CommandUiType::Inline,
    }
}

fn parse_note_args(args: &str) -> Result<NoteCommandMode, String> {
    let trimmed = args.trim();
    if trimmed.is_empty() {
        return Ok(NoteCommandMode::BuiltinPanel);
    }
    if trimmed != "lazyvim" && !trimmed.starts_with("lazyvim ") {
        return Ok(NoteCommandMode::BuiltinPanel);
    }

    let rest = trimmed.strip_prefix("lazyvim").unwrap_or_default().trim();
    if rest.is_empty() {
        return Ok(NoteCommandMode::LazyVim(NoteLaunchTarget::NotesRoot));
    }
    if let Some(path) = rest.strip_prefix("--path") {
        let path = path.trim();
        if path.is_empty() {
            return Err("Usage: /note lazyvim --path <absolute-or-relative-path>".into());
        }
        return Ok(NoteCommandMode::LazyVim(NoteLaunchTarget::Path(
            strip_wrapping_quotes(path).to_string(),
        )));
    }
    Ok(NoteCommandMode::LazyVim(NoteLaunchTarget::Named(
        strip_wrapping_quotes(rest).to_string(),
    )))
}

fn resolve_lazyvim_target(
    target: &NoteLaunchTarget,
    manager: &NoteManager,
) -> Result<(PathBuf, String, PathBuf), String> {
    match target {
        NoteLaunchTarget::NotesRoot => {
            let root = manager.notes_root();
            std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
            Ok((root.clone(), "Notes".into(), root))
        }
        NoteLaunchTarget::Named(name) => {
            let path = manager.resolve_named_note(name);
            manager.create_parent_dirs_for_file(&path)?;
            let cwd = path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| manager.notes_root());
            Ok((path, format!("Note: {name}"), cwd))
        }
        NoteLaunchTarget::Path(raw_path) => {
            let path = resolve_user_note_path(raw_path, &manager.notes_root())?;
            manager.create_parent_dirs_for_file(&path)?;
            let cwd = path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| manager.notes_root());
            Ok((path, "Note path".into(), cwd))
        }
    }
}

fn resolve_lazyvim_config(configured_config_dir: Option<&str>) -> Result<LazyVimConfig, String> {
    let config_dir = configured_config_dir
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(expand_user_path)
        .unwrap_or_else(default_nvim_config_dir);
    let project_managed = is_project_managed_lazyvim_dir(&config_dir);

    ensure_lazyvim_config(&config_dir, project_managed)?;

    if !looks_like_lazyvim_config(&config_dir) {
        return Err(format!(
            "LazyVim config was not found at {} and Keynova did not overwrite the existing directory.\nUse an empty project-managed directory, set notes.lazyvim_config_dir, or install the official starter with:\n  git clone https://github.com/LazyVim/starter {}\nThe LazyVim distro repo is https://github.com/LazyVim/LazyVim.",
            config_dir.display(),
            config_dir.display()
        ));
    }

    Ok(LazyVimConfig {
        env: lazyvim_env_for_config_dir(&config_dir),
    })
}

fn ensure_lazyvim_config(config_dir: &Path, project_managed: bool) -> Result<(), String> {
    if looks_like_lazyvim_config(config_dir) {
        write_missing_lazyvim_template_files(config_dir, false)?;
        return Ok(());
    }

    let dir_exists = config_dir.exists();
    let dir_empty = !dir_exists
        || std::fs::read_dir(config_dir)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(true);
    if !project_managed && !dir_empty {
        return Ok(());
    }

    write_missing_lazyvim_template_files(config_dir, project_managed || dir_empty)
}

fn write_missing_lazyvim_template_files(
    config_dir: &Path,
    overwrite_existing: bool,
) -> Result<(), String> {
    for (relative, content) in lazyvim_template_files() {
        let path = config_dir.join(relative);
        if path.exists() && !overwrite_existing {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, content).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

fn looks_like_lazyvim_config(config_dir: &Path) -> bool {
    if !config_dir.is_dir() {
        return false;
    }
    [
        config_dir.join("lua").join("config").join("lazy.lua"),
        config_dir.join("init.lua"),
        config_dir.join("lazy-lock.json"),
    ]
    .iter()
    .filter_map(|path| std::fs::read_to_string(path).ok())
    .any(|content| {
        content.contains("LazyVim/LazyVim")
            || content.contains("lazyvim.plugins")
            || content.contains("lazyvim.config")
    })
}

fn lazyvim_template_files() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            ".keynova-managed",
            "This LazyVim runtime config is generated by Keynova.\nDelete the project or this .keynova directory to remove it.\n",
        ),
        ("init.lua", "require(\"config.lazy\")\n"),
        (
            "lua/config/lazy.lua",
            r#"local lazypath = vim.fn.stdpath("data") .. "/lazy/lazy.nvim"
if not vim.uv.fs_stat(lazypath) then
  local lazyrepo = "https://github.com/folke/lazy.nvim.git"
  local result = vim.fn.system({ "git", "clone", "--filter=blob:none", "--branch=stable", lazyrepo, lazypath })
  if vim.v.shell_error ~= 0 then
    vim.api.nvim_echo({
      { "Failed to clone lazy.nvim:\n", "ErrorMsg" },
      { result, "WarningMsg" },
      { "\nPress any key to exit..." },
    }, true, {})
    vim.fn.getchar()
    os.exit(1)
  end
end
vim.opt.rtp:prepend(lazypath)

require("lazy").setup({
  spec = {
    { "LazyVim/LazyVim", import = "lazyvim.plugins" },
    { import = "plugins" },
  },
  install = { colorscheme = { "tokyonight", "habamax" } },
  checker = { enabled = true, notify = false },
})
"#,
        ),
        (
            "lua/config/options.lua",
            "vim.g.mapleader = \" \"\nvim.g.maplocalleader = \"\\\\\"\nvim.opt.wrap = true\nvim.opt.spell = false\n",
        ),
        (
            "lua/config/keymaps.lua",
            "-- Add project-local LazyVim keymaps here.\n",
        ),
        (
            "lua/config/autocmds.lua",
            "-- Add project-local LazyVim autocommands here.\n",
        ),
        (
            "lua/plugins/keynova.lua",
            "return {\n  -- Project-local LazyVim plugin specs can be added here.\n}\n",
        ),
    ]
}

fn lazyvim_env_for_config_dir(config_dir: &Path) -> Vec<TerminalEnvVar> {
    let layout = project_lazyvim_layout();
    let Some(appname) = config_dir
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
    else {
        return Vec::new();
    };

    let mut env = vec![TerminalEnvVar {
        key: "NVIM_APPNAME".into(),
        value: appname.to_string(),
    }];
    let config_parent = config_dir
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or(layout.config_parent);
    env.push(TerminalEnvVar {
        key: "XDG_CONFIG_HOME".into(),
        value: config_parent.display().to_string(),
    });
    env.push(TerminalEnvVar {
        key: "XDG_DATA_HOME".into(),
        value: layout.data_home.display().to_string(),
    });
    env.push(TerminalEnvVar {
        key: "XDG_STATE_HOME".into(),
        value: layout.state_home.display().to_string(),
    });
    env.push(TerminalEnvVar {
        key: "XDG_CACHE_HOME".into(),
        value: layout.cache_home.display().to_string(),
    });
    env
}

fn default_nvim_config_dir() -> PathBuf {
    project_lazyvim_layout().config_dir
}

fn project_lazyvim_layout() -> LazyVimLayout {
    let root = project_runtime_root().join(".keynova").join("lazyvim");
    LazyVimLayout {
        config_parent: root.join("config"),
        config_dir: root.join("config").join("keynova-lazyvim"),
        data_home: root.join("data"),
        state_home: root.join("state"),
        cache_home: root.join("cache"),
    }
}

fn project_runtime_root() -> PathBuf {
    if let Some(root) = std::env::var_os("KEYNOVA_PROJECT_DIR").map(PathBuf::from) {
        return root;
    }
    if let Ok(current) = std::env::current_dir() {
        if let Some(root) = find_project_root_from(&current) {
            return root;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn find_project_root_from(start: &Path) -> Option<PathBuf> {
    start.ancestors().find_map(|path| {
        (path.join("tasks.md").is_file() && path.join("src-tauri").is_dir())
            .then(|| path.to_path_buf())
    })
}

fn is_project_managed_lazyvim_dir(config_dir: &Path) -> bool {
    let layout = project_lazyvim_layout();
    config_dir == layout.config_dir
        || config_dir.starts_with(project_runtime_root().join(".keynova"))
}

fn resolve_user_note_path(input: &str, notes_root: &Path) -> Result<PathBuf, String> {
    let value = strip_wrapping_quotes(input.trim());
    if value.is_empty() {
        return Err("Note path cannot be empty".into());
    }
    let path = PathBuf::from(value);
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("Note paths cannot contain '..' segments".into());
    }
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(notes_root.join(path))
    }
}

fn expand_user_path(value: &str) -> PathBuf {
    let mut expanded = value.to_string();
    for key in [
        "LOCALAPPDATA",
        "APPDATA",
        "USERPROFILE",
        "HOME",
        "XDG_CONFIG_HOME",
    ] {
        if let Some(env_value) = std::env::var_os(key).and_then(|value| value.into_string().ok()) {
            expanded = expanded.replace(&format!("%{key}%"), &env_value);
            expanded = expanded.replace(&format!("${key}"), &env_value);
            expanded = expanded.replace(&format!("$env:{key}"), &env_value);
        }
    }
    if expanded == "~" {
        if let Some(home) = home_dir() {
            return home;
        }
    }
    if let Some(rest) = expanded
        .strip_prefix("~/")
        .or_else(|| expanded.strip_prefix("~\\"))
    {
        if let Some(home) = home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(expanded)
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn strip_wrapping_quotes(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
        .unwrap_or(value)
}

fn resolve_editor_command(
    configured_command: Option<&str>,
    command_finder: &dyn Fn(&str) -> Option<PathBuf>,
) -> Option<PathBuf> {
    configured_command
        .map(str::trim)
        .filter(|command| !command.is_empty())
        .and_then(|command| resolve_command(command, command_finder))
        .or_else(|| command_finder("nvim"))
}

fn resolve_command(
    command: &str,
    command_finder: &dyn Fn(&str) -> Option<PathBuf>,
) -> Option<PathBuf> {
    let path = PathBuf::from(command);
    if path.is_absolute() || command.contains('/') || command.contains('\\') {
        return path.is_file().then_some(path);
    }
    command_finder(command)
}

fn find_command_in_path(program: &str) -> Option<PathBuf> {
    let candidates = command_candidates(program);
    std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path).find_map(|dir| {
            candidates
                .iter()
                .map(|candidate| dir.join(candidate))
                .find(|path| path.is_file())
        })
    })
}

fn command_candidates(program: &str) -> Vec<String> {
    let path = Path::new(program);
    if path.extension().is_some() {
        return vec![program.to_string()];
    }

    #[cfg(target_os = "windows")]
    {
        let pathext = std::env::var("PATHEXT").unwrap_or_else(|_| ".EXE;.CMD;.BAT;.COM".into());
        let mut candidates = vec![program.to_string()];
        for ext in pathext.split(';').filter(|ext| !ext.is_empty()) {
            candidates.push(format!("{program}{ext}"));
        }
        candidates
    }

    #[cfg(not(target_os = "windows"))]
    {
        vec![program.to_string()]
    }
}

pub struct CalCommand;

impl BuiltinCommand for CalCommand {
    fn name(&self) -> &'static str {
        "cal"
    }
    fn description(&self) -> &'static str {
        "Calculator & unit conversion"
    }
    fn args_hint(&self) -> Option<&'static str> {
        Some("<expr>  e.g. 2+2, 5 km to m")
    }
    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: String::new(),
            ui_type: CommandUiType::Panel("calculator".into()),
        }
    }
}

pub struct HistoryCommand;

impl BuiltinCommand for HistoryCommand {
    fn name(&self) -> &'static str {
        "history"
    }
    fn description(&self) -> &'static str {
        "Clipboard history"
    }
    fn args_hint(&self) -> Option<&'static str> {
        Some("[search]")
    }
    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: String::new(),
            ui_type: CommandUiType::Panel("history".into()),
        }
    }
}

pub struct SysCtlCommand;

impl BuiltinCommand for SysCtlCommand {
    fn name(&self) -> &'static str {
        "system"
    }
    fn description(&self) -> &'static str {
        "System control: volume, brightness, wifi"
    }
    fn args_hint(&self) -> Option<&'static str> {
        Some("volume|brightness|wifi [value]")
    }
    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: String::new(),
            ui_type: CommandUiType::Panel("system".into()),
        }
    }
}

pub struct SysMonitorCommand;

impl BuiltinCommand for SysMonitorCommand {
    fn name(&self) -> &'static str {
        "system_monitoring"
    }
    fn description(&self) -> &'static str {
        "CPU, RAM, Disk, Network & Process monitor"
    }
    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: String::new(),
            ui_type: CommandUiType::Panel("system_monitoring".into()),
        }
    }
}

pub struct DownCommand;

impl BuiltinCommand for DownCommand {
    fn name(&self) -> &'static str {
        "down"
    }

    fn description(&self) -> &'static str {
        "Gracefully quit Keynova"
    }

    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: "Shutdown requested".into(),
            ui_type: CommandUiType::Inline,
        }
    }
}

pub struct RebuildSearchIndexCommand;

impl BuiltinCommand for RebuildSearchIndexCommand {
    fn name(&self) -> &'static str {
        "rebuild_search_index"
    }

    fn description(&self) -> &'static str {
        "Rebuild the local search index"
    }

    fn execute(&self, _args: &str) -> BuiltinCommandResult {
        BuiltinCommandResult {
            text: "Search index rebuild requested".into(),
            ui_type: CommandUiType::Inline,
        }
    }
}

pub struct BuiltinCmdHandler {
    registry: Arc<Mutex<BuiltinCommandRegistry>>,
    config: Arc<Mutex<ConfigManager>>,
    search_manager: Arc<Mutex<SearchManager>>,
}

impl BuiltinCmdHandler {
    pub fn new(
        registry: Arc<Mutex<BuiltinCommandRegistry>>,
        config: Arc<Mutex<ConfigManager>>,
        search_manager: Arc<Mutex<SearchManager>>,
    ) -> Self {
        Self {
            registry,
            config,
            search_manager,
        }
    }
}

impl CommandHandler for BuiltinCmdHandler {
    fn namespace(&self) -> &'static str {
        "cmd"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "list" => {
                let reg = self.registry.lock().map_err(|e| e.to_string())?;
                Ok(json!(reg.list()))
            }
            "run" => {
                let name = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing 'name' field".to_string())?;
                let args = payload
                    .get("args")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim();

                // Feature guard: disabled features return a friendly message instead of executing.
                const FEATURE_GUARDS: &[(&str, &str)] = &[
                    ("ai", "features.ai"),
                    ("tr", "features.translation"),
                    ("note", "features.notes"),
                    ("history", "features.history"),
                    ("cal", "features.calculator"),
                ];
                for &(cmd_name, feature_key) in FEATURE_GUARDS {
                    if name == cmd_name {
                        let cfg = self.config.lock().map_err(|e| e.to_string())?;
                        let enabled = cfg
                            .get(feature_key)
                            .as_deref()
                            .map(|v| !v.eq_ignore_ascii_case("false"))
                            .unwrap_or(true);
                        if !enabled {
                            return Ok(json!(BuiltinCommandResult {
                                text: format!(
                                    "/{name} 功能已停用。請前往 /setting → Features 開啟。"
                                ),
                                ui_type: CommandUiType::Inline,
                            }));
                        }
                        break;
                    }
                }

                if name == "setting" && !args.is_empty() {
                    let mut parts = args.splitn(2, ' ');
                    let key = parts.next().unwrap_or("").trim();
                    let value_opt = parts.next().map(str::trim).filter(|s| !s.is_empty());
                    return match value_opt {
                        Some(value) => {
                            let mut cfg = self.config.lock().map_err(|e| e.to_string())?;
                            cfg.set(key, value).map_err(|e| e.to_string())?;
                            Ok(json!(BuiltinCommandResult {
                                text: format!("✓ {} = {}", key, value),
                                ui_type: CommandUiType::Inline,
                            }))
                        }
                        None => {
                            let cfg = self.config.lock().map_err(|e| e.to_string())?;
                            let current = cfg.get(key).unwrap_or_else(|| "(not set)".to_string());
                            Ok(json!(BuiltinCommandResult {
                                text: format!("{} = {}", key, current),
                                ui_type: CommandUiType::Inline,
                            }))
                        }
                    };
                }

                if name == "help" {
                    let list = {
                        let reg = self.registry.lock().map_err(|e| e.to_string())?;
                        reg.list()
                    };
                    let text = list
                        .iter()
                        .map(|meta| format!("/{} - {}", meta.name, meta.description))
                        .collect::<Vec<_>>()
                        .join("\n");
                    return Ok(json!(BuiltinCommandResult {
                        text,
                        ui_type: CommandUiType::Inline,
                    }));
                }

                if name == "rebuild_search_index" {
                    let status = self
                        .search_manager
                        .lock()
                        .map_err(|e| e.to_string())?
                        .rebuild_index();
                    return Ok(json!(BuiltinCommandResult {
                        text: status.message,
                        ui_type: CommandUiType::Inline,
                    }));
                }

                let result = {
                    let reg = self.registry.lock().map_err(|e| e.to_string())?;
                    reg.run(name, args)
                };
                result
                    .map(|result| json!(result))
                    .ok_or_else(|| format!("unknown command '/{name}'"))
            }
            "suggest_args" => {
                let name = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing 'name'".to_string())?;
                let partial = payload
                    .get("partial")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_lowercase();
                if name == "setting" {
                    let cfg = self.config.lock().map_err(|e| e.to_string())?;
                    let mut keys: Vec<String> = cfg
                        .schema()
                        .into_iter()
                        .map(|schema| schema.key.to_string())
                        .filter(|key| partial.is_empty() || key.starts_with(&partial))
                        .collect();
                    if keys.is_empty() {
                        keys = cfg
                            .list_all()
                            .into_iter()
                            .filter(|(k, _)| partial.is_empty() || k.starts_with(&partial))
                            .map(|(k, _)| k)
                            .collect();
                    }
                    return Ok(json!(keys));
                }
                Ok(json!(Vec::<String>::new()))
            }
            _ => Err(format!("unknown cmd command '{command}'")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_note_manager() -> (NoteManager, PathBuf) {
        let root = std::env::temp_dir().join(format!("keynova-note-test-{}", Uuid::new_v4()));
        let manager = NoteManager::new(Some(root.display().to_string()));
        (manager, root)
    }

    fn write_lazyvim_config(root: &Path) -> PathBuf {
        let config_dir = root.join("lazyvim-config");
        let lazy_file = config_dir.join("lua").join("config").join("lazy.lua");
        std::fs::create_dir_all(lazy_file.parent().expect("lazy marker parent"))
            .expect("create lazyvim config dirs");
        std::fs::write(
            &lazy_file,
            r#"{ "LazyVim/LazyVim", import = "lazyvim.plugins" }"#,
        )
        .expect("write lazyvim marker");
        config_dir
    }

    fn fake_finder(command: &str) -> Option<PathBuf> {
        (command == "nvim").then(|| PathBuf::from("nvim"))
    }

    #[test]
    fn note_without_args_keeps_builtin_panel() {
        let (manager, root) = temp_note_manager();
        let result = run_note_command("", &manager, None, None, &fake_finder);
        assert!(matches!(result.ui_type, CommandUiType::Panel(ref name) if name == "note"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn note_lazyvim_without_target_opens_notes_root() {
        let (manager, root) = temp_note_manager();
        let config_dir = write_lazyvim_config(&root);
        let config_dir = config_dir.display().to_string();
        let result = run_note_command("lazyvim", &manager, None, Some(&config_dir), &fake_finder);
        let CommandUiType::Terminal(spec) = result.ui_type else {
            panic!("expected terminal result");
        };
        assert_eq!(spec.args, vec![root.display().to_string()]);
        assert_eq!(spec.cwd, Some(root.display().to_string()));
        assert!(spec.env.iter().any(|env| env.key == "NVIM_APPNAME"));
        assert!(spec.editor);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn note_lazyvim_named_note_resolves_markdown_file() {
        let (manager, root) = temp_note_manager();
        let config_dir = write_lazyvim_config(&root);
        let config_dir = config_dir.display().to_string();
        let result = run_note_command(
            "lazyvim project plan",
            &manager,
            None,
            Some(&config_dir),
            &fake_finder,
        );
        let CommandUiType::Terminal(spec) = result.ui_type else {
            panic!("expected terminal result");
        };
        assert!(spec.args[0].ends_with("project_plan.md"));
        assert_eq!(spec.cwd, Some(root.display().to_string()));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn note_lazyvim_path_resolves_relative_path_under_notes_root() {
        let (manager, root) = temp_note_manager();
        let config_dir = write_lazyvim_config(&root);
        let config_dir = config_dir.display().to_string();
        let result = run_note_command(
            "lazyvim --path nested/today.md",
            &manager,
            None,
            Some(&config_dir),
            &fake_finder,
        );
        let CommandUiType::Terminal(spec) = result.ui_type else {
            panic!("expected terminal result");
        };
        let expected = root.join("nested").join("today.md");
        assert_eq!(
            normalize_path_for_assertion(&spec.args[0]),
            normalize_path_for_assertion(&expected.display().to_string())
        );
        assert!(expected.parent().is_some_and(|parent| parent.is_dir()));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn note_lazyvim_rejects_parent_dir_path_segments() {
        let (manager, root) = temp_note_manager();
        let config_dir = write_lazyvim_config(&root);
        let config_dir = config_dir.display().to_string();
        let result = run_note_command(
            "lazyvim --path ../escape.md",
            &manager,
            None,
            Some(&config_dir),
            &fake_finder,
        );
        assert!(matches!(result.ui_type, CommandUiType::Inline));
        assert!(result.text.contains(".."));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn note_lazyvim_missing_nvim_returns_inline_guidance() {
        let (manager, root) = temp_note_manager();
        let result = run_note_command("lazyvim", &manager, None, None, &|_| None);
        assert!(matches!(result.ui_type, CommandUiType::Inline));
        assert!(result.text.contains("Neovim was not found"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn note_lazyvim_missing_config_bootstraps_starter_files() {
        let (manager, root) = temp_note_manager();
        let missing_config = root.join("missing-lazyvim");
        let missing_config_value = missing_config.display().to_string();
        let result = run_note_command(
            "lazyvim",
            &manager,
            None,
            Some(&missing_config_value),
            &fake_finder,
        );
        let CommandUiType::Terminal(spec) = result.ui_type else {
            panic!("expected terminal result");
        };
        assert!(missing_config.join("init.lua").is_file());
        assert!(missing_config
            .join("lua")
            .join("config")
            .join("lazy.lua")
            .is_file());
        assert!(spec.env.iter().any(|env| env.key == "XDG_DATA_HOME"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn terminal_launch_result_serializes_with_terminal_type() {
        let result = BuiltinCommandResult {
            text: String::new(),
            ui_type: CommandUiType::Terminal(TerminalLaunchSpec {
                launch_id: "launch-1".into(),
                program: "nvim".into(),
                args: vec!["note.md".into()],
                cwd: Some("notes".into()),
                title: Some("Note: note".into()),
                env: vec![TerminalEnvVar {
                    key: "NVIM_APPNAME".into(),
                    value: "lazyvim".into(),
                }],
                editor: true,
            }),
        };
        let value = serde_json::to_value(result).expect("serialize terminal command result");
        assert_eq!(value["ui_type"]["type"], "Terminal");
        assert_eq!(value["ui_type"]["value"]["launch_id"], "launch-1");
        assert_eq!(value["ui_type"]["value"]["args"][0], "note.md");
        assert_eq!(value["ui_type"]["value"]["env"][0]["key"], "NVIM_APPNAME");
        assert_eq!(value["ui_type"]["value"]["editor"], true);
    }

    fn normalize_path_for_assertion(path: &str) -> String {
        path.replace('\\', "/")
    }
}
