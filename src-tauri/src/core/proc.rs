//! Spawn child processes without flashing a console window on Windows.
//!
//! `std::process::Command` does not set `CREATE_NO_WINDOW` by default, so any
//! console subprocess (powershell, wmic, netsh, wsl, …) flashes a black console
//! window — and that window steals foreground focus, which makes the Keynova
//! overlay hide/close on blur. Route every Windows spawn through `.no_window()`.

/// Extension on [`std::process::Command`] to suppress the Windows console window.
pub trait SilentCommandExt {
    /// Apply `CREATE_NO_WINDOW` on Windows; no-op on other platforms.
    fn no_window(&mut self) -> &mut Self;
}

impl SilentCommandExt for std::process::Command {
    #[cfg(windows)]
    fn no_window(&mut self) -> &mut Self {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW
        self.creation_flags(0x0800_0000)
    }

    #[cfg(not(windows))]
    fn no_window(&mut self) -> &mut Self {
        self
    }
}
