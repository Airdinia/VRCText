# VRCText

A Windows desktop app for sending text to VRChat's OSC chatbox, with optional local text-to-speech.

[English](README.md) · [简体中文](README.zh-CN.md) · [Releases](https://github.com/Airdinia/VRCText/releases)

[![Source license: MIT](https://img.shields.io/badge/Source%20license-MIT-yellow.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/Airdinia/VRCText?include_prereleases&color=blue)](https://github.com/Airdinia/VRCText/releases)
[![Downloads](https://img.shields.io/github/downloads/Airdinia/VRCText/total?color=brightgreen)](https://github.com/Airdinia/VRCText/releases)
[![Stars](https://img.shields.io/github/stars/Airdinia/VRCText?style=social)](https://github.com/Airdinia/VRCText/stargazers)
![Windows 10/11](https://img.shields.io/badge/Windows%2010%2F11-0078D6?logo=windows&logoColor=white)

## Features

- Send with `Enter`; use `Shift+Enter` for a newline.
- Local speech with Windows voices or Kokoro, with selectable audio output.
- Last 50 messages, typing indicator, always-on-top, and Chinese / English UI.

## Get started

Requires Windows 10/11 x64 and [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).

1. Download the EXE from [Releases](https://github.com/Airdinia/VRCText/releases) and run it.
2. Enable OSC in VRChat. Keep `127.0.0.1:9000` when both apps run on the same PC.
3. Type a message and press `Enter`. Messages are limited to 144 characters.

In settings, choose **SAPI** for installed Windows voices or **AI Engine** to download Kokoro (about 365 MB, Chinese / English). Use a virtual audio cable to route speech output to VRChat's microphone input. Select its playback endpoint in VRCText and its recording endpoint in VRChat.

UDP does not confirm message delivery. The status dot indicates incoming OSC activity; another app using port 9001 can prevent detection.

## Local data

No accounts or application telemetry. Speech runs locally; model downloads contact GitHub. OSC sends text to your configured destination over unencrypted UDP.

Settings and message history are stored in plain text; do not include private messages in bug reports.

```text
%APPDATA%\vrctext\vrctext\config\config.toml
%APPDATA%\vrctext\vrctext\data\models\
```

Use **Clear all** above the history list to remove messages, or **Delete models** in settings to reclaim model storage.

## Build

Install Node.js 22+, Rust stable (Windows MSVC), Visual Studio C++ Build Tools with Windows SDK, and WebView2.

```powershell
npm ci
npm run check
npm run tauri build -- --no-bundle
```

Output: `src-tauri/target/release/vrctext.exe`. For development, run `npm run tauri dev`. The first build downloads native speech libraries.

## License

Original source: [MIT](LICENSE), © 2026 Airdinia. The executable includes GPLv3 speech components; [licenses, dependency sources and build instructions](THIRD_PARTY_NOTICES.md) are provided in this repository. Release downloads can be a single EXE, with a link to these materials in the release notes.

An independent community project, not affiliated with VRChat. Built with Tauri, Svelte and Sherpa-ONNX. Issues and PRs are welcome in Chinese or English.
