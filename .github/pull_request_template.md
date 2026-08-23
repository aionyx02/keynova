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

- [ ] `npm run verify` 通過（frontend + Rust）。
- [ ] `npm run lint` 與 `npm run rust:clippy`（`-D warnings`）乾淨。
- [ ] 相關測試已新增 / 更新並通過。
- [ ] 文件：預設不需要更新。只有當這個 PR 產生了程式碼裡讀不到的資訊
      （為什麼這樣選、踩到的雷、未完成的坑、repo 外的狀態）才寫進
      `docs/decisions.md` 或 `docs/state.md`。
- [ ] 觸及 `docs/security.md` 的「需要先立 ADR」清單者，已先有 ADR。
- [ ] 改到 `CLAUDE.md` 或任何 agent 組態者：這是**權限變更**，逐條審。
- [ ] 符合開發原則：不新增 generic shell、不把 AI 放回產品核心、不繞過 approval / confirmation boundary、不讓單一 component / handler 變成 god module。

## 驗證方式

<!-- 你如何驗證這個變更？手動步驟、測試、截圖等。 -->
