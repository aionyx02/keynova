<!-- 感謝貢獻！送出前請填寫以下內容，並完成檢查清單。 -->

## 變更說明

<!-- 這個 PR 做了什麼、為什麼。 -->

## 關聯 issue

<!-- 例如 Closes #123；沒有則填「無」。 -->

Closes #

## 變更類型

<!-- 勾選適用項目（在 [ ] 中填 x）。 -->

- [ ] Bug 修復
- [ ] 新功能 / 功能改進
- [ ] 重構（無行為變更）
- [ ] 文件
- [ ] 建置 / CI / 雜項

## 檢查清單

- [ ] `npm run verify` 通過（docs + frontend + Rust）。
- [ ] `npm run lint` 與 `npm run rust:clippy`（`-D warnings`）乾淨。
- [ ] 相關測試已新增 / 更新並通過。
- [ ] 文件已依路由更新，並跑過 `npm run docs:refresh`。
- [ ] 涉及架構 / 安全 / IPC contract 變更者，已先走 ADR（見 `docs/decisions.md`）。
- [ ] 符合開發原則：不新增 generic shell、不把 AI 放回產品核心、不繞過 approval / confirmation boundary、不讓單一 component / handler 變成 god module。

## 驗證方式

<!-- 你如何驗證這個變更？手動步驟、測試、截圖等。 -->