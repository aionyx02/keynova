# 安全政策 Security Policy

## 支援版本

Keynova 仍在快速迭代（pre-1.0），僅對**最新發布版本**提供安全修復。

| 版本 | 是否支援安全更新 |
| --- | --- |
| 最新 release（目前 0.5.x） | ✅ |
| 更舊版本 | ❌（請先升級到最新版） |

## 回報安全漏洞

**請勿開立公開 issue 來回報安全漏洞。**

請改用 GitHub 的私密回報管道：

1. 到本專案的 **Security** 分頁 →「Report a vulnerability」。
2. 描述漏洞、影響範圍與可重現步驟（若有 PoC 請一併提供）。

我們會盡快確認收到並評估。修復發布前請先不要公開揭露細節（responsible disclosure）。

> 若你的 GitHub 帳號看不到「Report a vulnerability」按鈕（私密回報尚未啟用），
> 可開一個**不含漏洞細節**的 issue 詢問私下聯絡管道，我們再提供。

## 安全模型與邊界

Keynova 的權限邊界、approval gate、沙箱與隱私原則詳見
[docs/security.md](docs/security.md)。重點：

- **Local-first**：預設偏向本機能力，外部服務（AI provider、翻譯、網路）需明確 opt-in。
- **Approval-aware**：高風險／破壞性操作必須可見、可確認、可取消。
- **無 generic shell**：不提供未受沙箱約束的任意 shell 執行。
- 機密類設定值（API 金鑰等）在回傳 UI 前於本機端遮罩。

## 已知限制

目前發布的安裝檔**尚未做 code signing / 公證**，第一次執行時
Windows SmartScreen / macOS Gatekeeper 會警告（見 README 的繞過步驟）。
這不是漏洞，而是已知的待辦項目。