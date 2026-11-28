//! Two small watchers: which program is using the microphone or the camera right now, and which file is being downloaded into the
//! Downloads folder. Everything is read-only; nothing is sent anywhere.
//!
//! * Microphone / camera: Windows keeps, for every program, when it last started and stopped using them (the "privacy" settings page
//!   reads the same list): `HKCU\...\CapabilityAccessManager\ConsentStore\microphone` and `...\webcam`. A program is using the device
//!   while its start time is later than its stop time.
//! * Downloads: browsers write a half-finished file next to the finished ones (`film.mp4.crdownload`, `.part` ...). While one of those
//!   grows, a download is running; when it is gone and the real file is there, it is done.
//!
//! Both only run while their switch in the settings is on (the loops just sleep otherwise).

use crate::media::Shared;
use crate::sys::{SysMsg, WM_SYS};
use crate::watchlogic::{consent_app_name, partial_final_name};
use std::collections::HashMap;
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{ERROR_NO_MORE_ITEMS, ERROR_SUCCESS, HWND, LPARAM, WPARAM};
use windows::Win32::System::Registry::*;
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

/// Switched from the settings.
pub static PRIVACY_ON: AtomicBool = AtomicBool::new(true);
pub static DOWNLOAD_ON: AtomicBool = AtomicBool::new(true);

pub fn start(hwnd: isize, shared: Arc<Shared>) {
    let s2 = shared.clone();
    let _ = std::thread::Builder::new().name("privacy".into()).spawn(move || privacy_loop(hwnd, shared));
    let _ = std::thread::Builder::new().name("downloads".into()).spawn(move || downloads_loop(hwnd, s2));
}

fn post(hwnd: isize, shared: &Shared, m: SysMsg) {
    if let Ok(mut g) = shared.sys.lock() {
        g.push(m);
    }
    unsafe {
        let _ = PostMessageW(HWND(hwnd as *mut _), WM_SYS, WPARAM(0), LPARAM(0));
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ---------------------------------------------------------------- microphone and camera

const CONSENT: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\CapabilityAccessManager\\ConsentStore\\";

unsafe fn subkeys(h: HKEY) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0u32;
    loop {
        let mut name = [0u16; 512];
        let mut n = name.len() as u32;
        let r = RegEnumKeyExW(h, i, windows::core::PWSTR(name.as_mut_ptr()), &mut n, None, windows::core::PWSTR::null(), None, None);
        if r == ERROR_NO_MORE_ITEMS || r != ERROR_SUCCESS {
            break;
        }
        out.push(String::from_utf16_lossy(&name[..n as usize]));
        i += 1;
    }
    out
}

unsafe fn qword(h: HKEY, name: PCWSTR) -> u64 {
    let mut v = 0u64;
    let mut n = 8u32;
    if RegGetValueW(h, PCWSTR::null(), name, RRF_RT_REG_QWORD, None, Some(&mut v as *mut u64 as *mut c_void), Some(&mut n)) == ERROR_SUCCESS {
        v
    } else {
        0
    }
}

/// Look at one key whose sub-keys are programs: the one that started latest and has not stopped.
unsafe fn busiest(parent: HKEY, best: &mut Option<(u64, String)>, skip_nonpackaged: bool) {
    for name in subkeys(parent) {
        let w = wide(&name);
        let mut h = HKEY::default();
        if RegOpenKeyExW(parent, PCWSTR(w.as_ptr()), 0, KEY_READ, &mut h) != ERROR_SUCCESS {
            continue;
        }
        if name == "NonPackaged" {
            if !skip_nonpackaged {
                busiest(h, best, true);
            }
        } else {
            let start = qword(h, windows::core::w!("LastUsedTimeStart"));
            let stop = qword(h, windows::core::w!("LastUsedTimeStop"));
            if start > 0 && start > stop && best.as_ref().map(|(t, _)| start > *t).unwrap_or(true) {
                *best = Some((start, name.clone()));
            }
        }
        let _ = RegCloseKey(h);
    }
}

/// The program using `kind` ("microphone" / "webcam") right now, if any.
fn in_use(kind: &str) -> Option<String> {
    unsafe {
        let path = wide(&format!("{CONSENT}{kind}"));
        let mut h = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(path.as_ptr()), 0, KEY_READ, &mut h) != ERROR_SUCCESS {
            return None;
        }
        let mut best = None;
        busiest(h, &mut best, false);
        let _ = RegCloseKey(h);
        best.map(|(_, name)| consent_app_name(&name))
    }
}

fn privacy_loop(hwnd: isize, shared: Arc<Shared>) {
    let mut last: Option<(Option<String>, Option<String>)> = None;
    loop {
        if !PRIVACY_ON.load(Ordering::Relaxed) {
            last = None;
            std::thread::sleep(Duration::from_secs(4));
            continue;
        }
        let cur = (in_use("microphone"), in_use("webcam"));
        if last.as_ref() != Some(&cur) {
            post(hwnd, &shared, SysMsg::Privacy { mic: cur.0.clone(), cam: cur.1.clone(), first: last.is_none() });
            last = Some(cur);
        }
        std::thread::sleep(Duration::from_millis(1500));
    }
}

// ---------------------------------------------------------------- downloads

fn downloads_dir() -> Option<PathBuf> {
    unsafe {
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::System::Com::CoTaskMemFree;
        use windows::Win32::UI::Shell::{FOLDERID_Downloads, SHGetKnownFolderPath, KF_FLAG_DEFAULT};
        if let Ok(p) = SHGetKnownFolderPath(&FOLDERID_Downloads, KF_FLAG_DEFAULT, HANDLE::default()) {
            let s = p.to_string().ok();
            CoTaskMemFree(Some(p.0 as *const c_void));
            if let Some(s) = s {
                return Some(PathBuf::from(s));
            }
        }
    }
    std::env::var_os("USERPROFILE").map(|p| PathBuf::from(p).join("Downloads"))
}

struct Seen {
    final_name: String,
    size: u64,
}

fn downloads_loop(hwnd: isize, shared: Arc<Shared>) {
    let Some(dir) = downloads_dir() else { return };
    // the half-finished files that are growing right now: file name -> what it will be called, its size
    let mut seen: HashMap<String, Seen> = HashMap::new();
    let mut quiet_since = Instant::now();
    loop {
        if !DOWNLOAD_ON.load(Ordering::Relaxed) {
            seen.clear();
            std::thread::sleep(Duration::from_secs(4));
            continue;
        }
        let mut now_active: Vec<(String, String, u64)> = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                let Some(final_name) = partial_final_name(&name) else { continue };
                let Ok(meta) = e.metadata() else { continue };
                // only one that is being written to now (an old, abandoned half-file is not a download)
                let fresh = meta.modified().ok().and_then(|m| SystemTime::now().duration_since(m).ok()).map(|d| d.as_secs() < 8).unwrap_or(false);
                if fresh || seen.contains_key(&name) {
                    now_active.push((name, final_name, meta.len()));
                }
            }
        }
        // finished (or cancelled): a half-file that is gone
        let gone: Vec<String> = seen.keys().filter(|k| !now_active.iter().any(|(n, _, _)| n == *k)).cloned().collect();
        for k in gone {
            if let Some(s) = seen.remove(&k) {
                if let Ok(m) = std::fs::metadata(dir.join(&s.final_name)) {
                    post(hwnd, &shared, SysMsg::Download { name: s.final_name, bytes: m.len(), done: true });
                }
            }
        }
        // the one that grew most recently is the one shown
        let mut growing: Option<(String, u64)> = None;
        for (name, final_name, size) in &now_active {
            let grew = seen.get(name).map(|s| *size > s.size).unwrap_or(true);
            seen.insert(name.clone(), Seen { final_name: final_name.clone(), size: *size });
            if grew && growing.as_ref().map(|(_, b)| *size >= *b).unwrap_or(true) {
                growing = Some((final_name.clone(), *size));
            }
        }
        if let Some((name, bytes)) = growing {
            post(hwnd, &shared, SysMsg::Download { name, bytes, done: false });
            quiet_since = Instant::now();
        }
        // poll quickly while something is downloading, slowly otherwise
        let busy = !seen.is_empty() || quiet_since.elapsed() < Duration::from_secs(10);
        std::thread::sleep(Duration::from_millis(if busy { 1000 } else { 2500 }));
    }
}
