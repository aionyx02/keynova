use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct VolumeInfo {
    pub level: f32,
    pub muted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WifiInfo {
    pub ssid: String,
    pub signal: i32,
    pub connected: bool,
}

/// 系統控制管理器：音量、亮度、WiFi（Windows 優先）。
pub struct SystemManager;

impl SystemManager {
    pub fn new() -> Self {
        Self
    }

    // ─── Volume ──────────────────────────────────────────────────────────────

    pub fn get_volume(&self) -> Result<VolumeInfo, String> {
        platform_get_volume()
    }

    pub fn set_volume(&self, level: f32) -> Result<(), String> {
        let clamped = level.clamp(0.0, 1.0);
        platform_set_volume(clamped)
    }

    pub fn set_mute(&self, muted: bool) -> Result<(), String> {
        platform_set_mute(muted)
    }

    // ─── Brightness ──────────────────────────────────────────────────────────

    /// 取得螢幕亮度（0–100）。
    pub fn get_brightness(&self) -> Result<u32, String> {
        platform_get_brightness()
    }

    /// 設定螢幕亮度（0–100）。
    pub fn set_brightness(&self, level: u32) -> Result<(), String> {
        platform_set_brightness(level.min(100))
    }

    // ─── WiFi ────────────────────────────────────────────────────────────────

    /// 取得當前 WiFi 連線資訊。
    pub fn get_wifi_info(&self) -> Result<Vec<WifiInfo>, String> {
        platform_wifi_info()
    }
}

// ─── Platform implementations ─────────────────────────────────────────────────

#[cfg(target_os = "windows")]
mod win_impl {
    use super::VolumeInfo;
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{
        eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
    };

    fn get_endpoint() -> Result<IAudioEndpointVolume, String> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                    .map_err(|e| e.to_string())?;
            let device = enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .map_err(|e| e.to_string())?;
            device
                .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
                .map_err(|e| e.to_string())
        }
    }

    pub fn get_volume() -> Result<VolumeInfo, String> {
        let ep = get_endpoint()?;
        unsafe {
            let level = ep.GetMasterVolumeLevelScalar().map_err(|e| e.to_string())?;
            let muted = ep.GetMute().map_err(|e| e.to_string())?.as_bool();
            Ok(VolumeInfo { level, muted })
        }
    }

    pub fn set_volume(level: f32) -> Result<(), String> {
        let ep = get_endpoint()?;
        unsafe {
            ep.SetMasterVolumeLevelScalar(level, std::ptr::null())
                .map_err(|e| e.to_string())
        }
    }

    pub fn set_mute(muted: bool) -> Result<(), String> {
        let ep = get_endpoint()?;
        unsafe {
            ep.SetMute(muted, std::ptr::null()).map_err(|e| e.to_string())
        }
    }

    pub fn get_brightness() -> Result<u32, String> {
        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command",
                "(Get-WmiObject -Namespace root/WMI -Class WmiMonitorBrightness).CurrentBrightness"])
            .output()
            .map_err(|e| e.to_string())?;
        let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
        s.parse::<u32>().map_err(|e| e.to_string())
    }

    pub fn set_brightness(level: u32) -> Result<(), String> {
        let script = format!(
            "(Get-WmiObject -Namespace root/WMI -Class WmiMonitorBrightnessMethods).WmiSetBrightness(1, {level})"
        );
        let status = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .status()
            .map_err(|e| e.to_string())?;
        if status.success() { Ok(()) } else { Err("brightness command failed".into()) }
    }

    pub fn wifi_info() -> Result<Vec<super::WifiInfo>, String> {
        // Current connection
        let current_output = std::process::Command::new("netsh")
            .args(["wlan", "show", "interfaces"])
            .output()
            .map_err(|e| e.to_string())?;
        let current_text = String::from_utf8_lossy(&current_output.stdout);

        let current_ssid = current_text.lines()
            .find(|l| l.contains("SSID") && !l.contains("BSSID"))
            .and_then(|l| l.split(':').nth(1))
            .map(str::trim)
            .unwrap_or("")
            .to_string();

        let signal_str = current_text.lines()
            .find(|l| l.contains("Signal"))
            .and_then(|l| l.split(':').nth(1))
            .map(|s| s.trim().trim_end_matches('%'))
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);

        if !current_ssid.is_empty() {
            return Ok(vec![super::WifiInfo {
                ssid: current_ssid,
                signal: signal_str,
                connected: true,
            }]);
        }
        Ok(vec![])
    }
}

#[cfg(not(target_os = "windows"))]
mod stub_impl {
    use super::{VolumeInfo, WifiInfo};

    pub fn get_volume() -> Result<VolumeInfo, String> {
        Err("volume control not implemented on this platform".into())
    }
    pub fn set_volume(_: f32) -> Result<(), String> {
        Err("volume control not implemented on this platform".into())
    }
    pub fn set_mute(_: bool) -> Result<(), String> {
        Err("volume control not implemented on this platform".into())
    }
    pub fn get_brightness() -> Result<u32, String> {
        Err("brightness control not implemented on this platform".into())
    }
    pub fn set_brightness(_: u32) -> Result<(), String> {
        Err("brightness control not implemented on this platform".into())
    }
    pub fn wifi_info() -> Result<Vec<WifiInfo>, String> {
        Err("wifi info not implemented on this platform".into())
    }
}

#[cfg(target_os = "windows")]
fn platform_get_volume() -> Result<VolumeInfo, String> { win_impl::get_volume() }
#[cfg(not(target_os = "windows"))]
fn platform_get_volume() -> Result<VolumeInfo, String> { stub_impl::get_volume() }

#[cfg(target_os = "windows")]
fn platform_set_volume(v: f32) -> Result<(), String> { win_impl::set_volume(v) }
#[cfg(not(target_os = "windows"))]
fn platform_set_volume(v: f32) -> Result<(), String> { stub_impl::set_volume(v) }

#[cfg(target_os = "windows")]
fn platform_set_mute(m: bool) -> Result<(), String> { win_impl::set_mute(m) }
#[cfg(not(target_os = "windows"))]
fn platform_set_mute(m: bool) -> Result<(), String> { stub_impl::set_mute(m) }

#[cfg(target_os = "windows")]
fn platform_get_brightness() -> Result<u32, String> { win_impl::get_brightness() }
#[cfg(not(target_os = "windows"))]
fn platform_get_brightness() -> Result<u32, String> { stub_impl::get_brightness() }

#[cfg(target_os = "windows")]
fn platform_set_brightness(l: u32) -> Result<(), String> { win_impl::set_brightness(l) }
#[cfg(not(target_os = "windows"))]
fn platform_set_brightness(l: u32) -> Result<(), String> { stub_impl::set_brightness(l) }

#[cfg(target_os = "windows")]
fn platform_wifi_info() -> Result<Vec<WifiInfo>, String> { win_impl::wifi_info() }
#[cfg(not(target_os = "windows"))]
fn platform_wifi_info() -> Result<Vec<WifiInfo>, String> { stub_impl::wifi_info() }