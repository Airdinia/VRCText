use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

use windows::core::{Interface, PCWSTR, PWSTR};
use windows::Win32::Media::Audio::{waveOutGetDevCapsW, waveOutGetNumDevs, WAVEOUTCAPSW};

const MMSYSERR_NOERROR: u32 = 0;
const SPCAT_VOICES: &str = r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Speech\Voices";

use windows::Win32::Media::Speech::{
    ISpMMSysAudio, ISpObjectToken, ISpObjectTokenCategory, ISpVoice, SpMMAudioOut,
    SpObjectTokenCategory, SpVoice, SPF_ASYNC, SPF_PURGEBEFORESPEAK,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};

pub const DEVICE_DEFAULT: u32 = u32::MAX;
pub const DEFAULT_DEVICE_LABEL: &str = "系统默认";
pub const DEFAULT_VOICE_LABEL: &str = "系统默认";

#[derive(Clone, Debug)]
pub struct TtsDevice {
    pub id: u32,
    pub name: String,
}

pub struct TtsEngine {
    voice: Option<ISpVoice>,
    current_name: Option<String>,
}

impl TtsEngine {
    pub fn new() -> Self {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let voice = CoCreateInstance::<_, ISpVoice>(&SpVoice, None, CLSCTX_ALL).ok();
            Self {
                voice,
                current_name: None,
            }
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

    pub fn list_devices(&self) -> Vec<TtsDevice> {
        let mut out = vec![TtsDevice {
            id: DEVICE_DEFAULT,
            name: DEFAULT_DEVICE_LABEL.into(),
        }];
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
                if name.is_empty() {
                    continue;
                }
                out.push(TtsDevice { id: i, name });
            }
        }
        out
    }

    /// Apply a device by user-visible name. `None` or an unmatched name ⇒ system default.
    /// Returns the name that ended up active (None = default).
    pub fn apply_device_by_name(&mut self, name: Option<&str>) -> Option<String> {
        let devices = self.list_devices();
        let chosen = name
            .and_then(|n| devices.iter().find(|d| d.name == n && d.id != DEVICE_DEFAULT))
            .cloned()
            .unwrap_or_else(|| devices[0].clone());

        if self.set_device(chosen.id) {
            if chosen.id == DEVICE_DEFAULT {
                self.current_name = None;
                None
            } else {
                self.current_name = Some(chosen.name.clone());
                Some(chosen.name)
            }
        } else {
            self.current_name = None;
            None
        }
    }

    pub fn list_voice_names(&self) -> Vec<String> {
        enum_voice_tokens()
            .into_iter()
            .map(|(name, _)| name)
            .collect()
    }

    /// Apply a voice by its description. `None` or unmatched name ⇒ SAPI default voice.
    /// Returns the name that ended up active (None = default).
    pub fn apply_voice_by_name(&mut self, name: Option<&str>) -> Option<String> {
        let Some(voice) = &self.voice else { return None; };
        unsafe {
            let Some(target) = name else {
                let _ = voice.SetVoice(None);
                return None;
            };
            for (token_name, token) in enum_voice_tokens() {
                if token_name == target {
                    if voice.SetVoice(&token).is_ok() {
                        return Some(token_name);
                    }
                    break;
                }
            }
            let _ = voice.SetVoice(None);
            None
        }
    }

    fn set_device(&mut self, device_id: u32) -> bool {
        let Some(voice) = &self.voice else { return false; };
        unsafe {
            if device_id == DEVICE_DEFAULT {
                return voice.SetOutput(None, true).is_ok();
            }
            let audio: ISpMMSysAudio =
                match CoCreateInstance(&SpMMAudioOut, None, CLSCTX_ALL) {
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
            if desc.is_empty() {
                continue;
            }
            out.push((desc, token));
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
