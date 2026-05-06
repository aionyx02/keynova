use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::Value;

/// Size and serialization timing for a JSON IPC payload.
#[derive(Debug, Clone, Copy)]
pub struct JsonMetric {
    pub bytes: usize,
    pub serialization_us: u128,
}

/// Measures JSON serialization cost without logging the payload contents.
pub fn measure_json(value: &Value) -> JsonMetric {
    let started = Instant::now();
    let bytes = serde_json::to_vec(value)
        .map(|encoded| encoded.len())
        .unwrap_or(0);
    JsonMetric {
        bytes,
        serialization_us: started.elapsed().as_micros(),
    }
}

pub fn log_ipc_command(
    route: &str,
    ok: bool,
    elapsed: Duration,
    request: JsonMetric,
    response: Option<JsonMetric>,
    error_code: Option<&str>,
) {
    #[cfg(debug_assertions)]
    {
        let response_bytes = response.map(|metric| metric.bytes).unwrap_or(0);
        let response_serialization_us = response.map(|metric| metric.serialization_us).unwrap_or(0);
        eprintln!(
            "[keynova][obs] ipc.command route={route} ok={ok} latency_ms={:.2} request_bytes={} request_serialize_us={} response_bytes={} response_serialize_us={} error_code={}",
            elapsed.as_secs_f64() * 1000.0,
            request.bytes,
            request.serialization_us,
            response_bytes,
            response_serialization_us,
            error_code.unwrap_or("-"),
        );
    }
}

pub fn log_action_execution(name: &str, ok: bool, elapsed: Duration) {
    #[cfg(debug_assertions)]
    eprintln!(
        "[keynova][obs] action.execution name={name} ok={ok} latency_ms={:.2}",
        elapsed.as_secs_f64() * 1000.0,
    );
}

pub fn log_search_query(
    backend: &str,
    query_len: usize,
    limit: usize,
    result_count: usize,
    elapsed: Duration,
) {
    #[cfg(debug_assertions)]
    eprintln!(
        "[keynova][obs] search.query backend={backend} query_len={query_len} limit={limit} result_count={result_count} latency_ms={:.2}",
        elapsed.as_secs_f64() * 1000.0,
    );
}

pub fn log_db_request(worker: &str, ok: bool, elapsed: Duration) {
    #[cfg(debug_assertions)]
    eprintln!(
        "[keynova][obs] db.request worker={worker} ok={ok} latency_ms={:.2}",
        elapsed.as_secs_f64() * 1000.0,
    );
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeBaseline {
    pub idle_memory_bytes: Option<u64>,
    pub idle_cpu_percent: Option<f64>,
    pub production_build_size_bytes: Option<u64>,
}

/// Records an idle runtime baseline after startup without blocking setup.
pub fn spawn_idle_baseline_probe() {
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(60));
        let baseline = sample_runtime_baseline(Duration::from_millis(500));
        #[cfg(debug_assertions)]
        eprintln!(
            "[keynova][obs] runtime.baseline idle_memory_bytes={} idle_cpu_percent={} production_build_size_bytes={}",
            baseline
                .idle_memory_bytes
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".into()),
            baseline
                .idle_cpu_percent
                .map(|v| format!("{v:.2}"))
                .unwrap_or_else(|| "-".into()),
            baseline
                .production_build_size_bytes
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".into()),
        );
    });
}

pub fn sample_runtime_baseline(sample_window: Duration) -> RuntimeBaseline {
    RuntimeBaseline {
        idle_memory_bytes: current_process_memory_bytes(),
        idle_cpu_percent: current_process_cpu_percent(sample_window),
        production_build_size_bytes: current_exe_size(),
    }
}

fn current_exe_size() -> Option<u64> {
    std::env::current_exe()
        .ok()
        .and_then(|path| std::fs::metadata(path).ok())
        .map(|metadata| metadata.len())
}

#[cfg(target_os = "windows")]
fn current_process_memory_bytes() -> Option<u64> {
    use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows::Win32::System::Threading::GetCurrentProcess;

    unsafe {
        let process = GetCurrentProcess();
        let mut counters = PROCESS_MEMORY_COUNTERS::default();
        GetProcessMemoryInfo(
            process,
            &mut counters,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        )
        .ok()?;
        Some(counters.WorkingSetSize as u64)
    }
}

#[cfg(not(target_os = "windows"))]
fn current_process_memory_bytes() -> Option<u64> {
    None
}

#[cfg(target_os = "windows")]
fn current_process_cpu_percent(sample_window: Duration) -> Option<f64> {
    let first = process_times_100ns()?;
    let started = Instant::now();
    std::thread::sleep(sample_window);
    let second = process_times_100ns()?;
    let elapsed = started.elapsed().as_secs_f64();
    if elapsed <= 0.0 {
        return None;
    }
    let cpu_time = (second.saturating_sub(first) as f64) / 10_000_000.0;
    let cores = std::thread::available_parallelism()
        .map(|n| n.get() as f64)
        .unwrap_or(1.0);
    Some((cpu_time / elapsed / cores * 100.0).max(0.0))
}

#[cfg(not(target_os = "windows"))]
fn current_process_cpu_percent(_sample_window: Duration) -> Option<f64> {
    None
}

#[cfg(target_os = "windows")]
fn process_times_100ns() -> Option<u64> {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

    unsafe {
        let process = GetCurrentProcess();
        let mut creation = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user).ok()?;
        Some(filetime_to_u64(kernel).saturating_add(filetime_to_u64(user)))
    }
}

#[cfg(target_os = "windows")]
fn filetime_to_u64(filetime: windows::Win32::Foundation::FILETIME) -> u64 {
    ((filetime.dwHighDateTime as u64) << 32) | filetime.dwLowDateTime as u64
}
