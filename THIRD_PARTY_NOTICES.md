# Third-party notices

The root [MIT license](LICENSE) covers VRCText's original source. It does not replace licenses for dependencies, fonts, model weights, dictionaries, or native libraries. Preserve upstream copyright and license notices when redistributing them.

## Application dependencies

Exact dependency versions are recorded in `package-lock.json` and `src-tauri/Cargo.lock`. The following table highlights major components; it is not a complete binary license inventory.

| Component | Upstream license / source |
| --- | --- |
| Tauri | [MIT / Apache-2.0](https://github.com/tauri-apps/tauri) |
| Svelte | [MIT](https://github.com/sveltejs/svelte/blob/main/LICENSE.md) |
| Lucide icons | [ISC, with retained MIT notices for inherited Feather icons](https://github.com/lucide-icons/lucide/blob/main/LICENSE) |
| Sherpa-ONNX Rust / C++ code | [Apache-2.0](https://github.com/k2-fsa/sherpa-onnx/blob/master/LICENSE) |
| ONNX Runtime | [MIT and third-party notices](https://github.com/microsoft/onnxruntime/blob/main/ThirdPartyNotices.txt) |

## Static speech libraries

The current Sherpa build statically links eSpeak NG and other native libraries. Its prebuilt archive is downloaded separately and is not verified by Cargo.lock.

[eSpeak NG uses GPL-3.0](https://github.com/espeak-ng/espeak-ng/blob/master/COPYING). Distributing the combined executable requires satisfying GPL terms, including providing Corresponding Source, matching build scripts and dependency notices. VRCText's original files retain their MIT grant.

Copies of [GPL-3.0](public/licenses/GPL-3.0.txt) and [Apache-2.0](public/licenses/Apache-2.0.txt) are included. Before a binary release, verify the native artifacts and assemble their exact sources and notices; the included license texts alone do not complete this requirement.

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

The Matcha archive identifies non-commercial training data without a clear model redistribution grant. New Matcha/Vocos downloads have been removed; existing local installations remain readable. Resolve the terms of each model and auxiliary asset before bundling or mirroring them.
