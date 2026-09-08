# Contributing

Bug reports, documentation improvements, and focused pull requests are welcome in English or Chinese.

Before opening a pull request, describe the problem and expected behavior. For a bug, include the app version, Windows version, selected TTS engine, and steps to reproduce. Remove private messages, local usernames, device identifiers, and credentials from attachments. Report security issues using [SECURITY.md](SECURITY.md).

## Development

Follow the Windows prerequisites in [README.md](README.md), then run:

```powershell
npm ci
npm run check
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --locked
node scripts/audit-repository.mjs --history
npm audit --audit-level=high
```

Use `npm run tauri dev -- --features devtools` when an inspector is needed. Set `VRCTEXT_DEVTOOLS=1` to open it on launch. Keep debugging features out of release builds.

Update both language dictionaries and both READMEs when changing user-facing behavior. Add a regression test for security or logic fixes where practical. Verify audio, OSC and IME behavior on Windows when those paths change; unit tests cannot exercise VRChat or physical audio devices.

Keep pull requests focused. Explain what changed and which checks ran, and add a concise entry to the Unreleased changelog. Retain the npm and Cargo lockfiles. Do not commit build outputs, downloaded models, local configuration, editor state, or credentials.

## Licensing and attribution

Submit only material you have the right to contribute under the project's MIT source license. Preserve third-party copyright notices and document additional dependencies in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). Generated or copied assets also need a verifiable source and permission to redistribute.

Commit authorship should identify the human contributor. Omit automated assistant co-author trailers. Assistance does not replace the contributor's responsibility to review and test a change.
