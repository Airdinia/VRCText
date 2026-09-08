<p align="center">
  <img src="public/icon.png" width="88" alt="VRCText icon">
</p>

<h1 align="center">VRCText</h1>

<p align="center">
  <strong>Stay in the conversation, even with your mic off.</strong><br>
  A desktop companion for VRChat: type, send, reuse, and give your words a voice.
</p>

<p align="center">
  <a href="https://github.com/Airdinia/VRCText/releases"><strong>Download for Windows</strong></a> ·
  <a href="#get-started">Get started</a> ·
  <a href="README.zh-CN.md">简体中文</a> ·
  <a href="https://github.com/Airdinia/VRCText/issues">Report an issue</a>
</p>

<p align="center">
  <a href="https://github.com/Airdinia/VRCText/releases"><img src="https://img.shields.io/github/v/release/Airdinia/VRCText?include_prereleases&amp;color=blue" alt="Release"></a>
  <img src="https://img.shields.io/badge/Windows%2010%2F11-0078D6?logo=windows&amp;logoColor=white" alt="Windows 10/11">
  <a href="LICENSE"><img src="https://img.shields.io/badge/Source%20license-MIT-yellow.svg" alt="Source license: MIT"></a>
  <a href="https://github.com/Airdinia/VRCText/releases"><img src="https://img.shields.io/github/downloads/Airdinia/VRCText/total?color=brightgreen" alt="Downloads"></a>
</p>

---

Exploring quietly, unable to use your mic, or simply more comfortable typing? Open VRCText, write a message, and press `Enter` to send it to VRChat's chatbox over OSC. Turn on local text-to-speech whenever you want your words read aloud, too.

## Small details that keep the conversation moving

- **Type and send** — `Enter` to send, `Shift+Enter` for a newline, and a character counter to keep you within the limit.
- **Say it again without retyping** — Keep your last 50 messages. Click to insert one into your draft, hold to resend, or browse history with the arrow keys.
- **Give your text a voice** — Use installed Windows voices or download Matcha (Chinese) or Kokoro (Chinese and English), synthesized on your own PC.
- **Choose where speech goes** — Pick a voice and output device, then preview it. With a virtual audio cable, friends in VRChat can hear your speech, too.
- **Keep the controls close** — Always-on-top mode, a typing indicator, a chatbox notification sound toggle, and English / Chinese UI.
- **No account to create** — Settings and history stay local, with no application telemetry. Text chat needs no speech model download.

## Get started

**Requirements:** Windows 10/11 x64 and [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).

1. **Download and open.** Get the EXE from [Releases](https://github.com/Airdinia/VRCText/releases) and run it.
2. **Enable OSC.** Turn on OSC in VRChat. When both apps run on the same PC, keep VRCText's default destination, `127.0.0.1:9000`.
3. **Send your first message.** Type, press `Enter`, and check the chatbox in VRChat.

Messages can contain up to **144 characters**; the app warns you if you go over. Speech is off by default, so you're ready to chat with just these steps.

## Want your text read aloud?

Open settings and choose a speech engine:

| Engine | A good fit for | What you need |
| --- | --- | --- |
| **Windows Native (SAPI)** | Using voices already on your system | An installed Windows voice; no extra model download |
| **AI Model (Matcha / Kokoro)** | Trying AI voices in Chinese and English | Download Matcha (Chinese, about 129 MB) or Kokoro (Chinese and English, about 365 MB) in the app; synthesis runs locally afterward |

Choose a voice and audio output device, use **Preview** to check the result, then enable speech with the speaker button at the top of the window. Messages you send will now be read aloud as well.

**To let friends in VRChat hear it:** install and configure a virtual audio cable separately to feed VRCText's audio into VRChat's microphone input.

```text
VRCText speech → Virtual cable playback endpoint → Recording endpoint → VRChat mic
```

Select the cable's **playback endpoint** as VRCText's output and its matching **recording endpoint** as VRChat's microphone input.

## Less retyping, more chatting

| Action | Result |
| --- | --- |
| `Enter` | Send your message |
| `Shift+Enter` | Insert a newline |
| `↑` with an empty input | Recall the last message; keep pressing to browse older ones |
| `↓` while browsing history | Move forward through history, then return to your draft |
| Click a history message | Insert its text into the input to edit before sending |
| Hold a history message for about half a second | Resend it immediately |

## Common questions

**The status dot isn't green. Am I disconnected?**

The dot indicates incoming OSC activity, not a delivery receipt. OSC uses UDP and cannot confirm that a message was received; another app using port `9001` can also prevent status detection. Check the actual chatbox in VRChat, along with your OSC toggle and destination address.

**I can hear speech, but my friends can't.**

Check that VRCText outputs to the virtual cable's playback endpoint, VRChat uses the matching recording endpoint as its microphone, and your in-game microphone is enabled.

**Where is my data stored?**

Settings and message history are saved as plain text in `config.toml`. Speech models are stored separately:

```text
%APPDATA%\vrctext\vrctext\config\config.toml
%APPDATA%\vrctext\vrctext\data\models\
```

Use **Clear all** above the history list to remove messages, or **Delete models** in settings to reclaim model storage. Remove private messages from your config before including it in a bug report.

Speech is synthesized locally; model downloads contact GitHub. OSC sends text to your configured destination over unencrypted UDP.

## Build

Built with **Tauri + Svelte + Rust**, with **Sherpa-ONNX** running the AI speech models.

Install Node.js 22+, Rust stable (Windows MSVC), Visual Studio C++ Build Tools with Windows SDK, and WebView2, then run:

```powershell
npm ci
npm run check
npm run tauri build -- --no-bundle
```

Output: `src-tauri/target/release/vrctext.exe`. For development, run `npm run tauri dev`. The first build downloads native speech libraries. See [third-party notices](THIRD_PARTY_NOTICES.md) for dependency sources and native library build instructions.

## Get involved

Found a bug or have an idea to make chatting easier? [Open an issue](https://github.com/Airdinia/VRCText/issues) or send a PR. English and Chinese are both welcome. For bug reports, include the app version, steps to reproduce, and relevant settings, with private chat content removed.

Original source: [MIT](LICENSE), © 2026 Airdinia. The executable includes GPLv3 speech components; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for distribution licensing and dependency details.

VRCText is an independent community project, not affiliated with VRChat.
