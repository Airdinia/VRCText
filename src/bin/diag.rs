use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::{thread, time::Duration};

use windows::core::{Interface, PCWSTR};
use windows::Win32::Media::Audio::{waveOutGetDevCapsW, waveOutGetNumDevs, WAVEOUTCAPSW};
use windows::Win32::Media::Speech::{
    ISpMMSysAudio, ISpVoice, SpMMAudioOut, SpVoice, SPF_PURGEBEFORESPEAK,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};

fn main() {
    unsafe {
        let ci = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        println!("CoInitializeEx(APARTMENTTHREADED) => {:?}", ci);

        // ── step 1: enumerate WaveOut devices
        let n = waveOutGetNumDevs();
        println!("\nwaveOutGetNumDevs() => {}", n);
        for i in 0..n {
            let mut caps: WAVEOUTCAPSW = std::mem::zeroed();
            let res = waveOutGetDevCapsW(
                i as usize,
                &mut caps,
                std::mem::size_of::<WAVEOUTCAPSW>() as u32,
            );
            let pname: [u16; 32] = std::ptr::addr_of!(caps.szPname).read_unaligned();
            let len = pname.iter().position(|&c| c == 0).unwrap_or(pname.len());
            let name = OsString::from_wide(&pname[..len])
                .to_string_lossy()
                .into_owned();
            println!("  [{}] (res={}) name={:?}", i, res, name);
        }

        // ── step 2: create ISpVoice
        println!("\nCreating ISpVoice …");
        let voice: ISpVoice = match CoCreateInstance(&SpVoice, None, CLSCTX_ALL) {
            Ok(v) => {
                println!("  OK");
                v
            }
            Err(e) => {
                println!("  FAIL: {:?}", e);
                return;
            }
        };

        // ── step 3a: speak with DEFAULT output (no SetOutput call)
        println!("\n[A] Speak with default output (no SetOutput):");
        let text = "测试 A 默认输出";
        speak_sync(&voice, text);
        thread::sleep(Duration::from_millis(2500));

        // ── step 3b: try SetOutput(None) then speak
        println!("\n[B] Speak after SetOutput(None):");
        let r = voice.SetOutput(None, true);
        println!("  SetOutput(None) => {:?}", r);
        speak_sync(&voice, "测试 B SetOutput None");
        thread::sleep(Duration::from_millis(2500));

        // ── step 3c: SetOutput with SpMMAudioOut on device 0
        if n > 0 {
            println!("\n[C] Route via SpMMAudioOut to device 0:");
            match CoCreateInstance::<_, ISpMMSysAudio>(&SpMMAudioOut, None, CLSCTX_ALL) {
                Ok(audio) => {
                    let r = audio.SetDeviceId(0);
                    println!("  audio.SetDeviceId(0) => {:?}", r);
                    match audio.cast::<windows::core::IUnknown>() {
                        Ok(unk) => {
                            let r = voice.SetOutput(&unk, true);
                            println!("  voice.SetOutput(&IUnknown, true) => {:?}", r);
                        }
                        Err(e) => println!("  cast IUnknown fail: {:?}", e),
                    }
                    // Alternative: pass &audio directly
                    let r2 = voice.SetOutput(&audio, true);
                    println!("  voice.SetOutput(&audio, true) [direct]  => {:?}", r2);
                    speak_sync(&voice, "测试 C 通过 SpMMAudioOut 到设备 0");
                    thread::sleep(Duration::from_millis(2500));
                }
                Err(e) => println!("  CoCreateInstance SpMMAudioOut FAIL: {:?}", e),
            }
        }

        // ── step 3d: SetOutput(None) then speak again, to confirm recovery
        println!("\n[D] Speak after restoring SetOutput(None):");
        let r = voice.SetOutput(None, true);
        println!("  SetOutput(None) => {:?}", r);
        speak_sync(&voice, "测试 D 恢复默认");
        thread::sleep(Duration::from_millis(2500));

        println!("\nDiagnostic complete.");
    }
}

unsafe fn speak_sync(voice: &ISpVoice, text: &str) {
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    // Sync speak: no ASYNC flag — call blocks until done
    let flags = SPF_PURGEBEFORESPEAK.0 as u32;
    let r = voice.Speak(PCWSTR(wide.as_ptr()), flags, None);
    println!("  Speak({:?}) => {:?}", text, r);
}
