pub mod ai_manager;
pub mod app_manager;
pub mod calculator_manager;
pub mod history_manager;
pub mod hotkey_manager;
pub mod learning_material_manager;
pub mod model_manager;
pub mod note_manager;
pub mod sandbox_manager;
pub mod search_manager;
pub(crate) mod search_service;
// Non-Windows file-search backend (search_manager uses it only under
// `#[cfg(not(target_os = "windows"))]`); excluded on Windows to avoid dead code.
#[cfg(not(target_os = "windows"))]
pub mod system_indexer;
pub mod system_manager;
pub mod tantivy_index;
pub mod terminal_manager;
pub mod translation_manager;
pub mod workspace_manager;
