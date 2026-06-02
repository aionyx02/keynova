#![cfg(target_os = "windows")]

//! Windows platform integration.
//!
//! REF.9.C split the original single file into focused submodules; this file is
//! now a thin facade re-exporting the public surface so external callers keep
//! using `crate::platform::windows::<fn>` unchanged.

mod apps;
mod everything;
mod fileindex;
mod icon;
mod input;

pub use apps::{launch_app, scan_applications};
pub use everything::{check_everything, everything_search};
pub use fileindex::{build_file_index, file_index_len, file_index_snapshot, scan_files_from_cache};
pub use icon::{search_icon_data_url, warm_icon_cache};
pub use input::{
    clipboard_sequence_number, move_cursor_absolute, move_cursor_relative, read_clipboard_text,
    simulate_click, simulate_key, type_text,
};
