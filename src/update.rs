//! Self-update from GitHub Releases: look at the newest release of the configured repository, download its .exe when it is newer,
//! check it, and swap it in on request. Everything is HTTPS to github.com; nothing else is contacted.

use crate::net;
use crate::version::{VERSION, hex, is_newer, parse_version, pick_asset};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;


/// "update check finished" (WM_APP + 5)
pub const WM_UPDATE: u32 = 0x8005;

pub enum Outcome {
    UpToDate,
    /// a newer version is downloaded and checked, ready to be swapped in
    Ready(String),
    Failed(String),
}

static RESULT: Mutex<Option<(bool, Outcome)>> = Mutex::new(None);
static BUSY: AtomicBool = AtomicBool::new(false);

fn exe_path() -> Option<PathBuf> {
    std::env::current_exe().ok()
}

fn with_suffix(p: &std::path::Path, suffix: &str) -> PathBuf {
    let mut s = p.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

/// Remove what an earlier update left behind.
pub fn cleanup() {
    if let Some(exe) = exe_path() {
        let _ = std::fs::remove_file(with_suffix(&exe, ".old"));
    }
}

/// Ask GitHub, and if a newer version exists download it next to the running program.
fn check_and_download(repo: &str) -> Outcome {
    if repo.is_empty() {
        return Outcome::Failed("Fill in the GitHub repository first (owner/name).".into());
    }
    let Some(body) = net::get("api.github.com", &format!("/repos/{repo}/releases/latest")) else {
        return Outcome::Failed("Could not reach GitHub (or no release exists in that repository yet).".into());
    };
    let Ok(json) = serde_json::from_slice::<serde_json::Value>(&body) else {
        return Outcome::Failed("GitHub's answer could not be read.".into());
    };
    let tag = json.get("tag_name").and_then(|t| t.as_str()).unwrap_or("").to_string();
    if parse_version(&tag).is_none() {
        return Outcome::Failed(format!("The latest release has no usable version number ({tag:?})."));
    }
    if !is_newer(&tag, VERSION) {
        return Outcome::UpToDate;
    }
    let Some((url, digest)) = pick_asset(&json) else {
        return Outcome::Failed(format!("Release {tag} has no .exe file attached."));
    };
    let Some((host, path)) = net::split_url(&url) else {
        return Outcome::Failed("The download address is not a normal https address.".into());
    };
    // only GitHub's own hosts
    if !(host == "github.com" || host.ends_with(".github.com") || host.ends_with(".githubusercontent.com")) {
        return Outcome::Failed(format!("Refusing to download from {host}."));
    }
    let Some(exe) = exe_path() else {
        return Outcome::Failed("Cannot find where the program lives.".into());
    };
    let Some(data) = net::get_limit(&host, &path, 60_000_000) else {
        return Outcome::Failed("The download failed.".into());
    };
    if data.len() < 300_000 || &data[..2] != b"MZ" {
        return Outcome::Failed("The downloaded file does not look like a program.".into());
    }
    if let Some(want) = digest {
        let got = hex(&Sha256::digest(&data));
        if got != want {
            return Outcome::Failed("The download is damaged (checksum differs). Nothing was installed.".into());
        }
    }
    let new = with_suffix(&exe, ".new");
    if std::fs::write(&new, &data).is_err() {
        return Outcome::Failed("Could not save the update next to the program (is the folder write-protected?).".into());
    }
    Outcome::Ready(tag.trim_start_matches(['v', 'V']).to_string())
}

/// Run a check in the background; the window hears about the result by WM_UPDATE.
/// `manual`: the user asked (so even "up to date" and errors are shown).
pub fn spawn_check(hwnd: isize, repo: String, manual: bool) {
    if BUSY.swap(true, Ordering::SeqCst) {
        return;
    }
    let _ = std::thread::Builder::new().name("update".into()).spawn(move || {
        cleanup();
        let out = check_and_download(&repo);
        if let Ok(mut g) = RESULT.lock() {
            *g = Some((manual, out));
        }
        BUSY.store(false, Ordering::SeqCst);
        unsafe {
            let _ = PostMessageW(HWND(hwnd as *mut _), WM_UPDATE, WPARAM(0), LPARAM(0));
        }
    });
}

pub fn take_result() -> Option<(bool, Outcome)> {
    RESULT.lock().ok().and_then(|mut g| g.take())
}

pub fn busy() -> bool {
    BUSY.load(Ordering::SeqCst)
}

/// Take a program file the user picked (a newer build made on this PC) and make it the pending update.
pub fn stage_file(path: &std::path::Path) -> Result<(), String> {
    let data = std::fs::read(path).map_err(|e| format!("could not read the file: {e}"))?;
    if data.len() < 300_000 || &data[..2] != b"MZ" {
        return Err("that file does not look like a program".into());
    }
    // it must be this app: its single-instance name is inside (as UTF-16 text)
    let needle: Vec<u8> = "DynamicIslandNative".encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
    if !data.windows(needle.len()).any(|w| w == needle.as_slice()) {
        return Err("that file is not a Dynamic Island program".into());
    }
    let exe = exe_path().ok_or("cannot find the program")?;
    std::fs::write(with_suffix(&exe, ".new"), &data).map_err(|e| format!("could not save the update next to the program: {e}"))
}

/// Swap the downloaded file in and start it. The caller then closes this instance.
pub fn apply() -> Result<(), String> {
    let exe = exe_path().ok_or("cannot find the program")?;
    let new = with_suffix(&exe, ".new");
    let old = with_suffix(&exe, ".old");
    if !new.exists() {
        return Err("no downloaded update".into());
    }
    let _ = std::fs::remove_file(&old);
    // a running program cannot be overwritten, but it can be renamed
    std::fs::rename(&exe, &old).map_err(|e| format!("could not move the old program away: {e}"))?;
    if let Err(e) = std::fs::rename(&new, &exe) {
        let _ = std::fs::rename(&old, &exe);
        return Err(format!("could not put the new program in place: {e}"));
    }
    if let Err(e) = std::process::Command::new(&exe).arg("--after-update").spawn() {
        // put things back so the old one keeps working
        let _ = std::fs::remove_file(&exe);
        let _ = std::fs::rename(&old, &exe);
        return Err(format!("could not start the new program: {e}"));
    }
    Ok(())
}

