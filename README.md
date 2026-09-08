# VRCText

A Windows desktop app for sending text to VRChat's OSC chatbox, with optional local text-to-speech.

[English](README.md) · [简体中文](README.zh-CN.md) · [Releases](https://github.com/Airdinia/VRCText/releases)

## Features

- Send chatbox messages with `Enter`; use `Shift+Enter` for a newline.
- Read messages aloud with installed Windows SAPI voices or downloadable Sherpa-ONNX models.
- Choose an audio output device for playback or virtual microphone routing.
- Reuse the last 50 messages: tap the history arrow to insert at the caret, or hold it for 0.5 seconds to resend.
- Typing indicator, always-on-top mode, and Chinese / English interface.

VRCText is an independent community project and is not affiliated with VRChat.

## Get started

Requires Windows 10/11 x64 and [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/). Voice availability depends on your installed Windows speech voices.

1. Get a Windows build from [Releases](https://github.com/Airdinia/VRCText/releases), when available, or build from source below. Keep accompanying license and notice files.
2. Run `vrctext.exe` and enable OSC in VRChat's settings.
3. Leave the target at `127.0.0.1:9000` for VRChat on the same PC, then send a message.

The app limits messages to 144 Unicode characters. UDP has no delivery receipt: a successful send does not guarantee that VRChat displayed the message. The status dot indicates incoming activity on UDP port 9001; another OSC app using that port can prevent detection.

Release builds may be unsigned. Verify their origin before running them; an unsigned executable is not a guarantee of safety.

## Text-to-speech

In settings, choose **SAPI** for installed Windows voices or **AI Engine** for downloadable models. Enable the speaker toggle to read sent messages, or use Preview without sending to VRChat.

| Model | Supported speech | Download |
| --- | --- | --- |
| Kokoro v1.1 | Chinese and English; 103 model speakers, 13 offered in the picker | About 365 MB |

Matcha is no longer offered for download: its archive identifies non-commercial training data and does not include a clear model license. Existing local installations remain readable for compatibility.

Sizes are compressed transfer sizes rounded in decimal MB; extracted models need additional disk space. Models download from official Sherpa-ONNX GitHub releases and are checked against fixed SHA-256 digests. [Upstream Kokoro documentation](https://k2-fsa.github.io/sherpa/onnx/tts/pretrained_models/kokoro.html) describes the Chinese / English v1.1 pack.

TTS plays to an output device. To use it as VRChat's microphone, install a virtual audio cable separately, select its playback endpoint in VRCText, and select its recording endpoint in VRChat. For example, [VB-CABLE](https://vb-audio.com/Cable/) uses `CABLE Input` and `CABLE Output`, respectively.

## Privacy and local data

VRCText has no account system or application telemetry. Synthesis runs locally. Model downloads contact GitHub and its download hosts; a configured hostname can trigger DNS lookups. OSC sends text and typing state to the configured host using unencrypted UDP. Use a trusted network for remote targets.

Settings **and message history** are stored in plain text:

```text
%APPDATA%\vrctext\vrctext\config\config.toml
%APPDATA%\vrctext\vrctext\data\models\
```

Use **Clear all** in settings to clear history, and **Delete models** to reclaim model storage. Do not include configuration or private messages in bug reports.

## Build from source

Install Node.js 22 or newer, Rust stable for `x86_64-pc-windows-msvc`, Visual Studio C++ Build Tools with the Windows SDK, and WebView2 Runtime.

```powershell
npm ci
npm run check
npm run tauri dev

# Release executable; no installer
npm run tauri build -- --no-bundle
```

Output: `src-tauri\target\release\vrctext.exe`. Use the Tauri CLI so the frontend is built and embedded. The first Rust build also downloads native Sherpa-ONNX libraries; the lockfile does not provide an integrity check for that separate native archive. Development server access is intended for the local machine.

See [CONTRIBUTING.md](CONTRIBUTING.md) for checks and [docs/RELEASING.md](docs/RELEASING.md) for distribution requirements.

## License and credits

VRCText's original source is [MIT licensed](LICENSE), copyright © 2026 Airdinia. Dependencies and downloaded models retain their own licenses. **The current static TTS build includes eSpeak NG, a GPL component; the complete executable must not be distributed as MIT-only.** See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for the distinction and release requirements.

Built with [Tauri](https://tauri.app/), [Svelte](https://svelte.dev/) and [Sherpa-ONNX](https://github.com/k2-fsa/sherpa-onnx). Bundled typography uses JetBrains Mono and Sarasa Gothic under the SIL Open Font License.
