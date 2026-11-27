//! The volume of the music app: the audio session of the playing program (Spotify, the browser ...), or the main volume of the
//! speakers when that program has no session of its own. Windows' own Core Audio, no extra libraries.

use windows::core::Interface;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eMultimedia, eRender, IAudioSessionControl2, IAudioSessionManager2, IMMDevice, IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};

fn device() -> Option<IMMDevice> {
    unsafe {
        let e: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
        e.GetDefaultAudioEndpoint(eRender, eMultimedia).ok()
    }
}

/// "Spotify.exe" / "spotifyab.spotifymusic_xyz!spotify" -> "spotify"
pub fn stem_of(app: &str) -> String {
    let tail = app.rsplit('!').next().unwrap_or(app).to_lowercase();
    tail.trim_end_matches(".exe").to_string()
}

fn process_stem(pid: u32) -> String {
    unsafe {
        let Ok(p) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return String::new() };
        let mut buf = [0u16; 520];
        let mut n = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(p, PROCESS_NAME_WIN32, windows::core::PWSTR(buf.as_mut_ptr()), &mut n).is_ok();
        let _ = CloseHandle(p);
        if !ok {
            return String::new();
        }
        let path = String::from_utf16_lossy(&buf[..n as usize]).to_lowercase();
        std::path::Path::new(&path).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
    }
}

/// The audio sessions of the program that plays the music.
fn sessions(dev: &IMMDevice, stem: &str) -> Vec<ISimpleAudioVolume> {
    let mut out = Vec::new();
    if stem.is_empty() {
        return out;
    }
    unsafe {
        let Ok(mgr) = dev.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) else { return out };
        let Ok(list) = mgr.GetSessionEnumerator() else { return out };
        let n = list.GetCount().unwrap_or(0);
        for i in 0..n {
            let Ok(ctl) = list.GetSession(i) else { continue };
            let Ok(c2) = ctl.cast::<IAudioSessionControl2>() else { continue };
            let Ok(pid) = c2.GetProcessId() else { continue };
            if pid == 0 {
                continue;
            }
            let name = process_stem(pid);
            if !name.is_empty() && (name.contains(stem) || stem.contains(&name)) {
                if let Ok(v) = ctl.cast::<ISimpleAudioVolume>() {
                    out.push(v);
                }
            }
        }
    }
    out
}

/// The volume of the music app (0..1).
pub fn get(app: &str) -> Option<f32> {
    let dev = device()?;
    unsafe {
        let s = sessions(&dev, &stem_of(app));
        if let Some(v) = s.first() {
            return v.GetMasterVolume().ok();
        }
        let m = dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None).ok()?;
        m.GetMasterVolumeLevelScalar().ok()
    }
}

/// Set the volume of the music app (0..1); without a session of its own, the main volume.
pub fn set(app: &str, v: f32) {
    let v = v.clamp(0.0, 1.0);
    let Some(dev) = device() else { return };
    unsafe {
        let s = sessions(&dev, &stem_of(app));
        if !s.is_empty() {
            for x in s {
                let _ = x.SetMasterVolume(v, std::ptr::null());
            }
            return;
        }
        if let Ok(m) = dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) {
            let _ = m.SetMasterVolumeLevelScalar(v, std::ptr::null());
        }
    }
}
