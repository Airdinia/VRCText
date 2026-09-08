# Third-party notices

The root [MIT license](LICENSE) covers VRCText's original source. It does not replace licenses for dependencies, fonts, model weights, dictionaries, or native libraries. Preserve upstream copyright and license notices when redistributing them.

## Application dependencies

Exact JavaScript and Rust versions are recorded in `package-lock.json` and `src-tauri/Cargo.lock`. Use `cargo metadata --locked --filter-platform x86_64-pc-windows-msvc` and `npm ls --all` to inspect the resolved dependency trees. A package manifest's license field is an index, not a substitute for its complete license text.

| Component | Upstream license / source |
| --- | --- |
| Tauri | [MIT / Apache-2.0](https://github.com/tauri-apps/tauri) |
| Svelte | [MIT](https://github.com/sveltejs/svelte/blob/main/LICENSE.md) |
| Lucide icons | [ISC, with retained MIT notices for inherited Feather icons](https://github.com/lucide-icons/lucide/blob/main/LICENSE) |
| Sherpa-ONNX Rust / C++ code | [Apache-2.0](https://github.com/k2-fsa/sherpa-onnx/blob/master/LICENSE) |
| ONNX Runtime | [MIT and third-party notices](https://github.com/microsoft/onnxruntime/blob/main/ThirdPartyNotices.txt) |

This table highlights components needing attention; it is not a complete binary bill of materials.

## Static speech libraries

The locked `sherpa-onnx-sys` 1.13.1 build script links `espeak-ng`, `piper_phonemize`, ONNX Runtime, Kaldi/FST components, kissfft, ucd and ssentencepiece, in addition to Sherpa itself. The prebuilt archive is fetched outside Cargo's checksum mechanism. Inspect the actual archive and matching build sources for a release.

[eSpeak NG is GPL-3.0 licensed](https://github.com/espeak-ng/espeak-ng/blob/master/COPYING). Because the current build statically links it, distributing the combined executable requires satisfying GPL terms for the combined work, including access to Corresponding Source. **An MIT file next to the executable, an upstream homepage link, or switching to DLLs alone is not sufficient.** VRCText's original files retain their MIT grant; the combined distribution has additional obligations.

A copy of [GPL-3.0](public/licenses/GPL-3.0.txt) and [Apache-2.0](public/licenses/Apache-2.0.txt) is included for reference. Before releasing a binary, assemble exact native dependency revisions, copyright notices, build instructions and Corresponding Source as described in [RELEASING.md](docs/RELEASING.md). These reference license files alone do not complete that work.

## Fonts

- JetBrains Mono is provided by `@fontsource/jetbrains-mono`; its complete [SIL OFL 1.1 notice](public/licenses/JetBrains-Mono-OFL.txt) is included.
- Sarasa Gothic is loaded from the `SarasaUiSC-Regular` subset in `subsetted-fonts`. Its upstream [SIL OFL 1.1 notice](public/licenses/Sarasa-Gothic-OFL.txt) is included, including inherited copyrights. The package's MIT metadata does not relicense the font. The app does not import that package's PingFang or MiSans fonts.

The `public/licenses` directory is copied into the frontend build and embedded in the executable. Include the same files in a distributable archive so recipients can read them without running the app.

## Downloaded models

Models are fetched on request and are not included in this source tree. Their terms are separate from the app's license. Preserve every license and notice shipped inside a model archive; do not assume that a model weight license also covers all lexicons, phonemizer data, or training data.

| Asset | Provenance |
| --- | --- |
| Matcha zh-baker | [Sherpa Matcha model documentation](https://k2-fsa.github.io/sherpa/onnx/tts/pretrained_models/matcha.html) |
| Vocos 22 kHz universal vocoder | [Sherpa vocoder release](https://github.com/k2-fsa/sherpa-onnx/releases/tag/vocoder-models), [Vocos upstream](https://github.com/gemelo-ai/vocos) |
| Kokoro v1.1 | [Model card, Apache-2.0 weights](https://huggingface.co/hexgrad/Kokoro-82M-v1.1-zh), [Sherpa conversion](https://k2-fsa.github.io/sherpa/onnx/tts/pretrained_models/kokoro.html) |

The inspected Matcha archive has no top-level LICENSE. Its README explicitly identifies the DataBaker training dataset as non-commercial only, without stating a model redistribution grant. New Matcha/Vocos downloads have therefore been removed; existing local installations remain readable. Do not bundle or mirror these assets without resolving their terms. Other auxiliary model assets also need an asset-level redistribution review.
