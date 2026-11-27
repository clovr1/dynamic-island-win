//! A tiny HTTPS GET through Windows' own WinHTTP (no extra libraries), and the lyrics lookup built on it.
//! Only the song title, artist and length are sent to lrclib.net, and only when you switch lyrics on.

use crate::lyrics::{self, Found};
use crate::media::Shared;
use std::sync::Arc;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Networking::WinHttp::*;
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

/// "lyrics arrived" (WM_APP + 4)
pub const WM_LYRICS: u32 = 0x8004;

pub struct LyricsMsg {
    pub key: String,
    pub found: Option<Found>,
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// GET https://host/path  ->  the body (None when anything fails or takes too long).
pub fn get(host: &str, path: &str) -> Option<Vec<u8>> {
    get_limit(host, path, 4_000_000)
}

/// Split "https://host/path?query" into (host, "/path?query").
pub fn split_url(url: &str) -> Option<(String, String)> {
    let rest = url.strip_prefix("https://")?;
    let (host, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    if host.is_empty() || host.contains(':') || host.contains('@') {
        return None;
    }
    Some((host.to_string(), path.to_string()))
}

/// Like `get`, with a larger body limit (a download). Only an HTTP 200 counts as an answer.
pub fn get_limit(host: &str, path: &str, limit: usize) -> Option<Vec<u8>> {
    unsafe {
        let session = WinHttpOpen(w!("DynamicIslandNative/1.0"), WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, PCWSTR::null(), PCWSTR::null(), 0);
        if session.is_null() {
            return None;
        }
        let out = (|| {
            let _ = WinHttpSetTimeouts(session, 4000, 4000, 6000, if limit > 4_000_000 { 60_000 } else { 8000 });
            let h = wide(host);
            let conn = WinHttpConnect(session, PCWSTR(h.as_ptr()), INTERNET_DEFAULT_HTTPS_PORT as u16, 0);
            if conn.is_null() {
                return None;
            }
            let p = wide(path);
            let req = WinHttpOpenRequest(conn, w!("GET"), PCWSTR(p.as_ptr()), PCWSTR::null(), PCWSTR::null(), std::ptr::null(), WINHTTP_FLAG_SECURE);
            if req.is_null() {
                let _ = WinHttpCloseHandle(conn);
                return None;
            }
            let body = (|| {
                WinHttpSendRequest(req, None, None, 0, 0, 0).ok()?;
                WinHttpReceiveResponse(req, std::ptr::null_mut()).ok()?;
                let mut status = 0u32;
                let mut len = 4u32;
                WinHttpQueryHeaders(req, WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER, PCWSTR::null(), Some(&mut status as *mut u32 as *mut _), &mut len, std::ptr::null_mut()).ok()?;
                if status != 200 {
                    return None;
                }
                let mut data = Vec::new();
                loop {
                    let mut avail = 0u32;
                    WinHttpQueryDataAvailable(req, &mut avail).ok()?;
                    if avail == 0 {
                        break;
                    }
                    let mut buf = vec![0u8; avail as usize];
                    let mut got = 0u32;
                    WinHttpReadData(req, buf.as_mut_ptr() as *mut _, avail, &mut got).ok()?;
                    data.extend_from_slice(&buf[..got as usize]);
                    if data.len() > limit {
                        return None;
                    }
                }
                Some(data)
            })();
            let _ = WinHttpCloseHandle(req);
            let _ = WinHttpCloseHandle(conn);
            body
        })();
        let _ = WinHttpCloseHandle(session);
        out
    }
}

/// Look the lyrics up in the background and tell the window when done.
pub fn fetch_lyrics(hwnd: isize, shared: Arc<Shared>, key: String, title: String, artist: String, duration: f64) {
    let _ = std::thread::Builder::new().name("lyrics".into()).spawn(move || {
        let mut found = None;
        for path in lyrics::search_paths(&title, &artist) {
            let Some(body) = get("lrclib.net", &path) else { continue };
            let Ok(json) = serde_json::from_slice::<serde_json::Value>(&body) else { continue };
            if let Some(f) = lyrics::pick_best(&json, duration) {
                found = Some(f);
                break;
            }
        }
        if let Ok(mut g) = shared.lyrics.lock() {
            g.push(LyricsMsg { key, found });
        }
        unsafe {
            let _ = PostMessageW(HWND(hwnd as *mut _), WM_LYRICS, WPARAM(0), LPARAM(0));
        }
    });
}
