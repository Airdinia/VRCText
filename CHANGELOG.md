# Changelog

All notable changes to **VRCText** are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- OSC target now accepts hostnames (`localhost`, LAN machine names) — previously only IP literals worked even though the settings UI let you type a hostname, so every send failed with "invalid target".
- Expand the curated Kokoro voice picker from 1 to 13 speakers (3 English, 5 Chinese female, 5 Chinese male). Any of the 103 sids can still be set by hand-editing `tts_voice_sherpa`.

### Fixed
- Stamp the taskbar (big) and title-bar (small) window icons explicitly at startup via `WM_SETICON`, loading them from the exe's embedded icon resource — tao only ever sets the small icon, so the taskbar icon fell back to the shell icon cache and occasionally showed up blank after launch.
- The AI engine's async load no longer triggers a redundant synchronous model reload on completion — previously this could double the multi-second load time and block the TTS worker, and the audio device was opened twice back to back.
- TTS status now clears stale device/voice names after an engine switch, and toggling the speaker while the AI engine is still loading shows the loading state instead of rendering it as an error.
- Frontend event listeners are registered before the initial state fetch, so a status event firing during app init (e.g. the AI engine finishing its load) can no longer be lost, which left the UI stuck on "loading".
- Editing the OSC IP/port in settings is no longer wiped when another setting (pin, bell, voice toggle) changes mid-edit.
- Model downloads now use connect/read timeouts — a stalled connection no longer wedges the downloader ("a download is already in progress") until app restart.
- Fixed a race where a lingering download watcher could observe, double-report, or wrongly clear a newer download.
- Deleting models now unloads the in-memory AI engine so status honestly reports "no model" (previewing no longer plays a deleted voice from RAM), and deletion is blocked while a download is writing into the directory.
- The composer textarea height now tracks programmatic text changes (history navigation ↑/↓, append-from-history, clear-on-send), not just typed input.

- Device/voice selections made while the AI engine is still loading are no longer rolled back once the load completes — the load restarts with the new voice and binds the newly picked device.

### Changed
- Download, delete, and device-refresh failures surface as toasts instead of logging silently to the console.

## [0.2.4] - 2026-05-10

### Changed
- Trim release binary size and tighten hot paths in the OSC and TTS dispatch loops.

## [0.2.3] - 2026-05-10

### Added
- Embed the app icon in the Windows executable resources and use it for the window viewport.

### Fixed
- Track IME pre-edit length as a third Enter-guard signal so candidate selections don't accidentally fire a send.
- Layer an IME-event cooldown on top of the IMM32 composition check to ride out brief gaps between composition end and the next key event.
- Always query IMM32 directly for IME composition state instead of relying on focus heuristics.

## [0.2.2] - 2026-05-10

### Changed
- Trim the TTS engine's runtime memory footprint and widen the composer so longer messages fit on a single visible row.
- Unify UI alignment across header / history / composer and de-jitter the history list on hover.

## [0.2.1] - 2026-05-10

### Added
- Gate Enter-to-send on the current IMM32 composition state so the IME's confirmation Enter no longer ships the draft.

### Changed
- Collapse the duplicate `ModelKind` / `PackKind` types into a single enum and tighten TTS engine initialisation.

### Fixed
- Hide the speaker toggle entirely while the TTS engine is unavailable instead of showing a non-functional control.

## [0.2.0] - 2026-05-10

### Added
- **Kokoro multi-lang v1.1 TTS pack** alongside Matcha — ~350 MB, full precision, 103 voices across English / Chinese / Japanese / Korean / French.
- Background loading for the Sherpa engine so the window opens instantly and the model warms up off the UI thread.

### Fixed
- Composer click-to-caret now resolves to the right insertion point in multi-line drafts.
- Dim the empty-composer hint text for better legibility against the terminal-style background.

[Unreleased]: https://github.com/congyoua/VRCText/compare/v0.2.4...HEAD
[0.2.4]: https://github.com/congyoua/VRCText/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/congyoua/VRCText/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/congyoua/VRCText/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/congyoua/VRCText/compare/v0.2...v0.2.1
[0.2.0]: https://github.com/congyoua/VRCText/releases/tag/v0.2
