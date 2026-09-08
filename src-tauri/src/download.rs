//! Background downloader for sherpa-onnx TTS model bundles. Progress is
//! surfaced to the UI via `AppHandle.emit`.
//!
//! Kokoro v1.1 is offered for download (Chinese / English, ~365 MB).
//! Existing Matcha installations remain readable for compatibility; new
//! Matcha downloads are not offered because the archive has unclear model
//! licensing and identifies its training dataset as non-commercial only.
//!
//! Packs are extracted into `%APPDATA%\vrctext\vrctext\data\models\` and discovered by the
//! TTS layer on startup.

use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

const KOKORO_URL: &str =
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/kokoro-multi-lang-v1_1.tar.bz2";

// Upstream GitHub release metadata, checked against the asset on 2026-09-08.
const KOKORO_SHA256: &str = "a3f4c73d043860e3fd2e5b06f36795eb81de0fc8e8de6df703245edddd87dbad";

/// Which pack the downloader is fetching.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelKind {
    MatchaZhBaker,
    KokoroMultiLang,
}

impl ModelKind {
    pub const ALL: &'static [ModelKind] = &[ModelKind::MatchaZhBaker, ModelKind::KokoroMultiLang];

    /// i18n dictionary key for the user-facing pack name. Resolved by the
    /// frontend's `maybeT()` so the label flips with UI language.
    pub fn i18n_key(self) -> &'static str {
        match self {
            ModelKind::MatchaZhBaker => "matchaName",
            ModelKind::KokoroMultiLang => "kokoroName",
        }
    }

    pub fn pack_dir(self) -> &'static str {
        match self {
            ModelKind::MatchaZhBaker => "matcha-icefall-zh-baker",
            ModelKind::KokoroMultiLang => "kokoro-multi-lang-v1_1",
        }
    }

    pub fn is_installed(self, models_dir: &Path) -> bool {
        let d = models_dir.join(self.pack_dir());
        match self {
            ModelKind::MatchaZhBaker => {
                let acoustic = ["model-steps-3.onnx", "model-steps-6.onnx"]
                    .iter()
                    .any(|n| d.join(n).exists());
                acoustic
                    && d.join("tokens.txt").exists()
                    && models_dir.join("vocos-22khz-univ.onnx").exists()
            }
            ModelKind::KokoroMultiLang => {
                let model = d.join("model.onnx").exists() || d.join("model.int8.onnx").exists();
                model
                    && d.join("voices.bin").exists()
                    && d.join("tokens.txt").exists()
                    && d.join("espeak-ng-data").is_dir()
            }
        }
    }
}

#[derive(Clone, Default)]
pub struct DownloadState {
    pub status: String,
    pub progress: f32,
    pub done: bool,
    pub error: Option<String>,
}

pub struct ModelDownloader {
    pub state: Arc<Mutex<DownloadState>>,
}

impl ModelDownloader {
    pub fn start(kind: ModelKind, models_dir: PathBuf) -> Self {
        let state = Arc::new(Mutex::new(DownloadState {
            status: format!("@i18n:dlPrepare|{}", kind.i18n_key()),
            progress: 0.0,
            done: false,
            error: None,
        }));
        let state_clone = state.clone();
        thread::spawn(move || {
            let result = run(kind, &models_dir, &state_clone);
            let mut s = state_clone.lock().unwrap();
            match result {
                Ok(()) => {
                    s.status = "@i18n:dlComplete".into();
                    s.progress = 1.0;
                }
                Err(msg) => s.error = Some(msg),
            }
            s.done = true;
        });
        Self { state }
    }
}

fn run(
    kind: ModelKind,
    models_dir: &Path,
    state: &Arc<Mutex<DownloadState>>,
) -> Result<(), String> {
    std::fs::create_dir_all(models_dir).map_err(|e| format!("@i18n:dlCreateDirFail|{e}"))?;
    // Never expose partial downloads or extraction results to the model loader.
    // Downloads are serialized by AppState; a stale staging directory is safe
    // to discard before retrying after a crash.
    let staging = models_dir.join(".download-staging");
    if staging.exists() {
        std::fs::remove_dir_all(&staging).map_err(|e| format!("@i18n:dlWriteFile|{e}"))?;
    }
    std::fs::create_dir(&staging).map_err(|e| format!("@i18n:dlCreateDirFail|{e}"))?;
    let result = (|| {
        match kind {
            ModelKind::MatchaZhBaker => return Err("Matcha downloads are no longer offered".into()),
            ModelKind::KokoroMultiLang => run_kokoro(&staging, state)?,
        }
        let target = models_dir.join(kind.pack_dir());
        // An explicit re-download replaces an existing pack only after staging
        // has passed validation. A failed transfer leaves the old pack intact.
        if target.exists() {
            std::fs::remove_dir_all(&target).map_err(|e| format!("@i18n:dlWriteFile|{e}"))?;
        }
        std::fs::rename(staging.join(kind.pack_dir()), target)
            .map_err(|e| format!("@i18n:dlWriteFile|{e}"))
    })();
    let _ = std::fs::remove_dir_all(&staging);
    result
}

fn run_kokoro(models_dir: &Path, state: &Arc<Mutex<DownloadState>>) -> Result<(), String> {
    let tmp = models_dir.join("kokoro-multi-lang-v1_1.tar.bz2");
    set_status(state, "@i18n:dlKokoro", 0.0);
    download_to(KOKORO_URL, KOKORO_SHA256, &tmp, state, (0.0, 0.9))?;
    set_status(state, "@i18n:dlExtractLarge", 0.9);
    extract(&tmp, models_dir)?;
    let _ = std::fs::remove_file(&tmp);
    verify_post_extract(ModelKind::KokoroMultiLang, models_dir)
}

fn verify_post_extract(kind: ModelKind, models_dir: &Path) -> Result<(), String> {
    if kind.is_installed(models_dir) {
        return Ok(());
    }
    Err(format!("@i18n:dlVerifyMissing|{}", kind.i18n_key()))
}

fn set_status(state: &Arc<Mutex<DownloadState>>, msg: &str, progress: f32) {
    let mut s = state.lock().unwrap();
    s.status = msg.into();
    s.progress = progress;
}

fn download_to(
    url: &str,
    expected_sha256: &str,
    dest: &Path,
    state: &Arc<Mutex<DownloadState>>,
    range: (f32, f32),
) -> Result<(), String> {
    // Explicit timeouts: without them a stalled connection blocks the read
    // forever and the UI stays on "已有下载在进行中" until an app restart.
    // No overall timeout — the Kokoro pack legitimately takes minutes; the
    // read timeout only fires when the socket goes silent.
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(15))
        .timeout_read(std::time::Duration::from_secs(30))
        .build();
    let resp = agent
        .get(url)
        .call()
        .map_err(|e| format!("@i18n:dlHttp|{e}"))?;
    let total: Option<u64> = resp.header("Content-Length").and_then(|s| s.parse().ok());
    let mut reader = resp.into_reader();
    write_download(&mut reader, dest, total, state, range)?;
    verify_digest(dest, expected_sha256)
}

fn verify_digest(path: &Path, expected: &str) -> Result<(), String> {
    let mut file = File::open(path).map_err(|e| format!("@i18n:dlOpenArchive|{e}"))?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file
            .read(&mut buffer)
            .map_err(|e| format!("@i18n:dlInterrupted|{e}"))?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    if format!("{:x}", hash.finalize()) != expected {
        return Err("@i18n:dlChecksumMismatch".into());
    }
    Ok(())
}

fn write_download(
    reader: &mut impl Read,
    dest: &Path,
    total: Option<u64>,
    state: &Arc<Mutex<DownloadState>>,
    range: (f32, f32),
) -> Result<(), String> {
    const MAX_DOWNLOAD_BYTES: u64 = 1024 * 1024 * 1024;
    let mut file = File::create(dest).map_err(|e| format!("@i18n:dlWriteFile|{e}"))?;
    let mut buf = vec![0u8; 64 * 1024];
    let mut downloaded: u64 = 0;
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("@i18n:dlInterrupted|{e}"))?;
        if n == 0 {
            break;
        }
        downloaded += n as u64;
        if downloaded > MAX_DOWNLOAD_BYTES {
            return Err("@i18n:dlSizeInvalid".into());
        }
        file.write_all(&buf[..n])
            .map_err(|e| format!("@i18n:dlWriteData|{e}"))?;
        if let Some(total) = total.filter(|n| *n > 0) {
            let frac = (downloaded as f32 / total as f32).min(1.0);
            let mapped = range.0 + (range.1 - range.0) * frac;
            state.lock().unwrap().progress = mapped;
        }
    }
    if downloaded == 0 || total.is_some_and(|n| n != downloaded) {
        return Err("@i18n:dlSizeInvalid".into());
    }
    file.sync_all()
        .map_err(|e| format!("@i18n:dlWriteData|{e}"))
}

fn extract(archive: &Path, dest: &Path) -> Result<(), String> {
    let file = File::open(archive).map_err(|e| format!("@i18n:dlOpenArchive|{e}"))?;
    let decoder = bzip2::read::BzDecoder::new(file);
    let mut tar = tar::Archive::new(decoder);
    extract_entries(&mut tar, dest).map_err(|e| format!("@i18n:dlExtractFail|{e}"))?;
    Ok(())
}

fn extract_entries<R: Read>(archive: &mut tar::Archive<R>, dest: &Path) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};
    use std::path::Component;
    let invalid = || Error::new(ErrorKind::InvalidData, "unsafe or oversized model archive");
    let mut bytes = 0u64;
    for (count, entry) in archive.entries()?.enumerate() {
        let mut entry = entry?;
        let kind = entry.header().entry_type();
        let path = entry.path()?;
        if count >= 50_000
            || !(kind.is_file() || kind.is_dir())
            || path
                .components()
                .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
        {
            return Err(invalid());
        }
        bytes = bytes.checked_add(entry.size()).ok_or_else(invalid)?;
        if bytes > 2 * 1024 * 1024 * 1024 || !entry.unpack_in(dest)? {
            return Err(invalid());
        }
    }
    Ok(())
}

pub fn installed_kinds(models_dir: &Path) -> Vec<ModelKind> {
    ModelKind::ALL
        .iter()
        .copied()
        .filter(|k| k.is_installed(models_dir))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Set VRCTEXT_MODEL_FIXTURES to the directory containing the official
    /// Kokoro archive, then run this test explicitly with --ignored.
    #[test]
    #[ignore = "requires the official Kokoro archive (about 365 MB)"]
    fn official_model_archives() {
        let fixtures = PathBuf::from(
            std::env::var_os("VRCTEXT_MODEL_FIXTURES")
                .expect("set VRCTEXT_MODEL_FIXTURES to the downloaded assets directory"),
        );
        let dir = temp_dir();
        let kind = ModelKind::KokoroMultiLang;
        let archive = fixtures.join(format!("{}.tar.bz2", kind.pack_dir()));
        verify_digest(&archive, KOKORO_SHA256).unwrap();
        extract(&archive, &dir).unwrap();
        verify_post_extract(kind, &dir).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vrctext-download-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        dir
    }

    #[test]
    fn truncated_and_empty_responses_are_rejected() {
        let dir = temp_dir();
        let state = Arc::new(Mutex::new(DownloadState::default()));
        let file = dir.join("model");
        assert!(write_download(&mut &b"abc"[..], &file, Some(5), &state, (0.0, 1.0)).is_err());
        assert!(write_download(&mut &b""[..], &file, None, &state, (0.0, 1.0)).is_err());
        assert!(write_download(&mut &b"abc"[..], &file, Some(3), &state, (0.0, 1.0)).is_ok());
        assert!(verify_digest(
            &file,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        )
        .is_ok());
        assert!(verify_digest(&file, &"0".repeat(64)).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn archive_links_are_rejected() {
        let dir = temp_dir();
        let mut builder = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_size(0);
        header.set_mode(0o777);
        builder
            .append_link(&mut header, "model/link", "../../outside")
            .unwrap();
        let data = builder.into_inner().unwrap();
        assert!(extract_entries(&mut tar::Archive::new(&data[..]), &dir).is_err());
        assert!(!dir.join("model/link").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
