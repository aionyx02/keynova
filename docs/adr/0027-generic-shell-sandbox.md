# Generic Shell Tool Sandbox Requirement

**狀態：** 接受（research phase 完成 — product 尚未解封）  
**日期：** 2026-05-09  
**決策者：** 開發者  
**相關文件：**
- docs/architecture.md
- docs/security.md
- docs/adr/0016-agent-read-only-tools.md
- docs/adr/0023-react-tool-schema-foundation.md

---

## 1. Context（技術背景）

approval + regex/string deny list 不足以安全執行 generic shell。需要真實 platform sandbox 才能解封。目前 Agent tool set 為 narrow typed tools，不暴露 `execute_shell_command` / `execute_bash_command`。

## 2. Constraints（系統限制與邊界）

- Windows：Job Object 可滿足，但網路隔離需 AppContainer（尚未實作）
- Linux：`bwrap` 可滿足，但 seccomp filter 尚未編譯；`bwrap` 缺失時必須明確拒絕（不 fallback 到 unsandboxed）
- macOS：`sandbox-exec` deny-by-default profile 可滿足，但已被 Apple 標記 deprecated；長期需評估 App Sandbox entitlement
- `output_cap_bytes` 截斷保護：caller 須接 `core::agent_observation` redaction

## 3. Alternatives Considered（替代方案分析）

### 方案 A：Approval + deny list only

**優點：**
- 快速實作，無需 platform sandbox

**缺點：**
- 正規表達式過濾無法覆蓋所有 shell bypass 技巧
- 無法防止沙盒逃逸（如 `$(cmd)` 替換）

### 方案 B：Platform sandbox（Job Object / bwrap / sandbox-exec）

**優點：**
- 更強的隔離保證
- 即使 deny list 漏掉，OS 層限制兜底

**缺點：**
- 各平台實作不同，需額外測試
- macOS `sandbox-exec` 已 deprecated

## 4. Decision（最終決策）

選擇：**方案 B — Platform sandbox**（先研究，product 尚未解封）

Keynova 不暴露 generic `execute_shell_command` / `execute_bash_command` Agent tool，直到各平台真實 sandbox 存在。第一批 ReAct tools 為 narrow typed tools（固定 binary + structured args）。

原因：
- 安全隔離比功能完整性優先（參見 docs/claude.md §1 安全優先級）
- deny list 無法完全覆蓋 shell bypass

犧牲：
- generic shell tool 無法在 MVP 使用

Feature flag：各平台 `sandbox_available()` 三態（`Available | BinaryMissing | Unsupported`）

Migration 需款：N/A

Rollback 需款：N/A（不解封即為安全狀態）

## 5. Research Findings（2026-05-09，`managers/sandbox_manager.rs`）

| 平台 | 機制 | 狀態 | 剩餘工作 |
|------|------|------|----------|
| Windows | Job Object（`CreateJobObjectW` + `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`）+ `WaitForSingleObject` timeout | 5/5 tests pass | AppContainer 網路隔離 |
| Linux | `bwrap --unshare-net --unshare-pid --unshare-ipc`；缺 bwrap 時明確拒絕 | 可行 | seccomp filter compilation；bwrap CI check |
| macOS | `sandbox-exec` deny-by-default `.sb` temp profile（per invocation，執行後刪除）| 可行（deprecated API）| App Sandbox entitlement 評估 |
| All | `output_cap_bytes` 截斷保護；caller 須接 `core::agent_observation` redaction | 實作完成 | 全平台 ReAct loop 整合測試 |

## 6. Consequences（系統影響與副作用）

### 正面影響
- 所有平台禁止 `sh -c`、`cmd /c`、shell-string 執行作為 generic Agent tool
- Denied tool attempts 作為 observation 回傳 LLM，讓模型 self-correct 到 typed read-only tools

### 負面影響 / 技術債
- generic shell tool 解封前，部分使用場景需手動操作
- `sandbox_available()` 回傳 `Available | BinaryMissing | Unsupported` 三態，需前端適當顯示

### 對安全性的影響
- 不解封即為安全狀態
- Linux `bwrap` 缺失時明確拒絕，不 fallback 到 unsandboxed

## 7. Implementation Plan（實作計畫）

Research phase 完成（`managers/sandbox_manager.rs`）。Product 解封需完成各平台剩餘工作（見 Open Questions）。

## 8. Rollback Plan（回滾策略）

N/A — 目前狀態即為安全（不解封）。若解封後需 rollback，設定 `agent.shell_sandbox = disabled` 拒絕所有 shell tool 呼叫。

## 9. Validation Plan（驗證方式）

| 測試類型 | 覆蓋目標 | 指令 |
|---------|---------|------|
| Unit test | sandbox_available() 三態 | `cargo test -- sandbox_manager` |
| Integration test | Windows Job Object 5 test cases | `cargo test -- sandbox_windows` |
| Security test | shell bypass 測試 | 手動 + 程式碼審查 |

## 10. Open Questions（未解問題）

- [ ] Windows AppContainer 或 network job restriction 何時實作？
- [ ] Linux seccomp filter 是否需要 CI pre-compilation？
- [ ] macOS App Sandbox entitlement path 是否可替代 `sandbox-exec`？
- [ ] 全平台 integration test 在真實 ReAct loop 下的計畫？