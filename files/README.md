# Keynova 🚀

> **90%+ 無鼠標工作流的終極鍵盤控制系統**

全鍵盤操控桌面環境，超越 Flow Launcher 和 Alfred。讓開發者的手指永遠停留在鍵盤上。

---

## 📌 核心願景

**讓開發者 90%+ 的操作無需鼠標**

傳統應用啟動器只解決了應用搜索問題，Keynova 走得更遠：

- ⌨️ **完整鍵盤控制** - 應用啟動、終端、文件搜索、鼠標操控，全部鍵盤完成
- 🎯 **開發者專注** - 專為編程工作流設計，集成 AI 助手、虛擬工作區、浮窗管理
- ⚡ **閃電般速度** - Tauri 原生應用，輕量 50MB，快速啟動
- 🔌 **插件生態** - v3.0+ 支持 JavaScript/Python 擴展
- 🌍 **跨平台** - Windows/Linux/macOS 完整支持

---

## ✨ 核心功能

### 🔍 快速應用啟動 (Win+K)
```
按下 Win+K → 輸入應用名 → 回車啟動
支持：模糊搜索、別名、快捷鍵直達
```

### 💻 終端浮窗 (Ctrl+Alt+T)
```
Tabby 風格的內置終端
不依賴系統終端，獨立進程，支持：
  • 標籤頁管理
  • 分割視圖
  • 快速命令執行
  • PTY 完整支持
```

### 🖱️ 鍵盤控制鼠標 (WASD/方向鍵)
```
WASD 或方向鍵：移動光標
Space + 方向鍵：加速移動
Ctrl + 方向鍵：細微移動
滑輪：Shift + 滾輪鍵
```

### 📂 文件搜索 (Ctrl+P)
```
Windows: Everything IPC (秒級搜索) + tantivy 備選
Linux/macOS: tantivy 索引
```

### 🧮 快速計算器 (Ctrl+=)
```
支持：
  • 基本四則運算
  • 單位轉換（長度、重量、溫度等）
  • 進制轉換（16進制、二進制等）
```

### 📋 剪貼簿管理 (Ctrl+Shift+V)
```
• 歷史記錄查看
• 圖片預覽
• 快速粘貼
• 同步支持（v2.0+）
```

### 🎮 虛擬工作區 (Ctrl+Alt+1/2/3)
```
組織應用和窗口
快速切換工作環境
```

### 🤖 AI 助手 (Ctrl+Shift+A) [v2.0+]
```
集成 Claude API
• 代碼解釋
• 文檔生成
• 快速提問
```

### 📝 快速筆記 (Win+N) [v2.0+]
```
• Markdown 編輯
• Notion 同步
• 即時保存
```

---

## 🏗️ 技術架構

### 技術棧

| 層級 | 技術 | 說明 |
|------|------|------|
| **框架** | Tauri 2.x | 輕量原生應用框架 |
| **前端** | React 18 + TypeScript | 現代 UI 框架 |
| **後端** | Rust | 高效後端邏輯 |
| **終端** | xterm.js + PTY | 完整終端支持 |
| **搜索** | tantivy / Everything | 文件索引 |
| **狀態管理** | Zustand | 前端狀態 |
| **配置** | TOML + SQLite | 配置存儲 |

### 核心特性

- **IPC 通訊** - Tauri 命令通道，40+ RPC 接口
- **全局熱鍵** - 使用 `rdev` 庫實現
- **鼠標控制** - `enigo` 跨平台鼠標操控
- **平台適配** - Win32 API / X11 / CGEventTap

---

## 📦 開發階段

### Phase 1: MVP (v0.1) - 4-6 週
- ✅ 應用啟動器 + 別名系統
- ✅ 終端浮窗
- ✅ 鍵盤鼠標控制
- ✅ 熱鍵配置 UI

### Phase 2: v1.0 - 4-5 週
- ✅ 文件搜索 (Everything/tantivy)
- ✅ 計算器
- ✅ 剪貼簿管理
- ✅ 系統控制面板
- ✅ 命令歷史

### Phase 3: v2.0 - 5-6 週
- ✅ AI 助手集成
- ✅ 虛擬工作區
- ✅ 快速筆記
- ✅ 浮窗標籤頁

### Phase 4: v3.0+ - 6+ 週
- ⏳ 插件系統
- ⏳ 雲端配置
- ⏳ API 暴露

---

## 🚀 快速開始

### 系統要求

- **Windows 10/11** (64-bit)
- **Linux** (X11/Wayland)
- **macOS 11+** (Intel/Apple Silicon)
- **Node.js 18+**
- **Rust 1.70+**

### 安裝

```bash
# 克隆項目
git clone https://github.com/your-name/keynova.git
cd keynova

# 安裝依賴
npm install

# 開發模式運行
npm run tauri dev

# 生成安裝程序
npm run tauri build
```

### 基礎配置

編輯 `~/.config/keynova/config.toml`：

```toml
[hotkeys]
app_launcher = "Win+K"           # 應用啟動
terminal = "Ctrl+Alt+T"          # 終端
file_search = "Ctrl+P"           # 文件搜索
calculator = "Ctrl+="            # 計算器
clipboard = "Ctrl+Shift+V"       # 剪貼簿
direct_command = "Win+Shift+P"   # 命令執行

[features]
enable_ai = true                 # AI 助手
enable_clipboard_sync = false    # 剪貼簿同步
enable_virtual_workspaces = true # 虛擬工作區

[appearance]
theme = "dark"                   # dark | light | auto
font_size = 13
```

---

## ⌨️ 快捷鍵速查

| 快捷鍵 | 功能 | 備註 |
|--------|------|------|
| `Win+K` | 應用啟動 | 模糊搜索 |
| `Ctrl+Alt+T` | 終端浮窗 | 獨立進程 |
| `Ctrl+P` | 文件搜索 | Everything/tantivy |
| `Ctrl+=` | 計算器 | 單位轉換支持 |
| `Ctrl+Shift+V` | 剪貼簿 | 歷史記錄 |
| `Win+S` | 系統控制 | 音量/亮度/WiFi |
| `Win+Shift+P` | 直接命令 | 執行系統命令 |
| `Ctrl+Alt+1/2/3` | 工作區切換 | v2.0+ |
| `Ctrl+Shift+A` | AI 助手 | v2.0+ |
| `Win+N` | 快速筆記 | v2.0+ |
| `WASD` / `↑↓←→` | 移動鼠標 | 鍵盤控制模式 |

---

## 📊 與競品對比

| 特性 | Keynova | Flow Launcher | Alfred | Raycast |
|------|---------|---------------|--------|---------|
| **跨平台** | ✅ Win/Linux/Mac | ❌ Windows only | ❌ macOS only | ❌ macOS only |
| **鍵盤鼠標控制** | ✅ 完整 | ❌ 無 | ❌ 無 | ❌ 無 |
| **內置終端** | ✅ PTY 完整 | ⚠️ 外部調用 | ❌ 無 | ❌ 無 |
| **虛擬工作區** | ✅ 支持 | ❌ 無 | ❌ 無 | ❌ 無 |
| **浮窗管理** | ✅ 標籤+分割 | ❌ 無 | ❌ 無 | ❌ 無 |
| **AI 集成** | ✅ Claude | ⚠️ 部分 | ⚠️ 部分 | ✅ OpenAI |
| **開源** | ✅ MIT | ✅ MIT | ❌ 商業 | ❌ 商業 |
| **無鼠標度** | ⭐⭐⭐⭐⭐ 90%+ | ⭐⭐⭐ 60% | ⭐⭐⭐ 60% | ⭐⭐⭐ 65% |

---

## 🎯 使用場景

### 開發編程工作流
```
9/10 可行度

快速打開編輯器 → 終端執行命令 → 文件搜索定位 
→ AI 助手查詢文檔 → 虛擬工作區組織 
→ 鍵盤完成所有操作 ✨
```

### 文檔寫作
```
8/10 可行度

Win+N 快速筆記 → Ctrl+P 搜索資料 
→ Ctrl+= 計算數據 → AI 潤色文本 
→ 鍵盤完成 80%+ 操作
```

### 網路瀏覽
```
6/10 可行度

需配合 Vimium 等瀏覽器插件
Keynova 負責應用和文件層面
```

---

## 📚 文檔

| 文檔 | 內容 | 用途 |
|------|------|------|
| [項目計劃書](./Keynova_項目計劃書_v2.0.md) | 開發時間表、預算、風險評估 | 項目管理 |
| [代碼架構](./全鍵控制系統_程序代碼架構.md) | 完整目錄結構、RPC 接口、流程圖 | 開發參考 |
| [名稱驗證](./全鍵控制系統_名稱驗證報告.md) | 品牌名驗證、競品分析 | 品牌確認 |

---

## 🤝 貢獻指南

我們歡迎任何形式的貢獻！

```bash
# 1. Fork 本倉庫
# 2. 創建特性分支
git checkout -b feature/your-feature

# 3. 提交更改
git commit -am 'Add new feature'

# 4. 推送到分支
git push origin feature/your-feature

# 5. 創建 Pull Request
```

### 開發規範

- 代碼風格：Rust fmt + Prettier
- 提交信息：英文，清晰描述改動
- 測試：所有新功能必須有測試
- 文檔：更新相關文檔

---

## 📋 路線圖

```
2024 Q2  → v0.1 Alpha (基礎功能)
2024 Q3  → v1.0 Release (核心功能完整)
2024 Q4  → v2.0 (AI + 高級功能)
2025 Q1+ → v3.0+ (插件生態)
```

---

## ⚖️ 許可證

MIT License - 自由使用、修改、分發

詳見 [LICENSE](./LICENSE) 文件

---

## 🙋 常見問題

### Q: 必須安裝 Everything 嗎？
**A:** 不必。Windows 上優先使用 Everything（如已安裝），否則自動使用 tantivy 索引。

### Q: 可以禁用某些功能嗎？
**A:** 可以。在 `config.toml` 中設定 `enable_xxx = false` 即可。

### Q: 支持自訂快捷鍵嗎？
**A:** 完全支持。編輯配置文件或在 UI 中設定。

### Q: 可以添加自訂命令嗎？
**A:** v1.0 支持命令別名，v3.0+ 支持插件編寫自訂命令。

### Q: 會定期更新嗎？
**A:** 是的，計劃每月發布一個小版本，每季度一個大版本。

---

## 📧 聯絡方式

- **GitHub Issues**: [報告 Bug 或提功能需求](https://github.com/your-name/keynova/issues)
- **討論區**: [功能討論、使用經驗分享](https://github.com/your-name/keynova/discussions)
- **郵件**: keynova@example.com

---

## 🙏 致謝

感謝以下開源項目的支持：

- [Tauri](https://tauri.app/) - 應用框架
- [tantivy](https://github.com/quickwit-oss/tantivy) - 搜索引擎
- [xterm.js](https://xtermjs.org/) - 終端
- [rdev](https://github.com/enigo-rs/rdev) - 全局熱鍵
- [Zustand](https://github.com/pmndrs/zustand) - 狀態管理

---

**Made with ❤️ for developers who love keyboards**

⭐ 如果這個項目對你有幫助，請給個 Star！

