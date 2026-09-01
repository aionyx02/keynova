use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sysinfo::System;

use crate::core::config_manager::ConfigManager;
use crate::core::observability;
use crate::core::AppEvent;
use crate::managers::model_manager::{HardwareInfo, LocalModel, ModelCandidate, ModelManager};

// v2 (REF.8): dropped the `nvim_dir` path field with the nvim feature removal.
// v3: added `model.probed`, because the model section can now be filled in
// without the Ollama probe having run at all.
const SNAPSHOT_SCHEMA_VERSION: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupPreflightPaths {
    pub config_dir: String,
    pub data_dir: String,
    pub notes_dir: String,
    pub search_index_dir: String,
    pub icon_cache_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupPreflightHardware {
    pub ram_mb: u64,
    pub vram_mb: u64,
    pub cpu_cores: u64,
}

impl StartupPreflightHardware {
    pub fn as_model_hardware(&self) -> HardwareInfo {
        HardwareInfo {
            ram_mb: self.ram_mb,
            vram_mb: self.vram_mb,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupPreflightIcons {
    pub bundled_assets_ok: bool,
    pub missing_assets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupPreflightModel {
    pub ollama_url: String,
    /// Whether the Ollama probe actually ran this boot.
    ///
    /// Without this, a skipped probe is indistinguishable from a failed one:
    /// `ollama_reachable` would be `false` either way, and the model panel
    /// would tell the user Ollama is offline when nobody ever asked it. A
    /// snapshot written before this field existed always probed, hence the
    /// `true` default rather than `Default::default()`.
    #[serde(default = "probed_by_default")]
    pub probed: bool,
    pub ollama_reachable: bool,
    pub local_models: Vec<LocalModel>,
    pub recommended_models: Vec<ModelCandidate>,
}

fn probed_by_default() -> bool {
    true
}

/// How much work a preflight run should do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreflightPlan {
    /// A snapshot already describes this boot. Nothing to do.
    Skip,
    /// Paths, hardware and bundled icons only. Everything here is local; the
    /// Ollama probe — the one part that opens a socket — is left out.
    WithoutModelProbe,
    /// Everything, the Ollama probe included.
    Full,
}

impl PreflightPlan {
    pub fn runs(self) -> bool {
        !matches!(self, PreflightPlan::Skip)
    }

    pub fn probes_model(self) -> bool {
        matches!(self, PreflightPlan::Full)
    }
}

/// The facts the preflight decides from. Grouped into a struct so the decision
/// itself stays a pure function that can be tested without a running app.
#[derive(Debug, Clone, Copy)]
pub struct PreflightConditions {
    /// The user explicitly asked for a refresh (the model panel's button).
    pub forced: bool,
    /// A snapshot exists and still matches this boot, build and Ollama URL.
    pub snapshot_covers_this_boot: bool,
    /// `features.ai`. Missing means enabled, matching the dispatch guard.
    pub ai_enabled: bool,
    /// `performance.low_memory_mode`.
    pub low_memory: bool,
}

/// Decides what this boot's preflight should do.
///
/// Startup is the most latency-sensitive moment there is, and the preflight's
/// one genuinely expensive step is a network probe for a service that a user
/// with AI switched off does not have and does not want. Rather than run it
/// unconditionally and throw the answer away, the preflight now judges for
/// itself: a snapshot that still describes this boot means there is nothing to
/// do at all, and AI being off (or the machine being in low-memory mode) means
/// do the local checks and stop there.
///
/// `forced` short-circuits the whole thing to [`PreflightPlan::Full`], gates
/// included: its only caller is the model panel asking for exactly this data,
/// which is the one moment the probe is worth its latency no matter what the
/// settings say.
pub fn plan_preflight(conditions: PreflightConditions) -> PreflightPlan {
    if conditions.forced {
        return PreflightPlan::Full;
    }
    if conditions.snapshot_covers_this_boot {
        return PreflightPlan::Skip;
    }
    if !conditions.ai_enabled || conditions.low_memory {
        return PreflightPlan::WithoutModelProbe;
    }
    PreflightPlan::Full
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupPreflightSnapshot {
    pub schema_version: u32,
    pub app_version: String,
    pub boot_id: String,
    pub source_mode: String,
    pub status: String,
    pub generated_at: String,
    pub paths: StartupPreflightPaths,
    pub hardware: StartupPreflightHardware,
    pub icons: StartupPreflightIcons,
    pub model: StartupPreflightModel,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StartupPreflightStatus {
    pub running: bool,
    pub stale: bool,
    pub snapshot: Option<StartupPreflightSnapshot>,
}

#[derive(Default)]
struct StartupPreflightState {
    snapshot: Option<StartupPreflightSnapshot>,
    running: bool,
}

#[derive(Clone)]
pub struct StartupPreflight {
    config: Arc<Mutex<ConfigManager>>,
    model_manager: Arc<ModelManager>,
    event_bus: crate::core::EventBus,
    state: Arc<Mutex<StartupPreflightState>>,
}

impl StartupPreflight {
    pub fn new(
        config: Arc<Mutex<ConfigManager>>,
        model_manager: Arc<ModelManager>,
        event_bus: crate::core::EventBus,
    ) -> Self {
        let snapshot = load_snapshot_from_disk().ok().flatten();
        Self {
            config,
            model_manager,
            event_bus,
            state: Arc::new(Mutex::new(StartupPreflightState {
                snapshot,
                running: false,
            })),
        }
    }

    pub fn current_snapshot(&self) -> Option<StartupPreflightSnapshot> {
        self.state
            .lock()
            .ok()
            .and_then(|state| state.snapshot.clone())
    }

    pub fn current_status(&self) -> StartupPreflightStatus {
        let current_ollama_url = current_ollama_url(&self.config);
        let current_boot_id = current_boot_id();
        let current_source_mode = current_source_mode();
        let (snapshot, running) = match self.state.lock() {
            Ok(state) => (state.snapshot.clone(), state.running),
            Err(_) => (None, false),
        };
        let stale = snapshot.as_ref().is_some_and(|snapshot| {
            snapshot_is_stale(
                snapshot,
                &current_boot_id,
                &current_source_mode,
                &current_ollama_url,
            )
        });
        StartupPreflightStatus {
            running,
            stale,
            snapshot,
        }
    }

    pub fn model_hardware_snapshot_or_live(&self) -> HardwareInfo {
        self.current_snapshot()
            .map(|snapshot| snapshot.hardware.as_model_hardware())
            .filter(|hardware| hardware.ram_mb > 0 || hardware.vram_mb > 0)
            .unwrap_or_else(|| self.model_manager.detect_hardware())
    }

    pub fn recommended_models_snapshot_or_live(&self) -> Vec<ModelCandidate> {
        if let Some(snapshot) = self.current_snapshot() {
            if !snapshot.model.recommended_models.is_empty() {
                return snapshot.model.recommended_models;
            }
            return self
                .model_manager
                .catalog_fast(&snapshot.hardware.as_model_hardware());
        }
        let hardware = self.model_manager.detect_hardware();
        self.model_manager.catalog_fast(&hardware)
    }

    pub fn ensure_started(&self) {
        self.spawn_refresh(false);
    }

    pub fn force_refresh(&self) {
        self.spawn_refresh(true);
    }

    /// Entry point for callers that need the Ollama answer specifically.
    ///
    /// A snapshot that skipped the probe is good enough for everything else —
    /// paths, hardware, icons are all there — but not for the model panel,
    /// whose entire job is telling the user whether Ollama is up. Boot-time
    /// laziness must not become a wrong answer at the one screen that asks.
    pub fn ensure_model_probed(&self) {
        let unprobed = self
            .current_snapshot()
            .is_none_or(|snapshot| !snapshot.model.probed);
        if unprobed {
            self.force_refresh();
        } else {
            self.ensure_started();
        }
    }

    fn spawn_refresh(&self, force: bool) {
        let current_boot_id = current_boot_id();
        let current_source_mode = current_source_mode();
        let current_ollama_url = current_ollama_url(&self.config);

        let plan;
        {
            let Ok(mut state) = self.state.lock() else {
                return;
            };
            if state.running {
                return;
            }
            let covered = state.snapshot.as_ref().is_some_and(|snapshot| {
                !snapshot_is_stale(
                    snapshot,
                    &current_boot_id,
                    &current_source_mode,
                    &current_ollama_url,
                )
            });
            plan = plan_preflight(PreflightConditions {
                forced: force,
                snapshot_covers_this_boot: covered,
                ai_enabled: config_flag(&self.config, "features.ai", true),
                low_memory: config_flag(&self.config, "performance.low_memory_mode", false),
            });
            if !plan.runs() {
                return;
            }
            state.running = true;
        }

        let state = Arc::clone(&self.state);
        let config = Arc::clone(&self.config);
        let model_manager = Arc::clone(&self.model_manager);
        let event_bus = self.event_bus.clone();

        let _ = event_bus.publish(AppEvent::new(
            "startup.preflight.updated",
            json!(StartupPreflightStatus {
                running: true,
                stale: true,
                snapshot: self.current_snapshot(),
            }),
        ));

        std::thread::spawn(move || {
            let started = Instant::now();
            let mut snapshot = build_snapshot(
                &config,
                &model_manager,
                &current_boot_id,
                &current_source_mode,
                plan.probes_model(),
            );
            if let Err(error) = save_snapshot_to_disk(&snapshot) {
                snapshot.status = "partial".to_string();
                snapshot
                    .errors
                    .push(format!("Failed to persist startup snapshot: {error}"));
            }

            let running = false;
            let stale = false;

            if let Ok(mut guard) = state.lock() {
                guard.snapshot = Some(snapshot.clone());
                guard.running = false;
            }

            observability::log_startup_preflight(
                snapshot.status.as_str(),
                started.elapsed(),
                snapshot.model.ollama_reachable,
                snapshot.errors.len(),
                snapshot.warnings.len(),
            );

            let event = if snapshot.errors.is_empty() {
                "startup.preflight.updated"
            } else {
                "startup.preflight.failed"
            };
            let _ = event_bus.publish(AppEvent::new(
                event,
                json!(StartupPreflightStatus {
                    running,
                    stale,
                    snapshot: Some(snapshot),
                }),
            ));
        });
    }
}

fn build_snapshot(
    config: &Arc<Mutex<ConfigManager>>,
    model_manager: &Arc<ModelManager>,
    boot_id: &str,
    source_mode: &str,
    probe_model: bool,
) -> StartupPreflightSnapshot {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    let paths = collect_paths(config, &mut warnings, &mut errors);
    let icons = collect_icons(source_mode, &mut warnings);
    let hardware = collect_hardware(model_manager, &mut warnings);
    let model = collect_model_snapshot(config, model_manager, &hardware, probe_model, &mut warnings);

    StartupPreflightSnapshot {
        schema_version: SNAPSHOT_SCHEMA_VERSION,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        boot_id: boot_id.to_string(),
        source_mode: source_mode.to_string(),
        status: if errors.is_empty() {
            "ready"
        } else {
            "partial"
        }
        .to_string(),
        generated_at: Utc::now().to_rfc3339(),
        paths,
        hardware,
        icons,
        model,
        warnings,
        errors,
    }
}

fn collect_paths(
    config: &Arc<Mutex<ConfigManager>>,
    warnings: &mut Vec<String>,
    errors: &mut Vec<String>,
) -> StartupPreflightPaths {
    let notes_dir = config
        .lock()
        .ok()
        .and_then(|cfg| cfg.get("notes.storage_dir"))
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| crate::platform_dirs::keynova_data_dir().join("notes"));
    let search_index_dir = {
        let configured = config
            .lock()
            .ok()
            .and_then(|cfg| cfg.get("search.index_dir"));
        crate::managers::tantivy_index::resolve_index_dir(configured.as_deref())
    };
    let config_dir = crate::platform_dirs::keynova_config_dir();
    let data_dir = crate::platform_dirs::keynova_data_dir();
    let icon_cache_dir = icon_cache_dir();
    let bootstrap_dir = snapshot_dir();

    for dir in [
        config_dir.as_path(),
        data_dir.as_path(),
        notes_dir.as_path(),
        search_index_dir.as_path(),
        icon_cache_dir.as_path(),
        bootstrap_dir.as_path(),
    ] {
        if let Err(error) = std::fs::create_dir_all(dir) {
            let message = format!("Failed to create {}: {error}", dir.display());
            if dir == bootstrap_dir {
                errors.push(message);
            } else {
                warnings.push(message);
            }
        }
    }

    StartupPreflightPaths {
        config_dir: config_dir.display().to_string(),
        data_dir: data_dir.display().to_string(),
        notes_dir: notes_dir.display().to_string(),
        search_index_dir: search_index_dir.display().to_string(),
        icon_cache_dir: icon_cache_dir.display().to_string(),
    }
}

fn collect_icons(source_mode: &str, warnings: &mut Vec<String>) -> StartupPreflightIcons {
    let mut missing_assets = Vec::new();
    if source_mode == "dev" {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for relative in [
            "../src/assets/keynova_icon.png",
            "icons/32x32.png",
            "icons/128x128.png",
            "icons/icon.ico",
        ] {
            if !manifest_dir.join(relative).exists() {
                missing_assets.push(relative.replace('\\', "/"));
            }
        }
    }
    if !missing_assets.is_empty() {
        warnings.push(format!(
            "Missing dev icon assets: {}",
            missing_assets.join(", ")
        ));
    }
    StartupPreflightIcons {
        bundled_assets_ok: missing_assets.is_empty(),
        missing_assets,
    }
}

fn collect_hardware(
    model_manager: &Arc<ModelManager>,
    warnings: &mut Vec<String>,
) -> StartupPreflightHardware {
    let hardware = model_manager.detect_hardware();
    if hardware.ram_mb == 0 {
        warnings.push("Hardware detection did not resolve RAM; using safe defaults.".to_string());
    }
    StartupPreflightHardware {
        ram_mb: hardware.ram_mb,
        vram_mb: hardware.vram_mb,
        cpu_cores: std::thread::available_parallelism()
            .map(|cores| cores.get() as u64)
            .unwrap_or(1),
    }
}

fn collect_model_snapshot(
    config: &Arc<Mutex<ConfigManager>>,
    model_manager: &Arc<ModelManager>,
    hardware: &StartupPreflightHardware,
    probe_model: bool,
    warnings: &mut Vec<String>,
) -> StartupPreflightModel {
    let ollama_url = current_ollama_url(config);

    // Not probing is not the same as probing and failing, and it is not a
    // warning either — it is the plan working. `probed: false` is what stops a
    // reader turning silence into "Ollama is offline".
    if !probe_model {
        return StartupPreflightModel {
            ollama_url,
            probed: false,
            ollama_reachable: false,
            local_models: Vec::new(),
            // `catalog_fast` is local arithmetic over the hardware figures, so
            // the recommendations stay useful even with no probe.
            recommended_models: model_manager.catalog_fast(&hardware.as_model_hardware()),
        };
    }

    let probe = model_manager.probe_local(&ollama_url, Duration::from_millis(1200));
    let (ollama_reachable, local_models) = match probe {
        Ok(local_models) => (true, local_models),
        Err(error) => {
            warnings.push(format!("Ollama bootstrap probe failed: {error}"));
            (false, Vec::new())
        }
    };

    StartupPreflightModel {
        ollama_url,
        probed: true,
        ollama_reachable,
        local_models,
        recommended_models: model_manager.catalog_fast(&hardware.as_model_hardware()),
    }
}

/// Reads a boolean setting the way the rest of the app does: an absent or
/// unreadable key falls back to `default` rather than to `false`.
fn config_flag(config: &Arc<Mutex<ConfigManager>>, key: &str, default: bool) -> bool {
    config
        .lock()
        .ok()
        .and_then(|cfg| cfg.get_bool(key))
        .unwrap_or(default)
}

fn current_ollama_url(config: &Arc<Mutex<ConfigManager>>) -> String {
    let fallback = "http://localhost:11434".to_string();
    let Ok(cfg) = config.lock() else {
        return fallback;
    };
    crate::core::network_policy::configured_url(&cfg, "ai.ollama_url", &fallback, "ai.ollama_url")
        .unwrap_or_else(|error| {
            eprintln!("[keynova] startup preflight ignored invalid ai.ollama_url: {error}");
            fallback
        })
}

fn current_boot_id() -> String {
    static BOOT_ID: OnceLock<String> = OnceLock::new();
    BOOT_ID
        .get_or_init(|| format!("{}:{}", std::env::consts::OS, System::boot_time()))
        .clone()
}

fn current_source_mode() -> String {
    if cfg!(debug_assertions) {
        "dev".to_string()
    } else {
        "packaged".to_string()
    }
}

fn snapshot_is_stale(
    snapshot: &StartupPreflightSnapshot,
    current_boot_id: &str,
    current_source_mode: &str,
    current_ollama_url: &str,
) -> bool {
    snapshot.schema_version != SNAPSHOT_SCHEMA_VERSION
        || snapshot.app_version != env!("CARGO_PKG_VERSION")
        || snapshot.boot_id != current_boot_id
        || snapshot.source_mode != current_source_mode
        || snapshot.model.ollama_url != current_ollama_url
}

fn snapshot_dir() -> PathBuf {
    crate::platform_dirs::keynova_data_dir().join("bootstrap")
}

fn snapshot_path() -> PathBuf {
    snapshot_dir().join("preflight-v1.json")
}

fn icon_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(crate::platform_dirs::keynova_data_dir)
        .join("keynova")
        .join("icons")
}

/// Read-only access to the persisted preflight snapshot for callers outside the
/// preflight runtime (e.g. the `/diag` diagnostics bundle). Returns `None` when
/// the snapshot is missing or unreadable; never triggers a refresh.
pub fn read_snapshot_from_disk() -> Option<StartupPreflightSnapshot> {
    load_snapshot_from_disk().ok().flatten()
}

fn load_snapshot_from_disk() -> Result<Option<StartupPreflightSnapshot>, String> {
    let path = snapshot_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let snapshot = serde_json::from_str::<StartupPreflightSnapshot>(&content)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Some(snapshot))
}

fn save_snapshot_to_disk(snapshot: &StartupPreflightSnapshot) -> Result<(), String> {
    let path = snapshot_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(snapshot).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A boot with nothing cached, AI on, plenty of memory: the case the
    /// preflight was originally written for.
    fn cold_boot() -> PreflightConditions {
        PreflightConditions {
            forced: false,
            snapshot_covers_this_boot: false,
            ai_enabled: true,
            low_memory: false,
        }
    }

    #[test]
    fn cold_boot_runs_everything() {
        assert_eq!(plan_preflight(cold_boot()), PreflightPlan::Full);
    }

    #[test]
    fn a_snapshot_that_covers_this_boot_skips_the_run() {
        let conditions = PreflightConditions {
            snapshot_covers_this_boot: true,
            ..cold_boot()
        };
        assert_eq!(plan_preflight(conditions), PreflightPlan::Skip);
        assert!(!plan_preflight(conditions).runs());
    }

    #[test]
    fn ai_off_keeps_the_local_checks_but_drops_the_network_probe() {
        let conditions = PreflightConditions {
            ai_enabled: false,
            ..cold_boot()
        };
        let plan = plan_preflight(conditions);
        assert_eq!(plan, PreflightPlan::WithoutModelProbe);
        assert!(plan.runs(), "paths, hardware and icons are still wanted");
        assert!(!plan.probes_model(), "nobody asked Ollama anything");
    }

    #[test]
    fn low_memory_mode_also_drops_the_probe() {
        let conditions = PreflightConditions {
            low_memory: true,
            ..cold_boot()
        };
        assert_eq!(plan_preflight(conditions), PreflightPlan::WithoutModelProbe);
    }

    #[test]
    fn forcing_overrides_both_the_cache_and_the_gates() {
        // The model panel's refresh button: it wants the probe precisely
        // because AI may be off and the user is about to turn it on.
        let conditions = PreflightConditions {
            forced: true,
            snapshot_covers_this_boot: true,
            ai_enabled: false,
            low_memory: true,
        };
        assert_eq!(plan_preflight(conditions), PreflightPlan::Full);
    }

    #[test]
    fn a_legacy_snapshot_without_the_field_counts_as_probed() {
        // v2 snapshots always probed, so defaulting `probed` to false would
        // make every upgrade look like a skipped probe once.
        let model: StartupPreflightModel = serde_json::from_str(
            r#"{"ollama_url":"http://localhost:11434","ollama_reachable":true,
                "local_models":[],"recommended_models":[]}"#,
        )
        .expect("a v2 model section should still deserialize");
        assert!(model.probed);
    }
    use std::sync::{Mutex, MutexGuard};
    use std::time::Instant;

    /// `snapshot_path()` resolves to one real location under the user's data
    /// directory, so every test that touches it shares a single file. `cargo
    /// test` runs tests on parallel threads, and capture/restore alone does not
    /// stop two of them interleaving: `snapshot_round_trips_on_disk` writes a
    /// fixture with `boot_id: "windows:test"`, and if that lands between the two
    /// reads in `ensure_started_creates_snapshot_and_reuses_same_boot`, the
    /// second `StartupPreflight` sees a foreign boot id, declares the snapshot
    /// stale and rebuilds it — failing the `generated_at` assertion. That is
    /// exactly the macOS failure observed on 2026-08-23; a loaded runner widens
    /// the window but is not the cause.
    ///
    /// Serializing on acquisition fixes it without giving production code a
    /// test-only injection point.
    static SNAPSHOT_LOCK: Mutex<()> = Mutex::new(());

    struct SnapshotFileGuard {
        path: PathBuf,
        original: Option<Vec<u8>>,
        // Declared last so it is dropped last: the file is restored while the
        // lock is still held.
        _lock: MutexGuard<'static, ()>,
    }

    impl SnapshotFileGuard {
        fn capture() -> Self {
            // A panicking test poisons the mutex. Recovering keeps one failure
            // from cascading into every other test that shares the file.
            let lock = SNAPSHOT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let path = snapshot_path();
            let original = std::fs::read(&path).ok();
            Self {
                path,
                original,
                _lock: lock,
            }
        }
    }

    impl Drop for SnapshotFileGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(bytes) => {
                    if let Some(parent) = self.path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(&self.path, bytes);
                }
                None => {
                    let _ = std::fs::remove_file(&self.path);
                }
            }
        }
    }

    #[test]
    fn detects_stale_snapshot_on_boot_change() {
        let snapshot = StartupPreflightSnapshot {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            boot_id: "windows:1".into(),
            source_mode: "dev".into(),
            status: "ready".into(),
            generated_at: Utc::now().to_rfc3339(),
            paths: StartupPreflightPaths {
                config_dir: "a".into(),
                data_dir: "b".into(),
                notes_dir: "c".into(),
                search_index_dir: "d".into(),
                icon_cache_dir: "f".into(),
            },
            hardware: StartupPreflightHardware {
                ram_mb: 1,
                vram_mb: 2,
                cpu_cores: 3,
            },
            icons: StartupPreflightIcons {
                bundled_assets_ok: true,
                missing_assets: Vec::new(),
            },
            model: StartupPreflightModel {
                ollama_url: "http://localhost:11434".into(),
                probed: true,
                ollama_reachable: false,
                local_models: Vec::new(),
                recommended_models: Vec::new(),
            },
            warnings: Vec::new(),
            errors: Vec::new(),
        };

        assert!(snapshot_is_stale(
            &snapshot,
            "windows:2",
            "dev",
            "http://localhost:11434"
        ));
        assert!(!snapshot_is_stale(
            &snapshot,
            "windows:1",
            "dev",
            "http://localhost:11434"
        ));
    }

    #[test]
    fn model_hardware_conversion_matches_shape() {
        let hardware = StartupPreflightHardware {
            ram_mb: 8192,
            vram_mb: 4096,
            cpu_cores: 8,
        };
        let model_hardware = hardware.as_model_hardware();
        assert_eq!(model_hardware.ram_mb, 8192);
        assert_eq!(model_hardware.vram_mb, 4096);
    }

    #[test]
    fn snapshot_round_trips_on_disk() {
        let _guard = SnapshotFileGuard::capture();
        let snapshot = StartupPreflightSnapshot {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            boot_id: "windows:test".into(),
            source_mode: "dev".into(),
            status: "ready".into(),
            generated_at: "2026-05-29T12:34:56Z".into(),
            paths: StartupPreflightPaths {
                config_dir: "cfg".into(),
                data_dir: "data".into(),
                notes_dir: "notes".into(),
                search_index_dir: "search".into(),
                icon_cache_dir: "icons".into(),
            },
            hardware: StartupPreflightHardware {
                ram_mb: 16384,
                vram_mb: 8192,
                cpu_cores: 16,
            },
            icons: StartupPreflightIcons {
                bundled_assets_ok: true,
                missing_assets: Vec::new(),
            },
            model: StartupPreflightModel {
                ollama_url: "http://localhost:11434".into(),
                probed: true,
                ollama_reachable: false,
                local_models: Vec::new(),
                recommended_models: Vec::new(),
            },
            warnings: vec!["warn".into()],
            errors: Vec::new(),
        };

        save_snapshot_to_disk(&snapshot).expect("snapshot should persist");
        let loaded = load_snapshot_from_disk()
            .expect("snapshot should load")
            .expect("snapshot should exist");

        assert_eq!(loaded.boot_id, "windows:test");
        assert_eq!(loaded.paths.icon_cache_dir, "icons");
        assert_eq!(loaded.hardware.cpu_cores, 16);
        assert_eq!(loaded.warnings, vec!["warn".to_string()]);
    }

    #[test]
    fn ensure_started_creates_snapshot_and_reuses_same_boot() {
        let _guard = SnapshotFileGuard::capture();
        let _ = std::fs::remove_file(snapshot_path());

        let config = Arc::new(Mutex::new(ConfigManager::new()));
        let model_manager = Arc::new(ModelManager::new());
        let event_bus = crate::core::EventBus::new(8);

        let preflight = StartupPreflight::new(
            Arc::clone(&config),
            Arc::clone(&model_manager),
            event_bus.clone(),
        );
        preflight.ensure_started();

        // The bound exists to fail a hang rather than let the harness sit
        // forever, so it is sized well above the work: preflight probes
        // hardware and reaches for Ollama while several hundred other tests
        // compete for cores. Ten seconds was close enough to the real cost to
        // fire on a loaded Windows runner. This is a hang detector, not a
        // performance assertion, so being generous costs nothing.
        let wait_started = Instant::now();
        loop {
            let status = preflight.current_status();
            if !status.running && status.snapshot.is_some() {
                break;
            }
            assert!(
                wait_started.elapsed() < Duration::from_secs(60),
                "startup preflight did not finish within 60s — treat as a hang, not slowness"
            );
            std::thread::sleep(Duration::from_millis(50));
        }

        let first = load_snapshot_from_disk()
            .expect("snapshot should load")
            .expect("snapshot should exist after first run");
        let first_write = std::fs::metadata(snapshot_path())
            .and_then(|m| m.modified())
            .expect("snapshot mtime should exist");

        let second = StartupPreflight::new(config, model_manager, event_bus);
        second.ensure_started();
        std::thread::sleep(Duration::from_millis(250));

        let second_status = second.current_status();
        assert!(
            !second_status.running,
            "same-boot second ensure_started should reuse existing snapshot"
        );

        let second_snapshot = load_snapshot_from_disk()
            .expect("snapshot should reload")
            .expect("snapshot should still exist");
        let second_write = std::fs::metadata(snapshot_path())
            .and_then(|m| m.modified())
            .expect("snapshot mtime should still exist");

        assert_eq!(first.generated_at, second_snapshot.generated_at);
        assert_eq!(first.boot_id, second_snapshot.boot_id);
        assert_eq!(first_write, second_write);
    }
}
