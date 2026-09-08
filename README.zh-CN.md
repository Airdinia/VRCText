<p align="center">
  <img src="public/icon.png" width="88" alt="VRCText 图标">
</p>

<h1 align="center">VRCText</h1>

<p align="center">
  <strong>用于 VRChat 的桌面文字聊天工具。</strong><br>
  向聊天框发送文字，复用历史消息，支持本地语音朗读。
</p>

<p align="center">
  <a href="https://github.com/Airdinia/VRCText/releases"><strong>下载 Windows 版</strong></a> ·
  <a href="#快速开始">快速开始</a> ·
  <a href="README.md">English</a> ·
  <a href="https://github.com/Airdinia/VRCText/issues">反馈问题</a>
</p>

<p align="center">
  <a href="https://github.com/Airdinia/VRCText/releases"><img src="https://img.shields.io/github/v/release/Airdinia/VRCText?include_prereleases&amp;color=blue" alt="版本"></a>
  <img src="https://img.shields.io/badge/Windows%2010%2F11-0078D6?logo=windows&amp;logoColor=white" alt="Windows 10/11">
  <a href="LICENSE"><img src="https://img.shields.io/badge/Source%20license-MIT-yellow.svg" alt="源码许可：MIT"></a>
  <a href="https://github.com/Airdinia/VRCText/releases"><img src="https://img.shields.io/github/downloads/Airdinia/VRCText/total?color=brightgreen" alt="下载量"></a>
</p>

---

VRCText 通过 OSC 向 VRChat 聊天框发送文字。输入消息后按 `Enter` 即可发送，也可以开启本地语音朗读。

## 主要功能

- **文字聊天** — `Enter` 发送，`Shift+Enter` 换行，实时显示剩余字符数。
- **消息历史** — 保留最近 50 条消息；点击插入输入框，长按直接重发，也可用方向键浏览历史。
- **本地朗读** — 使用 Windows 已安装的语音，或下载 Matcha（中文）或 Kokoro（中英文）模型，在本机合成语音。
- **音频设置** — 可选择声音和输出设备，支持试听；搭配虚拟音频线缆，可将朗读接入 VRChat 麦克风。
- **界面选项** — 支持窗口置顶、打字指示器、消息提示音开关和中英文界面。
- **本地存储** — 无需账号，设置和历史保存在本地，没有应用遥测；文字聊天无需下载语音模型。

## 快速开始

**运行环境：** Windows 10/11 x64，需要 [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)。

1. **下载并打开。** 从 [Releases](https://github.com/Airdinia/VRCText/releases) 下载 EXE，直接运行。
2. **启用 OSC。** 在 VRChat 中开启 OSC。两者在同一台电脑运行时，保留 VRCText 默认目标 `127.0.0.1:9000`。
3. **发送消息。** 输入文字，按 `Enter`，在 VRChat 中查看聊天框。

每条消息最多 **144 个字符**，超出时会提示。语音朗读默认关闭。

## 语音朗读

先通过窗口顶部的扬声器按钮开启朗读，再打开设置，选择语音引擎：

| 引擎 | 可用语音 | 配置方式 |
| --- | --- | --- |
| **Windows 原生（SAPI）** | Windows 已安装的语音 | 无需额外下载模型 |
| **AI 模型（Matcha / Kokoro）** | Matcha：中文；Kokoro：中英文 | 在应用内下载 Matcha（约 129 MB）或 Kokoro（约 365 MB），下载后在本机合成 |

选择声音和音频输出设备，点击 **试听** 检查效果。之后发送消息时会同时朗读。

**在 VRChat 中使用朗读：** 需要另外安装并配置虚拟音频线缆，将 VRCText 的音频输出接入 VRChat 麦克风。

```text
VRCText 朗读 → 虚拟音频线缆的播放端点 → 录音端点 → VRChat 麦克风输入
```

在 VRCText 中选择线缆的**播放端点**作为输出，在 VRChat 中选择对应的**录音端点**作为麦克风输入。

## 快捷操作

| 操作 | 效果 |
| --- | --- |
| `Enter` | 发送消息 |
| `Shift+Enter` | 插入换行 |
| 输入框为空时按 `↑` | 调出上一条历史消息，继续按可向前翻找 |
| 浏览历史时按 `↓` | 向后翻找，直到回到输入草稿 |
| 点击历史消息 | 将文字插入输入框，修改后再发 |
| 长按历史消息约半秒 | 直接重发 |

## 常见问题

**状态指示灯不是绿色，是否表示未连接？**

状态点表示检测到了传入的 OSC 活动，不是消息送达回执。OSC 使用 UDP，不会确认对方是否收到；其他应用占用 `9001` 端口时，状态检测也可能不可用。请以 VRChat 聊天框中的实际显示为准，并检查 OSC 开关与目标地址。

**自己能听到朗读，朋友却听不到？**

检查 VRCText 的输出是否选中了虚拟线缆的播放端点、VRChat 的麦克风是否选中了对应录音端点，以及游戏内麦克风是否开启。

**数据保存在哪里？**

设置和聊天历史以明文保存在 `config.toml` 中，语音模型单独保存：

```text
%APPDATA%\vrctext\vrctext\config\config.toml
%APPDATA%\vrctext\vrctext\data\models\
```

历史列表上方的 **全部清空** 可删除消息，设置中的 **删除已下载模型** 可回收模型空间。报告问题时，请移除配置中的私人消息。

语音在本机合成；下载模型会连接 GitHub。OSC 通过未加密的 UDP 向你设置的目标发送文字。

## 从源码构建

使用 **Tauri + Svelte + Rust** 构建桌面界面，通过 **Sherpa-ONNX** 运行 AI 语音模型。

安装 Node.js 22+、Rust stable（Windows MSVC）、包含 Windows SDK 的 Visual Studio C++ Build Tools，以及 WebView2，然后运行：

```powershell
npm ci
npm run check
npm run tauri build -- --no-bundle
```

产物为 `src-tauri/target/release/vrctext.exe`。开发时运行 `npm run tauri dev`；首次构建会下载原生语音库。依赖源码和原生库构建方式见 [第三方说明](THIRD_PARTY_NOTICES.md)。

## 参与项目

欢迎通过 [Issue](https://github.com/Airdinia/VRCText/issues) 反馈问题或提出建议，也欢迎提交 PR，中英文均可。反馈问题时，请附上应用版本、复现步骤和相关设置，并移除配置中的私人聊天内容。

原创源码采用 [MIT](LICENSE)，© 2026 Airdinia。可执行文件包含 GPLv3 语音组件，发布许可及依赖说明见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

VRCText 是独立的社区项目，与 VRChat 官方无隶属关系。
