use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

use windows::core::{Interface, PCWSTR, PWSTR};
use windows::Win32::Media::Audio::{waveOutGetDevCapsW, waveOutGetNumDevs, WAVEOUTCAPSW};
use windows::Win32::Media::Speech::{
    ISpMMSysAudio, ISpObjectToken, ISpObjectTokenCategory, ISpVoice, SpMMAudioOut,
    SpObjectTokenCategory, SpVoice, SPF_ASYNC, SPF_PURGEBEFORESPEAK,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};

const MMSYSERR_NOERROR: u32 = 0;
const DEVICE_DEFAULT: u32 = u32::MAX;
const DEFAULT_LABEL: &str = "系统默认";
const SPCAT_VOICES: &str = r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Speech\Voices";

/// A selectable item (audio device or voice) rendered in the settings UI.
/// `key = None` means "system default" — the sentinel is kept entirely inside
/// this module so callers never see `u32::MAX` or re-implement the default row.
#[derive(Clone, Debug)]
pub struct Choice {
    pub label: String,
    pub key: Option<String>,
}

pub struct TtsEngine {
    voice: Option<ISpVoice>,
}

impl TtsEngine {
    pub fn new() -> Self {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let voice = CoCreateInstance::<_, ISpVoice>(&SpVoice, None, CLSCTX_ALL).ok();
            Self { voice }
        }
    }

    pub fn available(&self) -> bool {
        self.voice.is_some()
    }

    pub fn speak(&mut self, text: &str) {
        if text.trim().is_empty() {
            return;
        }
        let Some(voice) = &self.voice else { return; };
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let flags = SPF_ASYNC.0 as u32 | SPF_PURGEBEFORESPEAK.0 as u32;
        unsafe {
            let _ = voice.Speak(PCWSTR(wide.as_ptr()), flags, None);
        }
    }

    pub fn stop(&mut self) {
        let Some(voice) = &self.voice else { return; };
        let empty = [0u16];
        unsafe {
            let _ = voice.Speak(
                PCWSTR(empty.as_ptr()),
                SPF_PURGEBEFORESPEAK.0 as u32,
                None,
            );
        }
    }

    pub fn device_choices(&self) -> Vec<Choice> {
        let mut out = vec![Choice {
            label: DEFAULT_LABEL.into(),
            key: None,
        }];
        for (_, name) in enumerate_wave_out() {
            out.push(Choice {
                label: name.clone(),
                key: Some(name),
            });
        }
        out
    }

    pub fn voice_choices(&self) -> Vec<Choice> {
        let mut out = vec![Choice {
            label: DEFAULT_LABEL.into(),
            key: None,
        }];
        for (name, _) in enum_voice_tokens() {
            out.push(Choice {
                label: shorten_voice_label(&name),
                key: Some(name),
            });
        }
        out
    }

    /// Apply a device by persisted key; returns the key that ended up active
    /// (None = default). An unmatched key falls back to default.
    pub fn apply_device(&mut self, key: Option<&str>) -> Option<String> {
        let Some(voice) = &self.voice else { return None; };
        if let Some(want) = key {
            if let Some((id, name)) = enumerate_wave_out()
                .into_iter()
                .find(|(_, name)| name == want)
            {
                if set_voice_output(voice, id) {
                    return Some(name);
                }
            }
        }
        let _ = unsafe { voice.SetOutput(None, true) };
        None
    }

    /// Apply a voice by persisted key; returns the key that ended up active
    /// (None = default). An unmatched key falls back to default.
    pub fn apply_voice(&mut self, key: Option<&str>) -> Option<String> {
        let Some(voice) = &self.voice else { return None; };
        unsafe {
            if let Some(want) = key {
                for (token_name, token) in enum_voice_tokens() {
                    if token_name == want && voice.SetVoice(&token).is_ok() {
                        return Some(token_name);
                    }
                }
            }
            let _ = voice.SetVoice(None);
            None
        }
    }
}

fn set_voice_output(voice: &ISpVoice, device_id: u32) -> bool {
    unsafe {
        if device_id == DEVICE_DEFAULT {
            return voice.SetOutput(None, true).is_ok();
        }
        let audio: ISpMMSysAudio = match CoCreateInstance(&SpMMAudioOut, None, CLSCTX_ALL) {
            Ok(a) => a,
            Err(_) => return false,
        };
        if audio.SetDeviceId(device_id).is_err() {
            return false;
        }
        let unk: windows::core::IUnknown = match audio.cast() {
            Ok(u) => u,
            Err(_) => return false,
        };
        voice.SetOutput(&unk, true).is_ok()
    }
}

/// Trim SAPI voice descriptions for compact display in the UI.
/// Example: "Microsoft Huihui Desktop - Chinese (Simplified, PRC)"
///       → "Microsoft Huihui"
fn shorten_voice_label(name: &str) -> String {
    let base = name.split(" - ").next().unwrap_or(name);
    base.replace(" Desktop", "").replace(" Server", "").trim().to_string()
}

fn enumerate_wave_out() -> Vec<(u32, String)> {
    let mut out = Vec::new();
    unsafe {
        let n = waveOutGetNumDevs();
        for i in 0..n {
            let mut caps: WAVEOUTCAPSW = std::mem::zeroed();
            let res = waveOutGetDevCapsW(
                i as usize,
                &mut caps,
                std::mem::size_of::<WAVEOUTCAPSW>() as u32,
            );
            if res != MMSYSERR_NOERROR {
                continue;
            }
            let pname: [u16; 32] = std::ptr::addr_of!(caps.szPname).read_unaligned();
            let len = pname.iter().position(|&c| c == 0).unwrap_or(pname.len());
            let name = OsString::from_wide(&pname[..len])
                .to_string_lossy()
                .into_owned();
            if !name.is_empty() {
                out.push((i, name));
            }
        }
    }
    out
}

fn enum_voice_tokens() -> Vec<(String, ISpObjectToken)> {
    let mut out = Vec::new();
    unsafe {
        let cat: ISpObjectTokenCategory =
            match CoCreateInstance(&SpObjectTokenCategory, None, CLSCTX_ALL) {
                Ok(c) => c,
                Err(_) => return out,
            };
        let cid: Vec<u16> = SPCAT_VOICES
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        if cat.SetId(PCWSTR(cid.as_ptr()), false).is_err() {
            return out;
        }
        let enum_tokens = match cat.EnumTokens(PCWSTR::null(), PCWSTR::null()) {
            Ok(e) => e,
            Err(_) => return out,
        };
        loop {
            let mut slot: Option<ISpObjectToken> = None;
            let mut fetched: u32 = 0;
            if enum_tokens
                .Next(1, &mut slot as *mut _, Some(&mut fetched as *mut _))
                .is_err()
                || fetched == 0
            {
                break;
            }
            let Some(token) = slot.take() else { continue; };
            let desc = match token.GetStringValue(PCWSTR::null()) {
                Ok(p) => pwstr_to_string_and_free(p),
                Err(_) => continue,
            };
            if !desc.is_empty() {
                out.push((desc, token));
            }
        }
    }
    out
}

unsafe fn pwstr_to_string_and_free(p: PWSTR) -> String {
    if p.is_null() {
        return String::new();
    }
    let mut len = 0usize;
    while *p.0.add(len) != 0 {
        len += 1;
    }
    let slice = std::slice::from_raw_parts(p.0, len);
    let s = OsString::from_wide(slice).to_string_lossy().into_owned();
    CoTaskMemFree(Some(p.0 as _));
    s
}
