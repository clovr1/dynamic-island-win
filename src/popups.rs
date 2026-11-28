//! "Notifications go to the island, not out of Windows": while Bloop runs, Windows' own pop-up banners are switched off for the apps
//! (their notifications still land in the notification centre, which is where Bloop reads them from). Everything is put back when
//! Bloop quits - and, if it was closed in a way that gave it no chance to, the next time it starts.
//!
//! What is changed: for each app under HKCU\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings, the value `ShowBanner`
//! (the "Show notification banner" switch of Settings > System > Notifications). The old values are kept in a small backup file.

use std::ffi::c_void;
use std::path::PathBuf;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{ERROR_NO_MORE_ITEMS, ERROR_SUCCESS};
use windows::Win32::System::Registry::*;

const BASE: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Notifications\\Settings";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn backup_path() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("APPDATA")?).join("DynamicIsland").join("banner_backup.txt"))
}

/// The names of the apps that have notification settings.
fn app_keys() -> Vec<String> {
    let mut out = Vec::new();
    unsafe {
        let base = wide(BASE);
        let mut h = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(base.as_ptr()), 0, KEY_READ, &mut h) != ERROR_SUCCESS {
            return out;
        }
        let mut i = 0u32;
        loop {
            let mut name = [0u16; 512];
            let mut n = name.len() as u32;
            let r = RegEnumKeyExW(h, i, windows::core::PWSTR(name.as_mut_ptr()), &mut n, None, windows::core::PWSTR::null(), None, None);
            if r == ERROR_NO_MORE_ITEMS {
                break;
            }
            if r == ERROR_SUCCESS {
                out.push(String::from_utf16_lossy(&name[..n as usize]));
            }
            i += 1;
            if i > 5000 {
                break;
            }
        }
        let _ = RegCloseKey(h);
    }
    out
}

fn key_path(app: &str) -> Vec<u16> {
    wide(&format!("{BASE}\\{app}"))
}

fn get_banner(app: &str) -> Option<u32> {
    unsafe {
        let p = key_path(app);
        let mut v = 0u32;
        let mut n = 4u32;
        let r = RegGetValueW(HKEY_CURRENT_USER, PCWSTR(p.as_ptr()), windows::core::w!("ShowBanner"), RRF_RT_REG_DWORD, None, Some(&mut v as *mut u32 as *mut c_void), Some(&mut n));
        (r == ERROR_SUCCESS).then_some(v)
    }
}

fn set_banner(app: &str, v: u32) {
    unsafe {
        let p = key_path(app);
        let _ = RegSetKeyValueW(HKEY_CURRENT_USER, PCWSTR(p.as_ptr()), windows::core::w!("ShowBanner"), REG_DWORD.0, Some(&v as *const u32 as *const c_void), 4);
    }
}

fn del_banner(app: &str) {
    unsafe {
        let p = key_path(app);
        let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, PCWSTR(p.as_ptr()), windows::core::w!("ShowBanner"));
    }
}

/// The backup: one line per app: `name<TAB>old value` (or `name<TAB>-` when there was none).
fn read_backup() -> Vec<(String, Option<u32>)> {
    let Some(p) = backup_path() else { return Vec::new() };
    std::fs::read_to_string(p)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let (name, v) = l.rsplit_once('\t')?;
            Some((name.to_string(), v.trim().parse().ok()))
        })
        .collect()
}

fn write_backup(list: &[(String, Option<u32>)]) {
    let Some(p) = backup_path() else { return };
    if list.is_empty() {
        let _ = std::fs::remove_file(p);
        return;
    }
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let text: String = list.iter().map(|(n, v)| format!("{n}\t{}\n", v.map(|v| v.to_string()).unwrap_or_else(|| "-".into()))).collect();
    let _ = std::fs::write(p, text);
}

/// Switch the banners off for every app that has settings (and remember how each one was). Safe to call again and again: it also catches
/// apps that appeared since the last time.
pub fn hide() {
    let mut backup = read_backup();
    let mut changed = 0;
    for app in app_keys() {
        // (the notification centre's own entries stay as they are)
        if backup.iter().any(|(n, _)| *n == app) {
            if get_banner(&app) != Some(0) {
                set_banner(&app, 0);
            }
            continue;
        }
        let old = get_banner(&app);
        if old == Some(0) {
            // already off before us: leave it off, and do not "restore" it to on later
            continue;
        }
        backup.push((app.clone(), old));
        set_banner(&app, 0);
        changed += 1;
    }
    write_backup(&backup);
    if changed > 0 {
        crate::win::log(&format!("popups: Windows' own banners are off for {changed} more app(s) while Bloop runs"));
    }
}

/// One app that has just sent a notification (it may not have had settings yet): its banner is switched off too.
pub fn hide_app(app: &str) {
    if app.is_empty() || app.contains(['\\', '/']) {
        return;
    }
    let mut backup = read_backup();
    if backup.iter().any(|(n, _)| n == app) {
        return;
    }
    let old = get_banner(app);
    if old == Some(0) {
        return;
    }
    backup.push((app.to_string(), old));
    set_banner(app, 0);
    write_backup(&backup);
}

/// Put every banner back the way it was.
pub fn restore() {
    let backup = read_backup();
    for (app, old) in &backup {
        match old {
            Some(v) => set_banner(app, *v),
            None => del_banner(app),
        }
    }
    if !backup.is_empty() {
        crate::win::log(&format!("popups: Windows' own banners are back for {} app(s)", backup.len()));
    }
    write_backup(&[]);
}
