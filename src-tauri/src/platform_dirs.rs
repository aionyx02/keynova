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

// ── 5.3.B verification ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_dir_last_component_is_keynova() {
        let dir = keynova_data_dir();
        assert_eq!(
            dir.file_name().and_then(|n| n.to_str()),
            Some("Keynova"),
            "keynova_data_dir should end with 'Keynova', got: {:?}",
            dir
        );
    }

    #[test]
    fn config_dir_last_component_is_keynova() {
        let dir = keynova_config_dir();
        assert_eq!(
            dir.file_name().and_then(|n| n.to_str()),
            Some("Keynova"),
            "keynova_config_dir should end with 'Keynova', got: {:?}",
            dir
        );
    }

    #[test]
    fn data_dir_is_absolute_path() {
        // A relative fallback would cause config files to be written relative to cwd.
        assert!(
            keynova_data_dir().is_absolute(),
            "keynova_data_dir must return an absolute path"
        );
    }

    #[test]
    fn config_dir_is_absolute_path() {
        assert!(
            keynova_config_dir().is_absolute(),
            "keynova_config_dir must return an absolute path"
        );
    }

    #[test]
    fn data_and_config_dirs_both_contain_keynova_segment() {
        // Both dirs must have "Keynova" somewhere in the path, not just as root.
        let data = keynova_data_dir();
        let cfg = keynova_config_dir();
        let data_has = data.components().any(|c| c.as_os_str() == "Keynova");
        let cfg_has = cfg.components().any(|c| c.as_os_str() == "Keynova");
        assert!(data_has, "data dir path should contain Keynova segment: {data:?}");
        assert!(cfg_has, "config dir path should contain Keynova segment: {cfg:?}");
    }
}