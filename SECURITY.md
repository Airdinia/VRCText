# Security

Security fixes target the current development branch and latest release. Older versions are not maintained separately.

Use [GitHub private vulnerability reporting](https://github.com/Airdinia/VRCText/security/advisories/new) if enabled. If unavailable, open an issue requesting a private contact channel without exploit details or secrets. Do not post credentials or private chat history publicly. Response times are not guaranteed.

Include the affected version, Windows version, reproduction steps, impact, and a minimal example with private data removed.

## Trust boundaries

- The frontend is bundled locally. Tauri capabilities are limited to the main window, and a content security policy restricts scripts and network access.
- OSC is unencrypted and unauthenticated UDP. The destination is user-configurable; incoming activity on port 9001 is an indicator, not proof of sender identity or message delivery.
- Settings and history are plain text in the user's application data directory.
- Download URLs are fixed. Models use HTTPS, SHA-256 verification, staging, size limits, and extraction rules that reject links and parent paths.
- Local model files and processes running as the same Windows user are trusted. Previously installed files are not rehashed on every launch.
- The Sherpa build dependency downloads prebuilt libraries separately from Cargo. Review and pin those artifacts before binary distribution; `cargo audit` does not inspect their C/C++ contents.

The repository hygiene script is a heuristic secret check, not a complete security scanner. See [docs/RELEASING.md](docs/RELEASING.md) for release checks.
