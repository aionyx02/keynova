mod app;
pub mod core;
mod handlers;
mod managers;
mod models;
mod platform;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app::bootstrap::run();
}
