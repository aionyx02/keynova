use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use serde_json::{json, Value};
use sysinfo::{Disks, Networks, System};

use crate::core::{AppEvent, CommandHandler, CommandResult, EventBus};

#[derive(Serialize)]
struct ProcessInfo {
    name: String,
    pid: u32,
    mem_mb: u64,
    cpu_pct: f32,
}

#[derive(Serialize)]
struct DiskInfo {
    mount: String,
    used_gb: f64,
    total_gb: f64,
    pct: f64,
}

#[derive(Serialize)]
struct NetworkInfo {
    name: String,
    rx_kbps: f64,
    tx_kbps: f64,
}

#[derive(Serialize)]
struct SystemSnapshot {
    cpu_pct: f32,
    ram_used_mb: u64,
    ram_total_mb: u64,
    disks: Vec<DiskInfo>,
    networks: Vec<NetworkInfo>,
    processes: Vec<ProcessInfo>,
}

fn collect_snapshot() -> SystemSnapshot {
    let mut sys = System::new_all();
    std::thread::sleep(Duration::from_millis(200));
    sys.refresh_cpu_usage();

    let cpu_pct = sys.global_cpu_info().cpu_usage();
    let ram_used_mb = sys.used_memory() / 1_048_576;
    let ram_total_mb = sys.total_memory() / 1_048_576;

    let disks: Vec<DiskInfo> = Disks::new_with_refreshed_list()
        .iter()
        .filter(|d| d.total_space() > 0)
        .map(|d| {
            let used = d.total_space().saturating_sub(d.available_space());
            DiskInfo {
                mount: d.mount_point().to_string_lossy().into_owned(),
                used_gb: used as f64 / 1e9,
                total_gb: d.total_space() as f64 / 1e9,
                pct: used as f64 / d.total_space() as f64 * 100.0,
            }
        })
        .collect();

    let networks: Vec<NetworkInfo> = Networks::new_with_refreshed_list()
        .iter()
        .take(8)
        .map(|(name, data)| NetworkInfo {
            name: name.clone(),
            rx_kbps: data.received() as f64 / 1024.0,
            tx_kbps: data.transmitted() as f64 / 1024.0,
        })
        .collect();

    let mut processes: Vec<ProcessInfo> = sys
        .processes()
        .values()
        .map(|p| ProcessInfo {
            name: p.name().to_string(),
            pid: p.pid().as_u32(),
            mem_mb: p.memory() / 1_048_576,
            cpu_pct: p.cpu_usage(),
        })
        .collect();
    processes.sort_by_key(|p| std::cmp::Reverse(p.mem_mb));
    processes.truncate(20);

    SystemSnapshot {
        cpu_pct,
        ram_used_mb,
        ram_total_mb,
        disks,
        networks,
        processes,
    }
}

fn snapshot_to_json(cpu_pct: f32, ram_used_mb: u64, ram_total_mb: u64,
    disks: &[Value], networks: &[Value], processes: &[Value]) -> Value {
    json!({
        "cpu_pct": cpu_pct,
        "ram_used_mb": ram_used_mb,
        "ram_total_mb": ram_total_mb,
        "disks": disks,
        "networks": networks,
        "processes": processes,
    })
}

pub struct SystemMonitoringHandler {
    event_bus: Arc<EventBus>,
    stream_stop: Arc<Mutex<Option<Arc<AtomicBool>>>>,
}

impl SystemMonitoringHandler {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            event_bus,
            stream_stop: Arc::new(Mutex::new(None)),
        }
    }

    fn stop_current_stream(&self) {
        if let Ok(mut guard) = self.stream_stop.lock() {
            if let Some(flag) = guard.take() {
                flag.store(true, Ordering::Relaxed);
            }
        }
    }
}

impl CommandHandler for SystemMonitoringHandler {
    fn namespace(&self) -> &'static str {
        "system_monitoring"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "snapshot" => {
                let s = collect_snapshot();
                serde_json::to_value(s).map_err(|e| e.to_string())
            }
            "stream_start" => {
                let interval_ms = payload
                    .get("interval_ms")
                    .and_then(Value::as_u64)
                    .unwrap_or(2000)
                    .clamp(500, 10_000);

                self.stop_current_stream();

                let stop_flag = Arc::new(AtomicBool::new(false));
                {
                    let mut guard = self.stream_stop.lock().map_err(|e| e.to_string())?;
                    *guard = Some(Arc::clone(&stop_flag));
                }

                let event_bus = Arc::clone(&self.event_bus);
                std::thread::spawn(move || {
                    let mut sys = System::new_all();
                    loop {
                        std::thread::sleep(Duration::from_millis(interval_ms));
                        if stop_flag.load(Ordering::Relaxed) {
                            break;
                        }

                        sys.refresh_cpu_usage();
                        sys.refresh_memory();
                        sys.refresh_processes_specifics(sysinfo::ProcessRefreshKind::new().with_cpu().with_memory());

                        let cpu_pct = sys.global_cpu_info().cpu_usage();
                        let ram_used_mb = sys.used_memory() / 1_048_576;
                        let ram_total_mb = sys.total_memory() / 1_048_576;

                        let disks: Vec<Value> = Disks::new_with_refreshed_list()
                            .iter()
                            .filter(|d| d.total_space() > 0)
                            .map(|d| {
                                let used = d.total_space().saturating_sub(d.available_space());
                                json!({
                                    "mount": d.mount_point().to_string_lossy(),
                                    "used_gb": used as f64 / 1e9,
                                    "total_gb": d.total_space() as f64 / 1e9,
                                    "pct": used as f64 / d.total_space() as f64 * 100.0,
                                })
                            })
                            .collect();

                        let networks: Vec<Value> = Networks::new_with_refreshed_list()
                            .iter()
                            .take(8)
                            .map(|(name, data)| json!({
                                "name": name,
                                "rx_kbps": data.received() as f64 / 1024.0,
                                "tx_kbps": data.transmitted() as f64 / 1024.0,
                            }))
                            .collect();

                        let mut processes: Vec<Value> = sys
                            .processes()
                            .values()
                            .map(|p| json!({
                                "name": p.name().to_string(),
                                "pid": p.pid().as_u32(),
                                "mem_mb": p.memory() / 1_048_576,
                                "cpu_pct": p.cpu_usage(),
                            }))
                            .collect();
                        processes.sort_by(|a, b| {
                            let am = a["mem_mb"].as_u64().unwrap_or(0);
                            let bm = b["mem_mb"].as_u64().unwrap_or(0);
                            bm.cmp(&am)
                        });
                        processes.truncate(20);

                        let payload = snapshot_to_json(
                            cpu_pct, ram_used_mb, ram_total_mb, &disks, &networks, &processes,
                        );
                        let _ = event_bus.publish(AppEvent::new("system_monitoring.tick", payload));
                    }
                });

                Ok(json!({ "ok": true, "interval_ms": interval_ms }))
            }
            "stream_stop" => {
                self.stop_current_stream();
                Ok(json!({ "ok": true }))
            }
            _ => Err(format!("unknown system_monitoring command '{command}'")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_stop_with_no_active_stream_does_not_panic() {
        let bus = Arc::new(EventBus::new(32));
        let handler = SystemMonitoringHandler::new(bus);
        handler.stop_current_stream();
        handler.stop_current_stream(); // idempotent
    }

    #[test]
    fn snapshot_command_returns_ok() {
        let bus = Arc::new(EventBus::new(32));
        let handler = SystemMonitoringHandler::new(bus);
        let result = handler.execute("snapshot", Value::Null);
        assert!(result.is_ok(), "snapshot failed: {:?}", result.err());
        let v = result.unwrap();
        assert!(v["cpu_pct"].is_number());
        assert!(v["ram_total_mb"].as_u64().unwrap_or(0) > 0);
    }
}