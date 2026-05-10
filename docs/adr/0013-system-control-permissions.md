# System Control 權限範圍

**狀態：** 接受  
**日期：** 2026-05-03  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md
- docs/security.md

---

## 1. Context（技術背景）

需定義 Keynova 可執行的系統控制操作（音量、亮度、WiFi），同時避免需要管理員權限或 UAC 提示。

## 2. Constraints（系統限制與邊界）

- 亮度 API（IOCTL）需要管理員權限，不適合標準使用者環境
- WiFi 切換需要 UAC；MVP 僅顯示狀態
- Tauri 沙盒不需特別開放（音量/亮度由 Rust backend 處理，非 WebView API）
- 非 Windows 平台需 stub 或 NotImplemented

## 3. Alternatives Considered（替代方案分析）

### 方案 A：Win32 API + PowerShell WMI

**優點：**
- 音量：`IAudioEndpointVolume` COM API（無需管理員）
- 亮度：PowerShell WMI 呼叫（標準使用者可執行）
- WiFi：`netsh wlan show interfaces`（僅顯示）

**缺點：**
- PowerShell spawn 有延遲（亮度讀取稍慢）

### 方案 B：需要管理員的 IOCTL 直接存取

**優點：**
- 更直接、更快

**缺點：**
- 需要 UAC 提示，使用者體驗差
- 不符合標準桌面 App 行為

## 4. Decision（最終決策）

選擇：**方案 A — Win32 API + PowerShell WMI**

原因：
- 無需管理員權限，使用者體驗好
- 標準 Windows API，相容性好

犧牲：
- PowerShell spawn 有延遲（亮度操作）
- WiFi 僅顯示狀態，無法切換

Feature flag：N/A

Migration 需求：N/A

Rollback 需求：N/A

## 5. Consequences（系統影響與副作用）

### 正面影響
- 所有操作無需管理員權限
- `managers/system_manager.rs`：Windows 實作 + stub

### 對安全性的影響
- PowerShell spawn 路徑固定，不接受使用者自訂命令
- 所有平台特定實作包在 `#[cfg(target_os)]`，非 Windows 回傳 `NotImplemented`

### handlers/system_control.rs 提供的介面
- namespace `"system"`，含 `volume.get/set/mute`、`brightness.get/set`、`wifi.info`

## 6. Implementation Plan（實作計畫）

已實作完成（Phase 3）。

## 7. Rollback Plan（回滾策略）

N/A — 平台特定實作，不影響其他模組。

## 8. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Unit test | volume/brightness stub | `cargo test -- system_manager` |
| Manual validation | Windows 音量控制 | `npm run tauri dev` |

## 9. Open Questions（未解問題）

- 無