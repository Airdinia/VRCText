//! Background downloader for sherpa-onnx TTS model bundles. Progress is
//! surfaced to the UI via `AppHandle.emit`.
//!
//! Two packs are currently exposed:
//!   • Matcha zh-baker    — Mandarin Chinese single voice, ~85 MB
//!                          (bundled with the universal vocos vocoder)
//!   • Kokoro multi-lang  — Multilingual v1.1, ~350 MB full precision,
//!                          103 voices across en / zh / ja / ko / fr
//!
//! Packs are extracted into `%APPDATA%\vrctext\models\` and discovered by the
//! TTS layer on startup.

// `size_hint` etc. live on as a public API for future settings copy.
#![allow(dead_code)]

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

const VOCOS_URL: &str =
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/vocoder-models/vocos-22khz-univ.onnx";
const MATCHA_URL: &str =
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/matcha-icefall-zh-baker.tar.bz2";
const KOKORO_URL: &str =
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/kokoro-multi-lang-v1_1.tar.bz2";

/// Which pack the downloader is fetching.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelKind {
    MatchaZhBaker,
    KokoroMultiLang,
}

impl ModelKind {
    pub const ALL: &'static [ModelKind] =
        &[ModelKind::MatchaZhBaker, ModelKind::KokoroMultiLang];

    /// i18n dictionary key for the user-facing pack name. Resolved by the
    /// frontend's `maybeT()` so the label flips with UI language.
    pub fn i18n_key(self) -> &'static str {
        match self {
            ModelKind::MatchaZhBaker => "matchaName",
            ModelKind::KokoroMultiLang => "kokoroName",
        }
    }

    pub fn size_hint(self) -> &'static str {
        match self {
            ModelKind::MatchaZhBaker => "~85 MB",
            ModelKind::KokoroMultiLang => "~350 MB",
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
                let model =
                    d.join("model.onnx").exists() || d.join("model.int8.onnx").exists();
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
    pub kind: ModelKind,
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
        Self { kind, state }
    }

    pub fn snapshot(&self) -> DownloadState {
        self.state.lock().unwrap().clone()
    }
}

fn run(kind: ModelKind, models_dir: &Path, state: &Arc<Mutex<DownloadState>>) -> Result<(), String> {
    std::fs::create_dir_all(models_dir).map_err(|e| format!("@i18n:dlCreateDirFail|{e}"))?;
    match kind {
        ModelKind::MatchaZhBaker => run_matcha(models_dir, state),
        ModelKind::KokoroMultiLang => run_kokoro(models_dir, state),
    }
}

fn run_matcha(models_dir: &Path, state: &Arc<Mutex<DownloadState>>) -> Result<(), String> {
    let vocos_path = models_dir.join("vocos-22khz-univ.onnx");
    if !vocos_path.exists() {
        set_status(state, "@i18n:dlVocoder", 0.0);
        download_to(VOCOS_URL, &vocos_path, state, (0.0, 0.25))?;
    }
    let tmp = models_dir.join("matcha-icefall-zh-baker.tar.bz2");
    set_status(state, "@i18n:dlMatcha", 0.25);
    download_to(MATCHA_URL, &tmp, state, (0.25, 0.9))?;
    set_status(state, "@i18n:dlExtract", 0.9);
    extract(&tmp, models_dir)?;
    let _ = std::fs::remove_file(&tmp);
    verify_post_extract(ModelKind::MatchaZhBaker, models_dir)
}

fn run_kokoro(models_dir: &Path, state: &Arc<Mutex<DownloadState>>) -> Result<(), String> {
    let tmp = models_dir.join("kokoro-multi-lang-v1_1.tar.bz2");
    set_status(state, "@i18n:dlKokoro", 0.0);
    download_to(KOKORO_URL, &tmp, state, (0.0, 0.9))?;
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
    let total: Option<u64> = resp
        .header("Content-Length")
        .and_then(|s| s.parse().ok());
    let mut reader = resp.into_reader();
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
        file.write_all(&buf[..n])
            .map_err(|e| format!("@i18n:dlWriteData|{e}"))?;
        downloaded += n as u64;
        if let Some(total) = total {
            let frac = (downloaded as f32 / total as f32).min(1.0);
            let mapped = range.0 + (range.1 - range.0) * frac;
            state.lock().unwrap().progress = mapped;
        }
    }
    Ok(())
}

fn extract(archive: &Path, dest: &Path) -> Result<(), String> {
    let file = File::open(archive).map_err(|e| format!("@i18n:dlOpenArchive|{e}"))?;
    let decoder = bzip2::read::BzDecoder::new(file);
    let mut tar = tar::Archive::new(decoder);
    tar.unpack(dest).map_err(|e| format!("@i18n:dlExtractFail|{e}"))?;
    Ok(())
}

pub fn installed_kinds(models_dir: &Path) -> Vec<ModelKind> {
    ModelKind::ALL
        .iter()
        .copied()
        .filter(|k| k.is_installed(models_dir))
        .collect()
}
