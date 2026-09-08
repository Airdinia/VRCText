# Releasing VRCText

## Source release

1. Run the checks in [CONTRIBUTING.md](../CONTRIBUTING.md), including the history hygiene scan. Review findings; a clean heuristic scan is not proof that no secrets exist.
2. Check both READMEs, version numbers in the npm/Cargo/Tauri manifests, and the changelog. Preserve human attribution and the original copyright notice.
3. Publish only Git-tracked source files. Do not upload a ZIP of the entire working folder: it can contain private history backups, configuration, caches and models.
4. Enable private vulnerability reporting on GitHub if desired; `SECURITY.md` includes a fallback when it is unavailable. Review repository visibility, description and branch protection in GitHub before launch.

Suggested repository description: **Windows VRChat OSC chatbox with local text-to-speech.**

## Binary release gate

The default build statically links GPL-covered eSpeak NG through Sherpa. A binary release needs more than this repository's MIT license. Until the following is assembled and reviewed, publish source only:

1. Resolve and record exact native archive and source revisions, including transitive C/C++ dependencies. Verify and record archive SHA-256 values; Cargo.lock does not verify these downloads. Use `SHERPA_ONNX_LIB_DIR` or `SHERPA_ONNX_ARCHIVE_DIR` for a reviewed local build input.
2. Assemble the Corresponding Source needed to rebuild the distributed executable, including build scripts and GPL components, and offer it with equivalent access alongside the binary. Do not rely only on a moving upstream repository link.
3. Include applicable GPL terms for the combined work, the original MIT notice, all dependency copyright/license/NOTICE files, and font notices. Check the exact resolved Rust/npm tree as well as the separately linked native libraries.
4. Keep downloadable models separate unless each model, dictionary and data asset has also passed a redistribution review.
5. Build using `npm ci` and `npm run tauri build -- --no-bundle`. Test launch on Windows x64 with WebView2, SAPI preview, OSC send/typing, Chinese IME confirmation, audio output selection, and model download interruption/retry/deletion.
6. Package the executable with licenses and notices, publish a SHA-256 manifest, and state whether it is code-signed. Do not advertise an unsigned download as inherently safe.

CI runs checks but deliberately does not publish binaries. A successful build or a vulnerability scan does not establish license compliance or replace the Windows interaction checks.

## Rewritten Git history

History cleanup changes commit IDs. Keep the pre-cleanup bundle private; it is a recovery copy containing the original metadata. Do not publish tool-owned refs, reflogs, or backup bundles.

Before updating an existing remote, inspect its current branch tip and coordinate with collaborators. Use an explicit expected old commit with `--force-with-lease` if overwriting the old history is intended. Avoid `--mirror`: it can publish internal refs. Existing clones, cached views, old releases and pull request refs can retain previous history independently of the local branch.

The local rewrite does not change GitHub-hosted history by itself. Do not merge the old remote history back into the cleaned branch.
