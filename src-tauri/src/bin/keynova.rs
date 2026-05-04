use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use clap::{Parser, Subcommand};
use tauri_app_lib::core::control_plane::{send_request, ControlCommand};

#[derive(Parser)]
#[command(name = "keynova")]
#[command(about = "Control the running Keynova app")]
struct Cli {
    #[command(subcommand)]
    command: CliCommand,
}

#[derive(Subcommand)]
enum CliCommand {
    Start,
    Down,
    Reload,
    Status,
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        CliCommand::Start => start(),
        CliCommand::Down => request(ControlCommand::Down),
        CliCommand::Reload => request(ControlCommand::Reload),
        CliCommand::Status => request(ControlCommand::Status),
    };

    match result {
        Ok(message) => println!("{message}"),
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(1);
        }
    }
}

fn request(command: ControlCommand) -> Result<String, String> {
    let response = send_request(command, Duration::from_secs(2))?;
    if response.ok {
        Ok(response.message)
    } else {
        Err(response.message)
    }
}

fn start() -> Result<String, String> {
    if let Ok(response) = send_request(ControlCommand::Start, Duration::from_millis(500)) {
        return if response.ok {
            Ok(response.message)
        } else {
            Err(response.message)
        };
    }

    launch_gui()?;
    for _ in 0..40 {
        std::thread::sleep(Duration::from_millis(125));
        if let Ok(response) = send_request(ControlCommand::Start, Duration::from_millis(500)) {
            return if response.ok {
                Ok(response.message)
            } else {
                Err(response.message)
            };
        }
    }

    Err("Keynova was launched, but the control plane did not become ready".into())
}

fn launch_gui() -> Result<(), String> {
    let current = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = current
        .parent()
        .ok_or_else(|| "cannot resolve keynova executable directory".to_string())?;

    let candidates = gui_candidates(dir);
    let gui = candidates
        .iter()
        .find(|path| path.is_file())
        .ok_or_else(|| {
            format!(
                "Keynova app is not running, and no GUI executable was found beside {}",
                current.display()
            )
        })?;

    Command::new(gui).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

fn gui_candidates(dir: &Path) -> Vec<PathBuf> {
    let suffix = std::env::consts::EXE_SUFFIX;
    ["tauri-app", "Keynova", "keynova-app"]
        .into_iter()
        .map(|name| dir.join(format!("{name}{suffix}")))
        .collect()
}
