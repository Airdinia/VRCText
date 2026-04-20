//! Background downloader for the Matcha-TTS Chinese model bundle.
//!
//! Pulls two artifacts from the official sherpa-onnx release pages into
//! `%APPDATA%\vrctext\models\` and extracts the tar.bz2 voice bundle. The
//! UI thread polls `DownloadState` each frame to paint progress / errors.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

const VOCOS_URL: &str =
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/vocoder-models/vocos-22khz-univ.onnx";
const BAKER_URL: &str =
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/matcha-icefall-zh-baker.tar.bz2";

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
    pub fn start(models_dir: PathBuf) -> Self {
        let state = Arc::new(Mutex::new(DownloadState {
            status: "准备下载…".into(),
            progress: 0.0,
            done: false,
            error: None,
        }));
        let state_clone = state.clone();
        thread::spawn(move || {
            let result = run(&models_dir, &state_clone);
            let mut s = state_clone.lock().unwrap();
            match result {
                Ok(()) => {
                    s.status = "完成".into();
                    s.progress = 1.0;
                }
                Err(msg) => s.error = Some(msg),
            }
            s.done = true;
        });
        Self { state }
    }

    pub fn snapshot(&self) -> DownloadState {
        self.state.lock().unwrap().clone()
    }
}

fn run(models_dir: &Path, state: &Arc<Mutex<DownloadState>>) -> Result<(), String> {
    std::fs::create_dir_all(models_dir).map_err(|e| format!("创建目录失败: {e}"))?;

    let vocos_path = models_dir.join("vocos-22khz-univ.onnx");
    if !vocos_path.exists() {
        set_status(state, "下载声码器 (~24 MB)", 0.0);
        download_to(VOCOS_URL, &vocos_path, state, (0.0, 0.25))?;
    }

    let tmp_archive = models_dir.join("matcha-icefall-zh-baker.tar.bz2");
    set_status(state, "下载 Matcha 中文模型 (~60 MB)", 0.25);
    download_to(BAKER_URL, &tmp_archive, state, (0.25, 0.9))?;

    set_status(state, "解压…", 0.9);
    extract(&tmp_archive, models_dir)?;

    let _ = std::fs::remove_file(&tmp_archive);

    // Verify the bundle laid itself out where `SherpaEngine` expects. The tar
    // crate returns `Ok` even when archives extract into unusual sub-paths,
    // so a successful `unpack()` alone isn't proof the engine can load.
    verify_layout(models_dir)?;
    Ok(())
}

fn verify_layout(models_dir: &Path) -> Result<(), String> {
    let voice_dir = models_dir.join("matcha-icefall-zh-baker");
    let acoustic_ok = ["model-steps-3.onnx", "model-steps-6.onnx"]
        .iter()
        .any(|n| voice_dir.join(n).exists());
    let tokens_ok = voice_dir.join("tokens.txt").exists();
    let vocoder_ok = models_dir.join("vocos-22khz-univ.onnx").exists();
    if acoustic_ok && tokens_ok && vocoder_ok {
        return Ok(());
    }
    Err(format!(
        "解压完成但关键文件缺失: acoustic={} tokens={} vocoder={}。\
         请在设置里点'打开模型目录'自查。",
        if acoustic_ok { "✔" } else { "✘" },
        if tokens_ok { "✔" } else { "✘" },
        if vocoder_ok { "✔" } else { "✘" },
    ))
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
    let resp = ureq::get(url)
        .call()
        .map_err(|e| format!("HTTP 请求失败: {e}"))?;
    let total: Option<u64> = resp
        .header("Content-Length")
        .and_then(|s| s.parse().ok());
    let mut reader = resp.into_reader();
    let mut file = File::create(dest).map_err(|e| format!("写文件失败: {e}"))?;
    let mut buf = vec![0u8; 64 * 1024];
    let mut downloaded: u64 = 0;
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("下载中断: {e}"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])
            .map_err(|e| format!("写入失败: {e}"))?;
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
    let file = File::open(archive).map_err(|e| format!("打开压缩包失败: {e}"))?;
    let decoder = bzip2::read::BzDecoder::new(file);
    let mut tar = tar::Archive::new(decoder);
    tar.unpack(dest).map_err(|e| format!("解压失败: {e}"))?;
    Ok(())
}
