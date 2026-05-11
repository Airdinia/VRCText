# VRCText

> Tiny VRChat OSC chatbox sender with optional AI text-to-speech. Single ~20 MB exe, double-click to run.

[English](./README.md) · [中文](./README.zh-CN.md)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/congyoua/VRCText?include_prereleases&color=blue)](https://github.com/congyoua/VRCText/releases)
[![Downloads](https://img.shields.io/github/downloads/congyoua/VRCText/total?color=brightgreen)](https://github.com/congyoua/VRCText/releases)
[![Stars](https://img.shields.io/github/stars/congyoua/VRCText?style=social)](https://github.com/congyoua/VRCText/stargazers)
![Windows](https://img.shields.io/badge/Windows%2010%2F11-0078D6?logo=windows&logoColor=white)

---

## Demo

<!-- TODO: replace this placeholder with a real screenshot or GIF -->
<!-- Suggested: docs/demo.gif, ~600 px wide, < 2 MB, recorded with ScreenToGif -->

<details><summary>Preview (placeholder — replace with a real shot)</summary>

```
┌────────────────────────────────────────────┐
│ VRCText  ● OSC 127.0.0.1:9000    📌 🔔 🔊 ⚙│
├────────────────────────────────────────────┤
│  14:32    Hello everyone              [↺] │
│  Yesterday 20:15  How's the weather?  [↺] │
├────────────────────────────────────────────┤
│ ┌────────────────────────────────────────┐│
│ │ type a message…   [Enter] to send       ││
│ └────────────────────────────────────────┘│
│ 12/144    Sent                    [Send ⏎]│
└────────────────────────────────────────────┘
```

</details>

---

## Features

- **Type → chatbox bubble** above your VRChat avatar via OSC. No more fumbling with the in-game soft keyboard.
- **Type → spoken voice**, two engines side by side:
  - **SAPI** — Windows built-in, zero install, ready out of the box
  - **AI engine** — natural neural synthesis, models fetched in-app on demand:
    - **Matcha zh-baker** (~85 MB) — Mandarin Chinese, single voice
    - **Kokoro multilingual v1.1** (~350 MB, full precision) — 103 voices across **English, Chinese, Japanese, Korean, French**
- **Output device routing** — pick any audio output (pair with a virtual cable to feed VRChat's mic input)
- **History** — hover any past message to re-send (tap = append to composer, hold 0.5 s = resend with progress)
- **Typing indicator**, always-on-top toggle, dark UI, persistent config
- **Bilingual UI** — full English + Chinese (zh) parity, switchable in settings
- **No telemetry, no account, no internet** (after the optional AI model download)

---

## Install

1. Grab `vrctext.exe` from [Releases](https://github.com/congyoua/VRCText/releases).
2. Double-click — no installer, no admin rights.
3. In VRChat: `Settings → OSC → Enabled`.

> First launch may show Windows SmartScreen ("Windows protected your PC") → **More info → Run anyway**. The exe is unsigned (no Authenticode cert); this is normal for indie tools.

---

## Usage

### Send to the chatbox

Type in the composer. `Enter` sends, `Shift+Enter` inserts a newline. The bubble appears above your avatar immediately.

### Voice readout

Open ⚙ Settings → **Engine** = **SAPI** or **AI Engine**.

- **SAPI** — works the moment you install Windows. Pick voice / language in the dropdown (Huihui zh-CN, Zira, David, …). Add more in *Windows Settings → Time & Language → Speech → Manage voices*.
- **AI Engine** — first time, click **Download** on the pack you want:
  - **Matcha (~85 MB)** for Chinese only
  - **Kokoro (~350 MB)** for multilingual coverage with 103 voices
  Stored at `%APPDATA%\vrctext\vrctext\data\models\`. Click **Delete models** to reclaim space at any time.

Hit **🔈 Preview** to hear a sample without sending anything to chat.

### Route TTS to VRChat's mic

VRChat treats your microphone as the input; to feed TTS audio in, you need a virtual cable (Windows doesn't ship one):

1. Install [**VB-Audio Virtual Cable**](https://vb-audio.com/Cable/) (free).
2. VRCText → ⚙ → **Output device** = `CABLE Input`, toggle 🔊 on.
3. VRChat → Microphone = `CABLE Output`.
4. *(Optional)* `CABLE Output` properties → **Listen** → enable *Listen to this device* so you can hear yourself too.

### History

| Action | Effect |
|---|---|
| Hover a row | `↺` button slides in on the right |
| Tap `↺` | Content appended to the composer at the caret |
| Hold `↺` for 0.5 s | Resend directly (progress bar; composer untouched) |
| `↑` / `↓` with empty composer | Step through recent messages |

Capped at 50 entries — oldest are dropped automatically. ⚙ → **Clear all** clears the lot (two-step confirm).

---

## Config / Data location

```
%APPDATA%\vrctext\vrctext\config\config.toml    # settings
%APPDATA%\vrctext\vrctext\data\models\          # AI models (downloaded on demand)
```

> The nested `vrctext\vrctext` isn't a typo — it's the standard layout the [`directories`](https://crates.io/crates/directories) crate produces from `ProjectDirs::from("dev", "vrctext", "vrctext")`. Delete either folder to reset settings or reclaim disk space.

---

## Known limitations

- **Windows only.** The backend talks directly to SAPI / WaveOut / WinSock / IMM32.
- Requires **Microsoft WebView2 Runtime**. Win 11 ships with it; a few older Win 10 builds need to install it manually from [microsoft.com/edge/webview2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) — VRCText shows a bilingual dialog with the link if it's missing.
- **No bundled virtual audio cable** (see VB-Cable section above).
- **Unsigned `.exe`** — SmartScreen prompts on first run. Harmless; skippable.
- **AI engine needs network for the first download**, then runs fully offline.

---

## Build from source

Stack: **Tauri 2** (Rust backend) + **Svelte 5** + **TypeScript** + **Vite**.

**Prerequisites**
- [Rust stable](https://rustup.rs/) with `x86_64-pc-windows-msvc` target
- Node.js 18 or newer
- The MSVC C++ build tools (Visual Studio Build Tools — anything that gives you `link.exe`)

```sh
# One-time setup
rustup target add x86_64-pc-windows-msvc
npm install

# Dev (hot-reload, DevTools available with --features devtools)
npm run tauri dev

# Release build — standalone exe, no NSIS / MSI installer
npm run tauri build -- --no-bundle
# Output: src-tauri\target\release\vrctext.exe
```

`bundle.active: false` in `tauri.conf.json` makes the release build produce a single standalone `.exe` rather than an installer. Flip it to `true` and set `"targets": "nsis"` if you want an NSIS installer instead.

> **Heads up:** Use `npm run tauri build` (which routes through `tauri-cli`), not a plain `cargo build --release`. The latter skips tauri-cli's dev/release URL swap, so the WebView keeps pointing at `devUrl: http://localhost:1420` and the window opens blank. If you really need to bypass tauri-cli, either remove `devUrl` from `tauri.conf.json` or set `app.windows[0].url = "index.html"` to force `frontendDist`.
>
> Also: Svelte 5 runes (`$state` / `$derived` / `$effect`) only compile inside `.svelte` and `.svelte.ts` / `.svelte.js` files. Putting `$state(...)` in a plain `.ts` throws `ReferenceError` at runtime — that's why the store is named [`src/lib/stores.svelte.ts`](src/lib/stores.svelte.ts), not `.ts`.

---

## Acknowledgements

- [Tauri](https://tauri.app/) — the desktop runtime that keeps the binary tiny
- [Svelte](https://svelte.dev/) — UI framework
- [Sherpa-ONNX](https://github.com/k2-fsa/sherpa-onnx) — on-device neural TTS
- [VB-Audio](https://vb-audio.com/Cable/) — virtual cable for mic routing

---

## License

[MIT](./LICENSE) © 2026 congyoua
