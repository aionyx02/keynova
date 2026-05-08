use std::path::PathBuf;

/// Platform-correct base directory for Keynova configuration files.
///
/// - Windows: `%APPDATA%\Keynova`
/// - macOS:   `~/Library/Application Support/Keynova`
/// - Linux:   `$XDG_CONFIG_HOME/keynova` → `~/.config/keynova`
pub fn keynova_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Keynova")
}

/// Platform-correct base directory for Keynova data files (notes, DB, search index).
///
/// - Windows: `%APPDATA%\Keynova`
/// - macOS:   `~/Library/Application Support/Keynova`
/// - Linux:   `$XDG_DATA_HOME/keynova` → `~/.local/share/keynova`
///
/// Falls back to `keynova_config_dir()` when `dirs::data_dir()` is unavailable.
pub fn keynova_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(keynova_config_dir)
        .join("Keynova")
}