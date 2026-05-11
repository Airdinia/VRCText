# Changelog

All notable changes to **VRCText** are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
