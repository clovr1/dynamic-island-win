//! Music, straight from Windows (System Media Transport Controls): Spotify, Apple Music, a browser tab ... whatever Windows shows
//! in its own media overlay. Windows tells us when something changes, so nothing is polled and nothing runs while music is steady.

use crate::ui::{ArtData, MediaInfo};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
use windows::core::Result;
use windows::Foundation::{EventRegistrationToken, TypedEventHandler};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session, GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionMediaProperties as Props, GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
    MediaPropertiesChangedEventArgs, PlaybackInfoChangedEventArgs, TimelinePropertiesChangedEventArgs,
};
use windows::Storage::Streams::DataReader;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

/// "something new from the music thread" (WM_APP + 1)
pub const WM_MEDIA: u32 = 0x8001;

#[derive(Default)]
pub struct Shared {
    pub info: Mutex<Option<MediaInfo>>,
    /// Some(None) = "no art", Some(Some(a)) = new art, None = nothing new
    pub art: Mutex<Option<Option<ArtData>>>,
    pub session: Mutex<Option<Session>>,
    /// the app id of the playing session ("Spotify.exe" ...), lowercase
    pub app: Mutex<String>,
    /// the same, as Windows wrote it (used to bring the app forward)
    pub app_raw: Mutex<String>,
    /// lyrics lookups that finished
    pub lyrics: Mutex<Vec<crate::net::LyricsMsg>>,
    /// notifications waiting for the window
    pub notifs: Mutex<Vec<crate::ui::NotifData>>,
}

#[derive(Clone, Copy)]
pub enum Cmd {
    Toggle,
    Next,
    Prev,
    Seek(f64),
}

/// Called from the window thread.
pub fn command(shared: &Shared, cmd: Cmd) {
    let session = shared.session.lock().ok().and_then(|g| g.clone());
    if let Some(s) = session {
        match cmd {
            Cmd::Toggle => { let _ = s.TryTogglePlayPauseAsync(); }
            Cmd::Next => { let _ = s.TrySkipNextAsync(); }
            Cmd::Prev => { let _ = s.TrySkipPreviousAsync(); }
            Cmd::Seek(secs) => { let _ = s.TryChangePlaybackPositionAsync((secs * 1e7) as i64); }
        }
    }
}

pub fn start(hwnd: isize, shared: Arc<Shared>) {
    let _ = std::thread::Builder::new().name("media".into()).spawn(move || {
        if let Err(e) = run(hwnd, shared) {
            crate::win::log(&format!("media thread stopped: {e}"));
        }
    });
}

struct Attached {
    session: Session,
    app: String,
    tokens: (EventRegistrationToken, EventRegistrationToken, EventRegistrationToken),
}

fn detach(a: Attached) {
    let _ = a.session.RemoveMediaPropertiesChanged(a.tokens.0);
    let _ = a.session.RemovePlaybackInfoChanged(a.tokens.1);
    let _ = a.session.RemoveTimelinePropertiesChanged(a.tokens.2);
}

fn run(hwnd: isize, shared: Arc<Shared>) -> Result<()> {
    unsafe { RoInitialize(RO_INIT_MULTITHREADED)? };
    let mgr = Manager::RequestAsync()?.get()?;
    let (tx, rx) = mpsc::channel::<()>();
    {
        let tx = tx.clone();
        mgr.CurrentSessionChanged(&TypedEventHandler::new(move |_, _| {
            let _ = tx.send(());
            Ok(())
        }))?;
    }
    {
        let tx = tx.clone();
        mgr.SessionsChanged(&TypedEventHandler::new(move |_, _| {
            let _ = tx.send(());
            Ok(())
        }))?;
    }

    let mut attached: Option<Attached> = None;
    let mut last_key = String::new();
    let mut have_art = false;
    let mut art_tries = 0u32;
    let mut last_pub: Option<(MediaInfo, Instant)> = None;

    loop {
        match mgr.GetCurrentSession() {
            Err(_) => publish_inactive(hwnd, &shared, &mut last_pub, &mut last_key, &mut attached),
            Ok(session) => {
                let app = session.SourceAppUserModelId().map(|s| s.to_string()).unwrap_or_default();
                if attached.as_ref().map(|a| a.app != app).unwrap_or(true) {
                    if let Some(old) = attached.take() {
                        detach(old);
                    }
                    let t1 = tx.clone();
                    let t2 = tx.clone();
                    let t3 = tx.clone();
                    let k1 = session.MediaPropertiesChanged(&TypedEventHandler::<Session, MediaPropertiesChangedEventArgs>::new(move |_, _| { let _ = t1.send(()); Ok(()) }))?;
                    let k2 = session.PlaybackInfoChanged(&TypedEventHandler::<Session, PlaybackInfoChangedEventArgs>::new(move |_, _| { let _ = t2.send(()); Ok(()) }))?;
                    let k3 = session.TimelinePropertiesChanged(&TypedEventHandler::<Session, TimelinePropertiesChangedEventArgs>::new(move |_, _| { let _ = t3.send(()); Ok(()) }))?;
                    attached = Some(Attached { session: session.clone(), app: app.clone(), tokens: (k1, k2, k3) });
                    if let Ok(mut g) = shared.session.lock() {
                        *g = Some(session.clone());
                    }
                    if let Ok(mut g) = shared.app.lock() {
                        *g = app.to_lowercase();
                    }
                    if let Ok(mut g) = shared.app_raw.lock() {
                        *g = app.clone();
                    }
                    unsafe {
                        let _ = PostMessageW(HWND(hwnd as *mut _), crate::win::WM_ENV, WPARAM(0), LPARAM(0));
                    }
                }
                if let Err(e) = refresh(hwnd, &shared, &session, &app, &mut last_pub, &mut last_key, &mut have_art, &mut art_tries) {
                    crate::win::log(&format!("refresh: {e}"));
                }
            }
        }
        match rx.recv_timeout(Duration::from_secs(4)) {
            Ok(()) => {
                std::thread::sleep(Duration::from_millis(80)); // a song change fires many events: handle them together
                while rx.try_recv().is_ok() {}
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    Ok(())
}

fn post(hwnd: isize) {
    unsafe {
        let _ = PostMessageW(HWND(hwnd as *mut _), WM_MEDIA, WPARAM(0), LPARAM(0));
    }
}

fn publish_inactive(hwnd: isize, shared: &Shared, last_pub: &mut Option<(MediaInfo, Instant)>, last_key: &mut String, attached: &mut Option<Attached>) {
    if let Some(old) = attached.take() {
        detach(old);
    }
    if let Ok(mut g) = shared.session.lock() {
        *g = None;
    }
    if matches!(last_pub, Some((i, _)) if !i.active) {
        return;
    }
    last_key.clear();
    let info = MediaInfo::default();
    *last_pub = Some((info.clone(), Instant::now()));
    if let Ok(mut g) = shared.info.lock() {
        *g = Some(info);
    }
    if let Ok(mut g) = shared.art.lock() {
        *g = Some(None);
    }
    post(hwnd);
}

fn filetime_now() -> i64 {
    // 100-ns ticks since 1601, like Windows' own timestamps
    let unix = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() / 100).unwrap_or(0) as i64;
    unix + 116_444_736_000_000_000
}

#[allow(clippy::too_many_arguments)]
fn refresh(
    hwnd: isize,
    shared: &Shared,
    session: &Session,
    app: &str,
    last_pub: &mut Option<(MediaInfo, Instant)>,
    last_key: &mut String,
    have_art: &mut bool,
    art_tries: &mut u32,
) -> Result<()> {
    let props = session.TryGetMediaPropertiesAsync()?.get()?;
    let title = props.Title()?.to_string();
    let artist = props.Artist()?.to_string();
    let playing = session.GetPlaybackInfo()?.PlaybackStatus()? == Status::Playing;
    let tl = session.GetTimelineProperties()?;
    let start = tl.StartTime()?.Duration;
    let end = tl.EndTime()?.Duration;
    let mut pos = (tl.Position()?.Duration - start) as f64 / 1e7;
    let dur = ((end - start) as f64 / 1e7).max(0.0);
    if playing {
        let age = (filetime_now() - tl.LastUpdatedTime()?.UniversalTime) as f64 / 1e7;
        if (0.0..36_000.0).contains(&age) {
            pos += age;
        }
    }
    if dur > 0.0 {
        pos = pos.min(dur);
    }
    let info = MediaInfo { active: true, title: title.clone(), artist, playing, pos: pos.max(0.0), dur };

    // only tell the window when something really changed (a steady song costs nothing)
    let changed = match last_pub {
        Some((old, at)) => {
            let expect = if old.playing { old.pos + at.elapsed().as_secs_f64() } else { old.pos };
            !old.active || old.title != info.title || old.artist != info.artist || old.playing != info.playing || (old.dur - info.dur).abs() > 0.5 || (expect - info.pos).abs() > 1.5
        }
        None => true,
    };
    if changed {
        *last_pub = Some((info.clone(), Instant::now()));
        if let Ok(mut g) = shared.info.lock() {
            *g = Some(info.clone());
        }
        post(hwnd);
    }

    // album art: fetched when the song changes (Spotify sometimes needs a moment, so it is retried a few times)
    let key = format!("{}|{}|{}", info.title, info.artist, app);
    if key != *last_key {
        *last_key = key;
        *have_art = false;
        *art_tries = 0;
        if let Ok(mut g) = shared.art.lock() {
            *g = Some(None);
        }
        post(hwnd);
    }
    if !*have_art && *art_tries < 8 {
        *art_tries += 1;
        if let Some(a) = fetch_art(&props) {
            *have_art = true;
            if let Ok(mut g) = shared.art.lock() {
                *g = Some(Some(a));
            }
            post(hwnd);
        }
    }
    Ok(())
}

fn fetch_art(props: &Props) -> Option<ArtData> {
    let thumb = props.Thumbnail().ok()?;
    let stream = thumb.OpenReadAsync().ok()?.get().ok()?;
    let size = stream.Size().ok()? as u32;
    if size == 0 || size > 12_000_000 {
        return None;
    }
    let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0).ok()?).ok()?;
    reader.LoadAsync(size).ok()?.get().ok()?;
    let mut buf = vec![0u8; size as usize];
    reader.ReadBytes(&mut buf).ok()?;
    let img = image::load_from_memory(&buf).ok()?;
    let (w, h) = (img.width(), img.height());
    let side = w.min(h);
    let sq = img.crop_imm((w - side) / 2, (h - side) / 2, side, side);
    let small = image::imageops::resize(&sq.to_rgba8(), 128, 128, image::imageops::FilterType::Triangle);
    Some(ArtData { w: 128, h: 128, rgba: small.into_raw() })
}
