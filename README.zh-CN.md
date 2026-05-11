# VRCText

> 极简、轻量的 VRChat OSC 聊天框发送器 + 可选 AI 语音朗读。单 exe 约 **20 MB**，双击即开。

[English](./README.md) · [中文](./README.zh-CN.md)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/congyoua/VRCText?include_prereleases&color=blue)](https://github.com/congyoua/VRCText/releases)
[![Downloads](https://img.shields.io/github/downloads/congyoua/VRCText/total?color=brightgreen)](https://github.com/congyoua/VRCText/releases)
[![Stars](https://img.shields.io/github/stars/congyoua/VRCText?style=social)](https://github.com/congyoua/VRCText/stargazers)
![Windows](https://img.shields.io/badge/Windows%2010%2F11-0078D6?logo=windows&logoColor=white)

依赖系统自带的 WebView2 Runtime（Win11 默认安装，Win10 大多数也已有）。

---

## 截图

<!-- TODO: 准备好真实截图 / GIF 后替换下面的占位 -->
<!-- 建议：docs/demo.gif，约 600px 宽，<2 MB，可用 ScreenToGif 录制 -->

<details><summary>预览（占位 — 待替换为真实截图）</summary>

```
┌────────────────────────────────────────────┐
│ VRCText  ● OSC 127.0.0.1:9000    📌 🔔 🔊 ⚙│
├────────────────────────────────────────────┤
│  14:32    你好，大家                  [↺] │
│  昨天 20:15  今天天气怎么样？         [↺] │
├────────────────────────────────────────────┤
│ ┌────────────────────────────────────────┐│
│ │ 输入消息… [Enter] 发送                  ││
│ └────────────────────────────────────────┘│
│ 12/144    已发送                  [发送 ⏎] │
└────────────────────────────────────────────┘
```

</details>

---

## 功能

- 打字 → **头顶气泡**（通过 VRChat OSC，绕过游戏内软键盘）
- 打字 → **语音朗读**，两档引擎：
  - **SAPI** — Windows 自带，零下载，启动即用
  - **AI 引擎** — 可选升级，应用内按需下载模型：
    - **Matcha 中文（zh-baker）** — ~85 MB，中文单音色
    - **Kokoro 多语言 v1.1** — ~350 MB（全精度），103 个声音，覆盖**中文、英文、日文、韩文、法文**
- 可选 **输出设备**（配合虚拟音频线缆路由到 VRChat 麦克风）
- **历史记录**：悬停重发（短按追加到输入框，长按 0.5 秒直接重发）
- **打字指示器**、窗口置顶、深色 UI、配置持久化
- **双语 UI**：中文 + 英文，设置里随时切换
- **无遥测、无账号、无联网**（除了一次性 AI 模型下载）

---

## 安装

1. 从 [Releases](https://github.com/congyoua/VRCText/releases) 下载 `vrctext.exe`
2. 双击运行（无需安装，无需管理员权限）
3. VRChat → `Settings → OSC → Enabled`

> 首次打开时 Windows SmartScreen 会提示"已保护你的电脑" → 点 **更多信息 → 仍要运行**。exe 未做代码签名（无 Authenticode 证书），独立开发者工具常见情况。

---

## 使用

### 发送文字到头顶

在输入框打字，`Enter` 发送 / `Shift+Enter` 换行。头顶气泡立即出现。

### 语音朗读

⚙ 设置 → **引擎** 选 **SAPI** 或 **AI 引擎**。

- **SAPI**：系统语音直接能用。`语音 / 语言` 下拉选 Huihui (zh-CN) / Zira / David 等；更多语音在 *Windows 设置 → 时间和语言 → 语音 → 管理语音* 里加。
- **AI 引擎**：首次使用点 **下载**，按需选包：
  - **Matcha (~85 MB)** —— 仅中文
  - **Kokoro (~350 MB)** —— 多语言，103 个声音
  下载到 `%APPDATA%\vrctext\vrctext\data\models\`。不再需要时点 **删除已下载模型** 回收空间。

点 **🔈 试听** 不用发送就能预听。

### 路由到 VRChat 麦克风

VRChat 把麦克风当输入；要把 TTS 音频喂进去，需要一条虚拟音频线缆（Windows 不自带）：

1. 装 [**VB-Audio Virtual Cable**](https://vb-audio.com/Cable/)（免费）
2. VRCText → ⚙ → **输出设备** 选 `CABLE Input`，打开 🔊
3. VRChat → 麦克风输入 选 `CABLE Output`
4. *（可选）* `CABLE Output` 属性 → **聆听** → 勾选"聆听此设备"，自己也能听到

### 历史记录

| 操作 | 效果 |
|---|---|
| 悬停某条 | 右侧出现 `↺` 按钮 |
| 短按 `↺` | 内容追加到当前输入框光标处 |
| 长按 `↺` 0.5 秒 | 直接重发（带进度条，不动输入框） |
| 输入框为空时 `↑` / `↓` | 翻阅最近消息 |

上限 50 条，超出自动丢弃最早的。⚙ → **清空全部历史**（两步确认）。

---

## 配置 / 数据文件位置

```
%APPDATA%\vrctext\vrctext\config\config.toml    # 设置
%APPDATA%\vrctext\vrctext\data\models\          # AI 模型（按需下载）
```

> 嵌套的 `vrctext\vrctext` 不是 typo —— 这是 [`directories`](https://crates.io/crates/directories) crate 从 `ProjectDirs::from("dev", "vrctext", "vrctext")` 生成的标准布局。删对应文件即可重置或回收空间。

---

## 已知限制

- **仅 Windows**（用了 SAPI / WaveOut / winsock / IMM32 直接调用）
- 需要系统已安装 **Microsoft WebView2 Runtime**（Win11 默认有；部分老版本 Win10 没有，启动失败时去 [microsoft.com/edge/webview2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) 装一下 —— VRCText 检测不到会弹出双语提示）
- 不自带虚拟音频线缆（见上面 VB-Cable 段）
- **未签名 `.exe`**，SmartScreen 会提示（无害，可跳过）
- **AI 引擎首次使用需联网下载模型**，之后完全离线运行

---

## 从源码构建

技术栈：**Tauri 2**（Rust 后端）+ **Svelte 5** + **TypeScript** + **Vite**。

**前置依赖**
- [Rust stable](https://rustup.rs/)，加上 `x86_64-pc-windows-msvc` target
- Node.js 18+
- MSVC C++ 构建工具（Visual Studio Build Tools，提供 `link.exe`）

```sh
# 一次性
rustup target add x86_64-pc-windows-msvc
npm install

# 开发（热重载，加 --features devtools 可开 DevTools）
npm run tauri dev

# 出包（必须用 tauri-cli，不要直接 cargo build）
npm run tauri build -- --no-bundle
# 产物在 src-tauri\target\release\vrctext.exe
```

`bundle.active: false` 让构建跳过 NSIS / MSI，只产出独立 `.exe`。如需安装包，在 `src-tauri/tauri.conf.json` 改 `"active": true` 并把 `"targets"` 设为 `"nsis"`。

> **注意**：必须用 `npm run tauri build`（走 tauri-cli）来出 release。直接 `cargo build --release` 会让 WebView 仍然指向 `devUrl: http://localhost:1420`，运行时显示空白页 —— 因为 `cargo build` 不参与 tauri-cli 的 dev/release URL 切换逻辑。
>
> 想绕过 tauri-cli 出 .exe 的话，要么去掉 `devUrl`，要么在 `tauri.conf.json` 的 `app.windows[0]` 里写 `"url": "index.html"` 显式强制 frontendDist。
>
> 另：Svelte 5 的 runes（`$state` / `$derived` / `$effect`）只在 `.svelte` 和 `.svelte.ts` / `.svelte.js` 文件里被编译。普通 `.ts` 文件里写 `$state(...)` 在 runtime 会报 `ReferenceError`。这就是为什么 store 文件命名为 [`src/lib/stores.svelte.ts`](src/lib/stores.svelte.ts) 而非 `.ts`。

---

## 致谢

- [Tauri](https://tauri.app/) —— 让二进制保持轻量的桌面运行时
- [Svelte](https://svelte.dev/) —— UI 框架
- [Sherpa-ONNX](https://github.com/k2-fsa/sherpa-onnx) —— 端上神经 TTS
- [VB-Audio](https://vb-audio.com/Cable/) —— 麦克风路由的虚拟线缆

---

## License

[MIT](./LICENSE) © 2026 congyoua
