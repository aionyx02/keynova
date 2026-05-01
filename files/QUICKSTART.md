# ⚡ Keynova 快速開始指南

只需 5 分鐘，讓你開始使用 Keynova！

---

## 📥 安裝

### Windows
```bash
# 下載最新版本
# https://github.com/your-name/keynova/releases

# 運行安裝程序
Keynova-Setup.exe

# 安裝完成後自動運行
```

### Linux
```bash
# Ubuntu/Debian
sudo apt install ./keynova_x.x.x_amd64.deb

# Fedora/RHEL
sudo rpm -i keynova_x.x.x_x86_64.rpm

# 或直接運行
./keynova-x.x.x.AppImage
```

### macOS
```bash
# 下載 DMG
# https://github.com/your-name/keynova/releases

# 或使用 Homebrew
brew install keynova
```

---

## 🎯 基礎使用

### 第一步：啟動應用

安裝後，Keynova 會在後台運行。

按下設定的啟動快捷鍵（默認 **Win+K**）即可開啟應用啟動器。

### 第二步：打開應用

```
按下 Win+K
輸入應用名（如 "chrome", "vscode", "notepad"）
按 Enter 或 ↓↑ 選擇，再 Enter 打開
```

### 第三步：嘗試其他功能

| 按鍵 | 功能 | 嘗試 |
|------|------|------|
| `Ctrl+Alt+T` | 打開終端 | 輸入命令試試 |
| `Ctrl+P` | 搜索文件 | 搜索你的文件 |
| `Ctrl+=` | 計算器 | 計算 100+50 |
| `Win+Shift+P` | 直接命令 | 輸入 `shutdown` |
| `WASD` | 控制鼠標 | 移動滑鼠試試 |

---

## ⚙️ 初始配置

### 修改快捷鍵

1. **打開配置文件**
   ```bash
   # Windows
   C:\Users\YourName\AppData\Roaming\Keynova\config.toml
   
   # Linux
   ~/.config/Keynova/config.toml
   
   # macOS
   ~/Library/Application Support/Keynova/config.toml
   ```

2. **編輯快捷鍵**
   ```toml
   [hotkeys]
   app_launcher = "Win+K"           # 改為你喜歡的快鍵
   terminal = "Ctrl+Alt+T"
   file_search = "Ctrl+P"
   calculator = "Ctrl+="
   # ... 等等
   ```

3. **保存文件並重啟 Keynova**

### 啟用/禁用功能

在 `config.toml` 中修改：

```toml
[features]
enable_ai = true                 # 啟用 AI 助手
enable_clipboard_sync = false    # 禁用剪貼簿同步
enable_virtual_workspaces = true # 啟用虛擬工作區
```

---

## 📚 常用快捷鍵

### 應用層面

| 快捷鍵 | 功能 | 備註 |
|--------|------|------|
| `Win+K` | 應用啟動 | **最常用** |
| `Ctrl+Alt+T` | 打開終端 | **次常用** |
| `Ctrl+P` | 文件搜索 | 快速找文件 |
| `Win+Shift+P` | 直接命令 | 執行系統命令 |

### 工具層面

| 快捷鍵 | 功能 |
|--------|------|
| `Ctrl+=` | 計算器 |
| `Ctrl+Shift+V` | 剪貼簿歷史 |
| `Win+S` | 系統控制面板 |

### 終端特定

在終端浮窗中：

| 快捷鍵 | 功能 |
|--------|------|
| `Ctrl+Tab` | 下一個標籤 |
| `Ctrl+Shift+Tab` | 上一個標籤 |
| `Ctrl+T` | 新建標籤 |
| `Ctrl+W` | 關閉標籤 |

### 鼠標控制

| 快捷鍵 | 功能 |
|--------|------|
| `WASD` 或 `↑↓←→` | 移動鼠標 |
| `Space + ↑↓←→` | 加速移動 |
| `Ctrl + ↑↓←→` | 細微移動 |
| `Enter` | 左鍵點擊 |
| `Alt + Enter` | 右鍵點擊 |

---

## 🆘 常見問題

### 快捷鍵不工作？

1. 檢查配置文件語法
2. 確保 Keynova 在運行
3. 在終端中查看錯誤日誌：
   ```bash
   Keynova --debug
   ```

### 應用找不到？

1. 確保應用已安裝
2. 嘗試輸入完整應用名
3. 清除應用索引：在設置中點擊"重建索引"

### Everything 搜索無法工作？

**Windows 用戶**：
- 確保已安裝 [Everything](https://www.voidtools.com/)
- 確保 Everything 服務在運行
- 如不想使用，編輯 `config.toml` 禁用

### 終端無法使用？

檢查 PTY 支持：
```bash
# Windows
echo $TERM

# Linux/macOS
echo $SHELL
```

如仍有問題，在 Issue 中報告。

---

## 🚀 下一步

### 推薦閱讀

1. **[完整 README](./README.md)** - 了解所有功能
2. **[項目計劃書](./Keynova_項目計劃書_v2.0.md)** - 了解項目方向
3. **[代碼架構](./全鍵控制系統_程序代碼架構.md)** - 深入技術細節（開發者）

### 常用命令

```bash
# 重啟 Keynova
Keynova --restart

# 顯示調試信息
Keynova --debug

# 重建索引
Keynova --rebuild-index

# 重置配置
Keynova --reset-config
```

### 進階技巧

1. **別名設置** - 在配置中添加別名，加快應用啟動
   ```toml
   [aliases]
   code = "Visual Studio Code"
   py = "Python"
   ```

2. **自訂命令** - 添加自訂快捷命令
   ```toml
   [commands]
   "restart-explorer" = "taskkill /F /IM explorer.exe && explorer"
   ```

3. **文件搜索過濾** - 在文件搜索中使用過濾語法
   ```
   Ctrl+P 後輸入：
   type:file *.pdf           搜索 PDF 文件
   path:documents            搜索 documents 文件夾
   size:>10mb                搜索大於 10MB 的文件
   ```

---

## 📞 需要幫助？

- 📖 查看 [完整文檔索引](./文檔索引.md)
- ❓ 查看 [FAQ](./FAQ.md)
- 🐛 [報告 Bug](https://github.com/your-name/keynova/issues)
- 💬 [提問](https://github.com/your-name/keynova/discussions)

---

## ✨ 提示

**💡 快速提升效率的三個秘訣：**

1. **掌握三個核心快鍵**
   - `Win+K` - 應用啟動
   - `Ctrl+Alt+T` - 終端
   - `Ctrl+P` - 文件搜索

2. **設置常用別名**
   ```toml
   [aliases]
   v = "Visual Studio Code"
   c = "Google Chrome"
   ```

3. **學會鍵盤鼠標控制**
   - WASD 移動
   - Space 加速
   - 很快你就不用實體鼠標了！

---

**祝你使用愉快！** 🎉

有任何問題或建議，歡迎反饋！

