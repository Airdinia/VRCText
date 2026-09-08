# VRCText

通过 OSC 向 VRChat 聊天框发送文字的 Windows 桌面工具，支持可选的本地语音朗读。

[English](README.md) · [简体中文](README.zh-CN.md) · [下载发布版](https://github.com/Airdinia/VRCText/releases)

[![源码许可：MIT](https://img.shields.io/badge/Source%20license-MIT-yellow.svg)](LICENSE)
[![版本](https://img.shields.io/github/v/release/Airdinia/VRCText?include_prereleases&color=blue)](https://github.com/Airdinia/VRCText/releases)
[![下载量](https://img.shields.io/github/downloads/Airdinia/VRCText/total?color=brightgreen)](https://github.com/Airdinia/VRCText/releases)
[![Stars](https://img.shields.io/github/stars/Airdinia/VRCText?style=social)](https://github.com/Airdinia/VRCText/stargazers)
![Windows 10/11](https://img.shields.io/badge/Windows%2010%2F11-0078D6?logo=windows&logoColor=white)

## 功能

- `Enter` 发送，`Shift+Enter` 换行。
- 使用 Windows 语音或 Kokoro 本地朗读，可选择音频输出设备。
- 最近 50 条消息、打字指示器、窗口置顶和中英文界面。

## 开始使用

需要 Windows 10/11 x64 和 [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)。

1. 从 [Releases](https://github.com/Airdinia/VRCText/releases) 下载 EXE，直接运行。
2. 在 VRChat 中启用 OSC；两者在同一台电脑时，保留默认目标 `127.0.0.1:9000`。
3. 输入文字并按 `Enter` 发送，每条最多 144 个字符。

设置中选择 **SAPI** 使用 Windows 已安装的语音，或选择 **AI 引擎** 下载 Kokoro（约 365 MB，支持中英文）。使用虚拟音频线缆将语音输出路由至 VRChat 的麦克风输入：VRCText 选择播放端点，VRChat 选择录音端点。

UDP 没有送达回执。状态点表示检测到 OSC 活动；其他应用占用 9001 端口时，检测可能不可用。

## 本地数据

没有账号系统或应用遥测。语音在本机合成，下载模型会连接 GitHub；OSC 通过未加密的 UDP 向设置的目标发送文字。

设置和聊天历史以明文保存在以下位置，报告问题时请勿附带私人消息。

```text
%APPDATA%\vrctext\vrctext\config\config.toml
%APPDATA%\vrctext\vrctext\data\models\
```

历史列表上方的“全部清空”可删除消息，设置中的“删除已下载模型”可回收模型空间。

## 构建

安装 Node.js 22+、Rust stable（Windows MSVC）、包含 Windows SDK 的 Visual Studio C++ Build Tools，以及 WebView2。

```powershell
npm ci
npm run check
npm run tauri build -- --no-bundle
```

产物为 `src-tauri/target/release/vrctext.exe`。开发时运行 `npm run tauri dev`；首次构建会下载原生语音库。

## 许可

原创源码采用 [MIT](LICENSE)，© 2026 Airdinia。可执行文件包含 GPLv3 语音组件；[许可证、依赖源码和构建说明](THIRD_PARTY_NOTICES.md)在仓库中提供。Release 可以仅上传 EXE，在发布说明中链接这些材料。

这是与 VRChat 官方无隶属关系的社区项目，使用 Tauri、Svelte 和 Sherpa-ONNX 构建。欢迎中英文 Issue 和 PR。
