# Licenses and dependency sources

VRCText's original source is [MIT licensed](LICENSE). The executable statically includes eSpeak NG and is distributed under [GNU GPLv3](public/licenses/GPL-3.0.txt). Fonts and other components retain their own licenses. [Complete third-party notices](public/licenses/THIRD-PARTY.txt) are included in the repository and embedded in the executable.

A release may provide the EXE as its only asset. Link this document at the matching release commit in the release notes so recipients can obtain the corresponding source and notices. Source can be hosted separately; keep its download links available. Users do not have to download source to run the app.

## Source locations

- **VRCText:** use the repository commit identified by the release tag.
- **Rust dependencies:** names, versions and source checksums are in [Cargo.lock](src-tauri/Cargo.lock). Source archives are available from `https://static.crates.io/crates/NAME/NAME-VERSION.crate`; `cargo vendor --manifest-path src-tauri/Cargo.toml --locked` retrieves them.
- **Frontend dependencies:** [package-lock.json](package-lock.json) records source package URLs, versions and integrity values. Svelte is MIT; Tauri is MIT / Apache-2.0; Lucide is ISC with retained MIT notices for inherited Feather icons.
- **Native speech dependencies:** [source links and SHA-256 checksums](third-party/native-sources.json) pin Sherpa-ONNX 1.13.1, eSpeak NG, ONNX Runtime 1.24.4 and their build dependencies. These are upstream source archives, not precompiled library downloads.
- **Fonts:** only JetBrains Mono and the SarasaUiSC subset are used; their [OFL notices](public/licenses/) are included. PingFang and MiSans assets are not included in the executable.

## Building native libraries

Normal application builds follow the [README](README.md#build) and download pinned prebuilt native libraries. To build them from source, extract the Sherpa and ONNX Runtime archives listed above. Install Visual Studio 2022 C++ tools, CMake 3.28+, Python and Protobuf `protoc` 21.12, then run:

```powershell
./scripts/build-native.ps1 -SherpaSource <sherpa-source-directory> -OnnxSource <onnx-source-directory>
$env:SHERPA_ONNX_LIB_DIR = '<printed sherpa-install/lib directory>'
npm run tauri build -- --no-bundle
```

The script and [static-library adapter](third-party/onnx-static/CMakeLists.txt) are MIT-licensed build materials. CMake fetches pinned dependencies listed in the source manifest; `FETCHCONTENT_SOURCE_DIR_<NAME>` can select local source copies. The release uses upstream native libraries verified by checksum; this alternative native rebuild has not been tested end to end locally.

## Models

Models are downloaded separately and are not part of the EXE. Kokoro v1.1 weights are [Apache-2.0](https://huggingface.co/hexgrad/Kokoro-82M-v1.1-zh); preserve notices included in its archive. [Matcha zh-baker](https://k2-fsa.github.io/sherpa/onnx/tts/pretrained_models/matcha.html) and its [Vocos vocoder](https://github.com/k2-fsa/sherpa-onnx/releases/tag/vocoder-models) are also available as on-demand downloads from official Sherpa releases. Model and dataset terms remain separate from the application license.
