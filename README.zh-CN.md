# VRCText

通过 OSC 向 VRChat 聊天框发送文字的 Windows 桌面工具，支持可选的本地语音朗读。

[English](README.md) · [简体中文](README.zh-CN.md) · [下载发布版](https://github.com/Airdinia/VRCText/releases)

## 功能

- `Enter` 发送文字，`Shift+Enter` 换行。
- 使用 Windows SAPI 语音或按需下载的 Sherpa-ONNX 模型朗读消息。
- 选择音频输出设备，用于播放或配合虚拟声卡输入到麦克风。
- 保留最近 50 条消息：短按历史记录的箭头插入到光标处，长按 0.5 秒直接重发。
- 打字指示器、窗口置顶、中英文界面。

VRCText 是独立社区项目，与 VRChat 官方无隶属关系。

## 开始使用

需要 Windows 10/11 x64 和 [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)。可用的系统语音取决于 Windows 已安装的语音包。

1. 从 [Releases](https://github.com/Airdinia/VRCText/releases) 获取可用的 Windows 发布包，或按下文从源码构建。请保留包内的许可和第三方声明。
2. 运行 `vrctext.exe`，在 VRChat 设置中启用 OSC。
3. VRChat 与本工具在同一台电脑时，保留默认目标 `127.0.0.1:9000`，输入并发送消息。

应用限制每条消息最多 144 个 Unicode 字符。UDP 没有送达回执，发送成功不代表 VRChat 一定显示了消息。状态点表示 UDP 9001 端口检测到活动；其他 OSC 工具占用该端口时，检测可能不可用。

发布版可能没有代码签名。运行前请核实下载来源；未签名不等于安全。

## 语音朗读

在设置中选择 **SAPI** 使用已安装的 Windows 语音，或选择 **AI 引擎** 下载模型。打开扬声器开关后会朗读已发送的消息，也可以用“试听”预听而不向 VRChat 发送文字。

| 模型 | 支持的语音 | 下载量 |
| --- | --- | --- |
| Kokoro v1.1 | 中文、英文；模型含 103 个音色，界面提供 13 个 | 约 365 MB |

Matcha 已从下载列表移除：其模型包说明训练数据仅限非商业用途，且未附带明确模型许可。已有本地安装仍保留兼容读取。

以上为十进制 MB 的压缩传输量，解压后还需要额外空间。模型来自 Sherpa-ONNX 官方 GitHub Releases，下载后会验证固定的 SHA-256 校验值。语言范围见[上游 Kokoro 文档](https://k2-fsa.github.io/sherpa/onnx/tts/pretrained_models/kokoro.html)。

朗读声音发送到音频输出设备。要让 VRChat 将其作为麦克风输入，需要另行安装虚拟音频线缆，并在 VRCText 选择其播放端点、在 VRChat 选择其录音端点。例如 [VB-CABLE](https://vb-audio.com/Cable/) 对应 `CABLE Input` 和 `CABLE Output`。

## 隐私与本地数据

VRCText 没有账号系统或应用遥测，语音在本机合成。下载模型会连接 GitHub 及其下载服务器；使用主机名可能产生 DNS 查询。OSC 通过未加密的 UDP 向设置的目标发送文字和打字状态，远程目标应位于可信网络。

设置和**聊天历史**以明文保存在：

```text
%APPDATA%\vrctext\vrctext\config\config.toml
%APPDATA%\vrctext\vrctext\data\models\
```

设置中的“全部清空”可清除历史，“删除已下载模型”可回收模型空间。报告问题时，请勿附带含私人消息的配置文件。

## 从源码构建

安装 Node.js 22 或更新版本、Rust stable（`x86_64-pc-windows-msvc`）、包含 Windows SDK 的 Visual Studio C++ Build Tools，以及 WebView2 Runtime。

```powershell
npm ci
npm run check
npm run tauri dev

# 构建可执行文件，不生成安装程序
npm run tauri build -- --no-bundle
```

产物位置：`src-tauri\target\release\vrctext.exe`。请通过 Tauri CLI 构建，以正确生成并嵌入前端。首次 Rust 构建还会下载 Sherpa-ONNX 原生库；Cargo 锁文件不会校验这份单独下载的原生压缩包。开发服务器仅用于本机开发。

贡献及检查方法见 [CONTRIBUTING.md](CONTRIBUTING.md)，二进制分发要求见 [docs/RELEASING.md](docs/RELEASING.md)。

## 许可与致谢

VRCText 的原创源码采用 [MIT 许可](LICENSE)，版权 © 2026 congyoua。依赖和下载模型适用各自的许可。**当前静态 TTS 构建包含 GPL 组件 eSpeak NG，不能将完整可执行文件按“仅 MIT”分发。** 详见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

使用 [Tauri](https://tauri.app/)、[Svelte](https://svelte.dev/) 和 [Sherpa-ONNX](https://github.com/k2-fsa/sherpa-onnx) 构建。内置字体 JetBrains Mono 和 Sarasa Gothic 采用 SIL Open Font License。
