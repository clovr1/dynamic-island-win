//! The Windows shell around the island: one transparent window at the top edge of the screen, drawn with per-pixel alpha.
//! Pixels that are fully transparent let the mouse through, so only the island itself ever catches the mouse.

use crate::media::{self, Cmd, Shared, WM_MEDIA};
use crate::net::{self, WM_LYRICS};
use crate::notify::{self, WM_NOTIF};
use crate::settings::Settings;
use crate::settings_win::{self, Action};
use crate::update::{self, Outcome, WM_UPDATE};
use crate::sys::{self, SysMsg, WM_SYS};
use crate::ui::{Hit, Need, StatusData, Ui};
use windows::Win32::UI::Shell::{Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW};
use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Graphics::Dwm::DwmFlush;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Registry::*;
use windows::Win32::System::Threading::{CreateMutexW, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_SINGLETHREADED};
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{keybd_event, ReleaseCapture, SetCapture, TrackMouseEvent, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, TME_LEAVE, TRACKMOUSEEVENT, VK_MENU};
use windows::Win32::UI::Shell::{SHQueryUserNotificationState, ShellExecuteW, QUNS_RUNNING_D3D_FULL_SCREEN};
use windows::Win32::UI::WindowsAndMessaging::*;

/// "the window in front changed" / "the playing app changed" (WM_APP + 3)
pub const WM_ENV: u32 = 0x8003;
const EVENT_SYSTEM_FOREGROUND: u32 = 0x0003;
const WINEVENT_OUTOFCONTEXT: u32 = 0;
static MAIN_HWND: AtomicIsize = AtomicIsize::new(0);

/// Dedicated music apps. A browser is not one: you are always "in" it, so its pill stays.
const PLAYERS: [&str; 11] = ["spotify", "applemusic", "itunes", "tidal", "deezer", "amazonmusic", "musicbee", "foobar2000", "aimp", "winamp", "vlc"];

/// Windows tells us the instant another window comes to the front.
unsafe extern "system" fn on_foreground(_h: HWINEVENTHOOK, _event: u32, _hwnd: HWND, _o: i32, _c: i32, _t: u32, _ms: u32) {
    let h = MAIN_HWND.load(Ordering::Relaxed);
    if h != 0 {
        let _ = PostMessageW(HWND(h as *mut _), WM_ENV, WPARAM(0), LPARAM(0));
    }
}

/// File name (without .exe, lowercase) of the program that owns the window in front.
fn foreground_program() -> String {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() {
            return String::new();
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(fg, Some(&mut pid));
        let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return String::new() };
        let mut buf = [0u16; 520];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, windows::core::PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
        let _ = CloseHandle(h);
        if !ok {
            return String::new();
        }
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        std::path::Path::new(&path).file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default()
    }
}

const TIMER_ID: usize = 1;
/// what the rest of the screen is doing (full screen / maximized window): checked about once a second
const TIMER_ENV: usize = 2;
/// while the island is tucked away: is the cursor at the top edge?
const TIMER_PEEK: usize = 3;
/// the next look for a new version
const TIMER_UPD: usize = 4;
/// a mouse event on the tray icon (WM_APP + 6)
const WM_TRAY: u32 = 0x8006;
/// Explorer was restarted: the tray icon must be added again
static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);
const WM_MOUSELEAVE: u32 = 0x02A3;
const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
/// the "app" of the update card: a click on it installs instead of opening an app
const UPDATE_AUMID: &str = "island:update";
const RUN_NAME: PCWSTR = w!("DynamicIsland");

pub fn log(msg: &str) {
    use std::io::Write;
    let path = std::env::temp_dir().join("bloop.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{msg}");
    }
}

struct App {
    hwnd: HWND,
    ui: Ui,
    mem_dc: HDC,
    bits: *mut u8,
    pw: i32,
    ph: i32,
    shared: Arc<Shared>,
    tracking: bool,
    timer: Need,
    last: Instant,
    bmp: HBITMAP,
    /// the screen's own scale (DPI); the user's size setting multiplies it
    dpi_scale: f32,
    settings: Settings,
    /// a downloaded, checked update waits to be swapped in (its version)
    update_ready: Option<String>,
    clock: String,
    ticks: u32,
}

/// `%APPDATA%\DynamicIsland\settings.ini`
fn settings_path() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("APPDATA")?;
    Some(std::path::PathBuf::from(base).join("DynamicIsland").join("settings.ini"))
}

fn load_settings() -> Settings {
    settings_path().and_then(|p| std::fs::read_to_string(p).ok()).map(|t| Settings::parse(&t)).unwrap_or_default()
}

fn save_settings(s: &Settings) {
    if let Some(p) = settings_path() {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, s.to_text());
    }
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|a| a.try_borrow_mut().ok().and_then(|mut g| g.as_mut().map(f)))
}

impl App {
    fn present(&mut self) {
        // sound effects the island wants to make
        let snd = self.ui.take_sounds();
        if self.settings.sounds {
            for s in snd {
                crate::sound::play(s, self.settings.volume);
            }
        }
        let pm = self.ui.render();
        unsafe {
            let dst = std::slice::from_raw_parts_mut(self.bits, (self.pw * self.ph * 4) as usize);
            for (d, s) in dst.chunks_exact_mut(4).zip(pm.data().chunks_exact(4)) {
                d[0] = s[2];
                d[1] = s[1];
                d[2] = s[0];
                d[3] = s[3];
            }
            let size = SIZE { cx: self.pw, cy: self.ph };
            let src = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8 };
            let _ = UpdateLayeredWindow(self.hwnd, None, None, Some(&size), self.mem_dc, Some(&src), COLORREF(0), Some(&blend), ULW_ALPHA);
        }
    }

    /// Fast = the message loop draws one frame per screen refresh (see `run`), Slow = a 30 fps timer, Rest = nothing runs at all.
    fn set_timer(&mut self, need: Need) {
        if need == self.timer {
            return;
        }
        unsafe {
            match need {
                Need::Slow => {
                    SetTimer(self.hwnd, TIMER_ID, 33, None);
                }
                Need::Idle => {
                    SetTimer(self.hwnd, TIMER_ID, 100, None);
                }
                Need::Doze => {
                    SetTimer(self.hwnd, TIMER_ID, 400, None);
                }
                Need::Rest | Need::Fast => {
                    let _ = KillTimer(self.hwnd, TIMER_ID);
                }
            }
        }
        if self.timer == Need::Rest {
            self.last = Instant::now();
        }
        self.timer = need;
    }

    fn is_animating(&self) -> bool {
        self.timer == Need::Fast
    }

    fn kick(&mut self) {
        let need = self.ui.need();
        self.set_timer(need);
    }

    /// One animation step (called by the 30 fps timer, and once per screen refresh while something is moving).
    fn on_timer(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f32().min(0.1);
        self.last = now;
        // safety net: if Windows never told us the mouse left, look where the cursor really is
        if self.ui.has_cursor() || self.ui.is_open() || self.tracking {
            let mut pt = POINT::default();
            unsafe {
                if GetCursorPos(&mut pt).is_ok() {
                    let mut r = RECT::default();
                    let _ = GetWindowRect(self.hwnd, &mut r);
                    let (ix, iy, iw, ih) = self.ui.rect_px();
                    let m = 6.0 * self.ui.scale();
                    let (cx, cy) = ((pt.x - r.left) as f32, (pt.y - r.top) as f32);
                    if cx < ix - m || cx > ix + iw + m || cy < iy - m || cy > iy + ih + m {
                        self.ui.set_cursor(None);
                        self.tracking = false;
                    }
                }
            }
        }
        // the face looks after the mouse, wherever it is on the screen
        if self.ui.face_follows() {
            let mut pt = POINT::default();
            unsafe {
                if GetCursorPos(&mut pt).is_ok() {
                    let mut r = RECT::default();
                    let _ = GetWindowRect(self.hwnd, &mut r);
                    let s = self.ui.scale();
                    let (cx, cy) = ((pt.x - r.left) as f32 / s, (pt.y - r.top) as f32 / s);
                    self.ui.set_look(Some((cx - crate::ui::W / 2.0, cy - (crate::ui::TOP + 18.0))));
                }
            }
        } else {
            self.ui.set_look(None);
        }
        let need = self.ui.tick(dt);
        self.present();
        self.set_timer(need);
    }

    /// Start the lyrics lookup the UI asked for (only when lyrics are switched on, once per song).
    fn start_lyrics_lookup(&mut self) {
        if let Some(r) = self.ui.take_lyrics_request() {
            net::fetch_lyrics(self.hwnd.0 as isize, self.shared.clone(), r.key, r.title, r.artist, r.dur);
        }
    }

    fn on_lyrics(&mut self) {
        let list: Vec<_> = self.shared.lyrics.lock().map(|mut g| std::mem::take(&mut *g)).unwrap_or_default();
        for m in list {
            self.ui.set_lyrics(&m.key, m.found);
        }
        self.present();
        self.kick();
    }

    fn on_media(&mut self) {
        let info = self.shared.info.lock().ok().and_then(|mut g| g.take());
        let art = self.shared.art.lock().ok().and_then(|mut g| g.take());
        if let Some(a) = art {
            self.ui.set_art(a);
        }
        if let Some(i) = info {
            self.ui.set_media(i);
            self.start_lyrics_lookup();
        }
        self.present();
        self.kick();
    }

    /// Full screen / maximized window: asked about once a second (Windows has no cheaper way to tell us).
    fn check_env(&mut self) {
        self.ticks += 1;
        if self.ticks % 60 == 0 && self.settings.hide_windows_popups {
            crate::popups::hide();
        }
        self.update_clock();
        self.refresh_volume();
        let mut env = self.ui.env();
        unsafe {
            // A full-screen app: Windows says "exclusive full screen" (a game), or the window in front covers the whole screen without a title bar
            // (a video, F11 in the browser, a borderless game). Windows' other "busy" flags are not enough on their own: a live wallpaper or a
            // presentation setting can set them while an ordinary desktop is showing, and then the island would vanish for no reason.
            let fg0 = GetForegroundWindow();
            let mut covers = false;
            if !fg0.0.is_null() && fg0 != self.hwnd && IsWindowVisible(fg0).as_bool() && !IsIconic(fg0).as_bool() {
                let mut buf = [0u16; 64];
                let n = GetClassNameW(fg0, &mut buf) as usize;
                let class = String::from_utf16_lossy(&buf[..n]);
                if !matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd" | "XamlExplorerHostIslandWindow") {
                    let mon = MonitorFromWindow(fg0, MONITOR_DEFAULTTONEAREST);
                    let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
                    let mut r = RECT::default();
                    if GetMonitorInfoW(mon, &mut mi).as_bool() && GetWindowRect(fg0, &mut r).is_ok() {
                        let m = mi.rcMonitor;
                        let style = GetWindowLongW(fg0, GWL_STYLE) as u32;
                        let has_title = style & WS_CAPTION.0 == WS_CAPTION.0;
                        covers = r.left <= m.left && r.top <= m.top && r.right >= m.right && r.bottom >= m.bottom && !has_title;
                    }
                }
            }
            let mut fs = covers;
            if let Ok(st) = SHQueryUserNotificationState() {
                fs = fs || st == QUNS_RUNNING_D3D_FULL_SCREEN;
            }
            let mut mx = false;
            let fg = GetForegroundWindow();
            if !fg.0.is_null() && fg != self.hwnd && IsWindowVisible(fg).as_bool() && !IsIconic(fg).as_bool() && IsZoomed(fg).as_bool() {
                let mut buf = [0u16; 64];
                let n = GetClassNameW(fg, &mut buf) as usize;
                let class = String::from_utf16_lossy(&buf[..n]);
                mx = !matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd");
            }
            env.fullscreen = fs;
            env.maximized = mx;
        }
        // is the window in front the music app that is playing? (then its own window shows the song: the island stays empty)
        let app = self.shared.app.lock().map(|g| g.clone()).unwrap_or_default();
        let prog = foreground_program();
        env.player_front = !prog.is_empty() && PLAYERS.iter().any(|p| app.contains(p) && prog.contains(p));
        let concealed = env.fullscreen || env.maximized;
        if !concealed {
            env.peek = false;
        }
        if env != self.ui.env() {
            self.ui.set_env(env);
            unsafe {
                if concealed {
                    SetTimer(self.hwnd, TIMER_PEEK, 100, None);
                } else {
                    let _ = KillTimer(self.hwnd, TIMER_PEEK);
                }
            }
            self.present();
            self.kick();
        }
    }

    /// Cursor at the very top edge (middle of the screen) shows the island again for a moment, like a hidden menu bar.
    fn check_peek(&mut self) {
        let mut pt = POINT::default();
        unsafe {
            if GetCursorPos(&mut pt).is_err() {
                return;
            }
            let mut r = RECT::default();
            let _ = GetWindowRect(self.hwnd, &mut r);
            let s = self.ui.scale();
            let cx = r.left as f32 + self.pw as f32 / 2.0;
            let (dx, y) = ((pt.x as f32 - cx).abs(), (pt.y - r.top) as f32);
            let mut env = self.ui.env();
            // a visible notch is hovered directly; the top-edge peek is only for an island that is hidden
            let want = if !self.ui.needs_peek() { false } else if env.peek { y <= 140.0 * s && dx < 280.0 * s } else { y <= 1.0 && dx < 140.0 * s };
            if want != env.peek {
                env.peek = want;
                self.ui.set_env(env);
                self.present();
                self.kick();
            }
        }
    }

    fn test_notification(&mut self) {
        let (dnd, respect) = self.ui.dnd_state();
        log(&format!("test notification (Windows Do Not Disturb: {}, the island respects it: {} - a test is shown anyway)", if dnd { "on" } else { "off" }, if respect { "yes" } else { "no" }));
        self.ui.push_test_notification(crate::ui::NotifData {
            app: "Bloop".into(),
            aumid: String::new(),
            title: "Test notification".into(),
            body: "If you can see this, cards work".into(),
            icon: None,
        });
        self.present();
        self.kick();
    }

    fn on_notifs(&mut self) {
        let list: Vec<_> = self.shared.notifs.lock().map(|mut g| std::mem::take(&mut *g)).unwrap_or_default();
        if !list.is_empty() {
            let (dnd, respect) = self.ui.dnd_state();
            log(&format!("notifications: showing {} (Windows Do Not Disturb: {}, the island respects it: {})", list.len(), if dnd { "on" } else { "off" }, if respect { "yes - they are only listed" } else { "no" }));
        }
        for n in list {
            if self.settings.hide_windows_popups {
                crate::popups::hide_app(&n.aumid);
            }
            // Windows announces "Bluetooth turned on" etc. as toasts: those become the island's own status pill
            if let Some((key, on)) = crate::sysicons::toast_to_sys(&n.app, &n.title) {
                self.ui.push_status(StatusData::Sys { key: key.to_string(), on });
            } else {
                self.ui.push_notification(n);
            }
        }
        self.present();
        self.kick();
    }

    /// The mouse went down. Buttons press in; a drag on the progress bar starts here.
    fn on_down(&mut self, x: f32, y: f32) {
        unsafe {
            SetCapture(self.hwnd);
        }
        self.ui.mouse_down(x, y);
        self.apply_volume_request();
        self.present();
        self.kick();
    }

    /// The volume slider was dragged: tell Windows.
    fn apply_volume_request(&mut self) {
        if let Some(f) = self.ui.take_volume_request() {
            let app = self.shared.app.lock().map(|g| g.clone()).unwrap_or_default();
            crate::volume::set(&app, f);
        }
    }

    /// Ask Windows for the music app's volume (only while the player is open, once a second).
    fn refresh_volume(&mut self) {
        if self.ui.wants_volume() {
            let app = self.shared.app.lock().map(|g| g.clone()).unwrap_or_default();
            if let Some(v) = crate::volume::get(&app) {
                self.ui.set_volume(v);
            }
        }
    }

    /// The mouse went up: the click happens now (if it is still on what it went down on).
    fn on_up(&mut self, x: f32, y: f32) {
        unsafe {
            let _ = ReleaseCapture();
        }
        match self.ui.mouse_up(x, y) {
            // a long message unfolds on the first click; the next click (or a short message) opens the app
            Hit::Card(_) if self.ui.card_click() => {}
            Hit::Card(aumid) => {
                self.ui.dismiss_card();
                if aumid == UPDATE_AUMID {
                    install_update();
                } else {
                    open_app(&aumid);
                }
            }
            Hit::Bubble => {
                let aumid = self.ui.bubble_aumid();
                self.ui.dismiss_bubble();
                open_app(&aumid);
            }
            Hit::HomeRow(i) => {
                let aumid = self.ui.home_take(i);
                open_app(&aumid);
            }
            Hit::HomeClear => self.ui.clear_log(),
            Hit::Volume(_) => self.apply_volume_request(),
            Hit::LyricsToggle => {
                self.ui.toggle_lyrics();
                self.start_lyrics_lookup();
            }
            Hit::LyricLine(t) => media::command(&self.shared, Cmd::Seek(t as f64)),
            Hit::PlayPause => {
                media::command(&self.shared, Cmd::Toggle);
                self.ui.toggle_playing_locally();
                let now_playing = self.ui.is_playing();
                self.ui.play_sound(if now_playing { crate::ui::Sound::Play } else { crate::ui::Sound::Pause });
            }
            Hit::Prev => {
                media::command(&self.shared, Cmd::Prev);
                self.ui.play_sound(crate::ui::Sound::Skip);
            }
            Hit::Next => {
                media::command(&self.shared, Cmd::Next);
                self.ui.play_sound(crate::ui::Sound::Skip);
            }
            Hit::Seek(f) => media::command(&self.shared, Cmd::Seek(f as f64 * self.ui.duration())),
            Hit::OpenSource => {
                let app = self.shared.app_raw.lock().map(|g| g.clone()).unwrap_or_default();
                open_music_app(&app);
            }
            _ => {}
        }
        self.present();
        self.kick();
    }

    /// New settings: apply them at once, save them.
    fn apply_settings(&mut self, mut s: Settings) {
        s.clamp();
        if s == self.settings {
            return;
        }
        let size_changed = s.scale_pct != self.settings.scale_pct;
        if s.hide_windows_popups != self.settings.hide_windows_popups {
            if s.hide_windows_popups {
                crate::popups::hide();
            } else {
                crate::popups::restore();
            }
        }
        if s.sounds != self.settings.sounds {
            crate::sound::set_enabled(s.sounds);
        }
        self.settings = s.clone();
        crate::watch::PRIVACY_ON.store(s.privacy_dots, Ordering::Relaxed);
        crate::watch::DOWNLOAD_ON.store(s.download_pill, Ordering::Relaxed);
        save_settings(&s);
        self.ui.set_settings(&s);
        if size_changed {
            self.resize();
        }
        self.check_env();
        self.present();
        self.kick();
    }

    /// The island's size changed (the size setting): a new bitmap, the window resized and centred again.
    fn resize(&mut self) {
        let scale = self.dpi_scale * self.settings.scale_pct as f32 / 100.0;
        let (pw, ph) = self.ui.set_scale(scale);
        let (pw, ph) = (pw as i32, ph as i32);
        unsafe {
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: pw,
                    biHeight: -ph,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits: *mut c_void = std::ptr::null_mut();
            if let Ok(bmp) = CreateDIBSection(self.mem_dc, &bmi, DIB_RGB_COLORS, &mut bits, None, 0) {
                SelectObject(self.mem_dc, bmp);
                let _ = DeleteObject(self.bmp);
                self.bmp = bmp;
                self.bits = bits as *mut u8;
                self.pw = pw;
                self.ph = ph;
                self.center();
            }
        }
    }

    /// Something about the system changed (Wi-Fi, battery, Do Not Disturb ...).
    fn on_sys(&mut self) {
        let list: Vec<SysMsg> = self.shared.sys.lock().map(|mut g| std::mem::take(&mut *g)).unwrap_or_default();
        for m in list {
            match m {
                SysMsg::Toggle(key, on) => self.ui.push_status(StatusData::Sys { key, on }),
                SysMsg::Battery { label, level, charging, low } => self.ui.push_status(StatusData::Battery { label, level, charging, low }),
                SysMsg::Device { name, kind, connected } => self.ui.push_status(StatusData::Device { name, kind, connected }),
                SysMsg::Privacy { mic, cam, first } => self.ui.set_privacy(mic, cam, first),
                SysMsg::Download { name, bytes, done } => self.ui.push_status(StatusData::Download { name, bytes, done }),
                SysMsg::Dnd { on, first } => {
                    self.ui.set_dnd(on);
                    if !first {
                        self.ui.push_status(StatusData::Sys { key: "dnd".into(), on });
                    }
                }
            }
        }
        self.present();
        self.kick();
    }

    /// Something was copied. The island only learns what KIND of thing (never its content), and stays out of it when the program that
    /// copied it asked not to be recorded (password managers do).
    fn on_clipboard(&mut self) {
        use windows::Win32::System::DataExchange::{IsClipboardFormatAvailable, RegisterClipboardFormatW};
        let kind = unsafe {
            let private = ["ExcludeClipboardContentFromMonitorProcessing", "Clipboard Viewer Ignore"].iter().any(|name| {
                let w: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
                let id = RegisterClipboardFormatW(PCWSTR(w.as_ptr()));
                id != 0 && IsClipboardFormatAvailable(id).is_ok()
            });
            if private {
                return;
            }
            // 15 = files, 13 / 1 = text, 8 / 17 / 2 = a picture
            if IsClipboardFormatAvailable(15).is_ok() {
                "Files"
            } else if IsClipboardFormatAvailable(13).is_ok() || IsClipboardFormatAvailable(1).is_ok() {
                "Text"
            } else if IsClipboardFormatAvailable(8).is_ok() || IsClipboardFormatAvailable(17).is_ok() || IsClipboardFormatAvailable(2).is_ok() {
                "Image"
            } else {
                return;
            }
        };
        self.ui.push_status(StatusData::Copied { kind: kind.to_string() });
        self.present();
        self.kick();
    }

    /// The date and time line of the home view ("Friday, 27 November  14:05").
    fn update_clock(&mut self) {
        const DAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
        const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
        let t = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
        let text = format!("{}, {} {}  {:02}:{:02}", DAYS[t.wDayOfWeek as usize % 7], t.wDay, MONTHS[(t.wMonth as usize).clamp(1, 12) - 1], t.wHour, t.wMinute);
        if self.clock != text {
            self.clock = text.clone();
            self.ui.set_clock(text);
            if self.ui.is_open() {
                self.present();
            }
        }
    }

    /// A background update check finished.
    fn on_update(&mut self) {
        let Some((manual, out)) = update::take_result() else { return };
        let card = manual && settings_win::handle().is_none();
        match &out {
            Outcome::UpToDate if card => self.say("You have the newest version"),
            Outcome::Failed(m) if card => self.say(m),
            _ => {}
        }
        match out {
            Outcome::UpToDate => {
                log("update: up to date");
                if manual {
                    settings_win::set_update_status(&format!("You have the newest version ({}).", crate::version::VERSION), self.update_ready.is_some());
                }
            }
            Outcome::Failed(msg) => {
                log(&format!("update: {msg}"));
                if manual {
                    settings_win::set_update_status(&msg, self.update_ready.is_some());
                }
            }
            Outcome::Ready(ver) => {
                log(&format!("update: {ver} downloaded"));
                let first = self.update_ready.as_deref() != Some(ver.as_str());
                self.update_ready = Some(ver.clone());
                settings_win::set_update_status(&format!("Version {ver} is downloaded and ready."), true);
                if first {
                    self.ui.push_notification(crate::ui::NotifData {
                        app: "Bloop".into(),
                        aumid: UPDATE_AUMID.into(),
                        title: "Update ready".into(),
                        body: format!("Version {ver} - click here to restart and install"),
                        icon: None,
                    });
                    self.present();
                    self.kick();
                }
            }
        }
    }

    fn say(&mut self, text: &str) {
        self.ui.push_notification(crate::ui::NotifData { app: "Bloop".into(), aumid: String::new(), title: "Updates".into(), body: text.into(), icon: None });
        self.present();
        self.kick();
    }

    /// Time to look for a new version (every few hours, if the user allows it).
    fn on_update_timer(&mut self) {
        unsafe {
            SetTimer(self.hwnd, TIMER_UPD, 6 * 60 * 60 * 1000, None);
        }
        if self.settings.check_updates && self.update_ready.is_none() {
            update::spawn_check(self.hwnd.0 as isize, self.settings.update_repo.clone(), false);
        }
    }

    fn center(&self) {
        unsafe {
            let sw = GetSystemMetrics(SM_CXSCREEN);
            let _ = SetWindowPos(self.hwnd, HWND_TOPMOST, (sw - self.pw) / 2, 0, self.pw, self.ph, SWP_NOACTIVATE);
        }
    }
}

/// Bring the music app forward: its window if it is open, otherwise start it.
fn open_music_app(aumid: &str) {
    if aumid.is_empty() {
        return;
    }
    // "Spotify.exe" -> "spotify", "SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify" -> "spotify"
    let tail = aumid.rsplit('!').next().unwrap_or(aumid);
    let stem = tail.trim_end_matches(".exe").trim_end_matches(".EXE").to_lowercase();
    struct Find {
        stem: String,
        found: HWND,
    }
    unsafe extern "system" fn each(h: HWND, lp: LPARAM) -> BOOL {
        let f = &mut *(lp.0 as *mut Find);
        if !IsWindowVisible(h).as_bool() || GetWindow(h, GW_OWNER).map(|o| !o.0.is_null()).unwrap_or(false) {
            return BOOL(1);
        }
        let len = GetWindowTextLengthW(h);
        if len == 0 {
            return BOOL(1);
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(h, Some(&mut pid));
        let Ok(p) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return BOOL(1) };
        let mut buf = [0u16; 520];
        let mut n = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(p, PROCESS_NAME_WIN32, windows::core::PWSTR(buf.as_mut_ptr()), &mut n).is_ok();
        let _ = CloseHandle(p);
        if ok {
            let path = String::from_utf16_lossy(&buf[..n as usize]).to_lowercase();
            let name = std::path::Path::new(&path).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            if !f.stem.is_empty() && name.contains(&f.stem) {
                f.found = h;
                return BOOL(0);
            }
        }
        BOOL(1)
    }
    let mut f = Find { stem, found: HWND::default() };
    unsafe {
        let _ = EnumWindows(Some(each), LPARAM(&mut f as *mut Find as isize));
        if !f.found.0.is_null() {
            if IsIconic(f.found).as_bool() {
                let _ = ShowWindow(f.found, SW_RESTORE);
            }
            // Windows only lets the program in front switch windows: a tap on Alt makes it allow it
            keybd_event(VK_MENU.0 as u8, 0, KEYBD_EVENT_FLAGS(0), 0);
            keybd_event(VK_MENU.0 as u8, 0, KEYEVENTF_KEYUP, 0);
            let _ = SetForegroundWindow(f.found);
        } else {
            open_app(aumid);
        }
    }
}

/// Opens (or brings forward) the app a notification came from, by its Windows app id.
fn open_app(aumid: &str) {
    if aumid.is_empty() || aumid.contains('"') {
        return;
    }
    let target: Vec<u16> = format!("shell:AppsFolder\\{aumid}").encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        ShellExecuteW(None, w!("open"), PCWSTR(target.as_ptr()), None, None, SW_SHOWNORMAL);
    }
}

const APPROVED_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run");

/// The command Windows should run at sign-in: this very program, quoted.
fn autostart_command() -> Option<String> {
    std::env::current_exe().ok().map(|e| format!("\"{}\"", e.display()))
}

/// What the Run entry says now (None: no entry).
fn autostart_value() -> Option<String> {
    unsafe {
        let mut n = 0u32;
        if RegGetValueW(HKEY_CURRENT_USER, RUN_KEY, RUN_NAME, RRF_RT_REG_SZ, None, None, Some(&mut n)) != ERROR_SUCCESS || n < 2 {
            return None;
        }
        let mut buf = vec![0u16; (n as usize + 1) / 2 + 1];
        let mut n2 = (buf.len() * 2) as u32;
        if RegGetValueW(HKEY_CURRENT_USER, RUN_KEY, RUN_NAME, RRF_RT_REG_SZ, None, Some(buf.as_mut_ptr() as *mut c_void), Some(&mut n2)) != ERROR_SUCCESS {
            return None;
        }
        let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Some(String::from_utf16_lossy(&buf[..end]))
    }
}

/// Windows' own Startup apps list (Settings > Apps > Startup, Task Manager > Startup) can switch an entry off without removing it.
fn autostart_blocked() -> bool {
    unsafe {
        let mut data = [0u8; 12];
        let mut n = 12u32;
        if RegGetValueW(HKEY_CURRENT_USER, APPROVED_KEY, RUN_NAME, RRF_RT_REG_BINARY, None, Some(data.as_mut_ptr() as *mut c_void), Some(&mut n)) != ERROR_SUCCESS {
            return false;
        }
        data[0] & 1 == 1 // 02 / 06 = enabled, 03 / 07 = switched off by the user
    }
}

/// Will the island really start with Windows? (entry present, pointing at THIS program, not switched off in Windows' Startup list)
fn autostart_enabled() -> bool {
    match (autostart_value(), autostart_command()) {
        (Some(v), Some(c)) => v.eq_ignore_ascii_case(&c) && !autostart_blocked(),
        _ => false,
    }
}

fn set_autostart(on: bool) {
    unsafe {
        if on {
            if let Some(cmd) = autostart_command() {
                let v: Vec<u16> = cmd.encode_utf16().chain(std::iter::once(0)).collect();
                let r = RegSetKeyValueW(HKEY_CURRENT_USER, RUN_KEY, RUN_NAME, REG_SZ.0, Some(v.as_ptr() as *const c_void), (v.len() * 2) as u32);
                // and make sure Windows' Startup list has it switched ON (it may have been switched off there before)
                let enabled: [u8; 12] = [2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
                let r2 = RegSetKeyValueW(HKEY_CURRENT_USER, APPROVED_KEY, RUN_NAME, REG_BINARY.0, Some(enabled.as_ptr() as *const c_void), 12);
                log(&format!("autostart: on -> {cmd} (run entry {:?}, approved {:?})", r, r2));
            }
        } else {
            let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, RUN_NAME);
            log("autostart: off");
        }
    }
}

/// At start: an entry that points to an old place (the program was moved or renamed) is repaired, so "Start with Windows" keeps working.
fn repair_autostart() {
    if let (Some(v), Some(c)) = (autostart_value(), autostart_command()) {
        if !v.eq_ignore_ascii_case(&c) {
            log(&format!("autostart: entry pointed to {v}, now {c}"));
            set_autostart(true);
        }
    }
}

/// The tray icon: a small island (black pill, white outline, cover dot and equalizer bars), drawn here at the size the tray wants.
/// The app's own icon (embedded in the exe), at the size asked for.
pub fn app_icon(size: i32) -> Option<HICON> {
    unsafe {
        let inst = GetModuleHandleW(None).ok()?;
        let h = LoadImageW(inst, PCWSTR(1usize as *const u16), IMAGE_ICON, size, size, LR_DEFAULTCOLOR).ok()?;
        Some(HICON(h.0))
    }
}

pub fn make_tray_icon() -> Option<HICON> {
    use tiny_skia::Pixmap;
    unsafe {
        let n = (GetSystemMetrics(SM_CXSMICON).max(16)) as u32;
        let f = n as f32 / 32.0;
        let mut pm = Pixmap::new(n, n)?;
        let (x, y, w, h) = (1.0 * f, 5.0 * f, 30.0 * f, 22.0 * f);
        crate::gfx::fill_rrect(&mut pm, x, y, w, h, [h / 2.0; 4], [255, 255, 255], 1.0);
        let b = 2.6 * f;
        crate::gfx::fill_rrect(&mut pm, x + b, y + b, w - 2.0 * b, h - 2.0 * b, [(h - 2.0 * b) / 2.0; 4], [0, 0, 0], 1.0);
        let d = 8.0 * f;
        crate::gfx::fill_rrect(&mut pm, 6.6 * f, 16.0 * f - d / 2.0, d, d, [d / 2.0; 4], [255, 255, 255], 1.0);
        for (i, bh) in [7.0f32, 11.0, 6.0].iter().enumerate() {
            let (bw, bh) = (2.6 * f, bh * f);
            crate::gfx::fill_rrect(&mut pm, (15.8 + i as f32 * 4.0) * f, 16.0 * f - bh / 2.0, bw, bh, [bw / 2.0; 4], [255, 255, 255], 1.0);
        }
        // premultiplied RGBA -> straight BGRA, the way a Windows icon wants it
        let mut bgra = Vec::with_capacity((n * n * 4) as usize);
        for p in pm.data().chunks_exact(4) {
            let a = p[3] as f32;
            let un = |c: u8| if a > 0.0 { ((c as f32) * 255.0 / a).round().min(255.0) as u8 } else { 0 };
            bgra.extend_from_slice(&[un(p[2]), un(p[1]), un(p[0]), p[3]]);
        }
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: n as i32, biHeight: -(n as i32), biPlanes: 1, biBitCount: 32, biCompression: BI_RGB.0, ..Default::default() },
            ..Default::default()
        };
        let screen = GetDC(None);
        let mut bits: *mut c_void = std::ptr::null_mut();
        let color = CreateDIBSection(screen, &bmi, DIB_RGB_COLORS, &mut bits, None, 0);
        ReleaseDC(None, screen);
        let color = color.ok()?;
        std::ptr::copy_nonoverlapping(bgra.as_ptr(), bits as *mut u8, bgra.len());
        let mask = CreateBitmap(n as i32, n as i32, 1, 1, None);
        let info = ICONINFO { fIcon: BOOL(1), xHotspot: 0, yHotspot: 0, hbmMask: mask, hbmColor: color };
        let icon = CreateIconIndirect(&info).ok();
        let _ = DeleteObject(color);
        let _ = DeleteObject(mask);
        icon
    }
}

fn tray_data(hwnd: HWND) -> NOTIFYICONDATAW {
    let mut nid = NOTIFYICONDATAW { cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32, hWnd: hwnd, uID: 1, ..Default::default() };
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TRAY;
    for (i, c) in "Bloop".encode_utf16().enumerate() {
        nid.szTip[i] = c;
    }
    nid
}

fn add_tray_icon(hwnd: HWND) {
    unsafe {
        let mut nid = tray_data(hwnd);
        let sz = unsafe { GetSystemMetrics(SM_CXSMICON).max(16) };
        if let Some(icon) = app_icon(sz).or_else(make_tray_icon) {
            nid.hIcon = icon;
        }
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
        if !nid.hIcon.is_invalid() {
            let _ = DestroyIcon(nid.hIcon);
        }
    }
}

fn remove_tray_icon(hwnd: HWND) {
    unsafe {
        let nid = tray_data(hwnd);
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

fn open_settings() {
    if let Some(s) = with_app(|a| a.settings.clone()) {
        let icon = app_icon(32).or_else(make_tray_icon);
        settings_win::open(&s, autostart_enabled(), icon);
    }
}

/// Swap the downloaded update in and restart (the new copy waits for this one to end).
fn install_update() {
    match update::apply() {
        Ok(()) => unsafe {
            let h = MAIN_HWND.load(Ordering::Relaxed);
            let _ = DestroyWindow(HWND(h as *mut _));
        },
        Err(e) => {
            log(&format!("update: install failed: {e}"));
            settings_win::set_update_status(&format!("Could not install: {e}"), true);
        }
    }
}

/// "Open file" dialog for a program file.
fn pick_exe() -> Option<std::path::PathBuf> {
    use windows::Win32::UI::Controls::Dialogs::{GetOpenFileNameW, OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST, OPENFILENAMEW};
    unsafe {
        let mut buf = [0u16; 1040];
        let filter: Vec<u16> = "Bloop program (*.exe)\0*.exe\0\0".encode_utf16().collect();
        let mut ofn = OPENFILENAMEW {
            lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
            hwndOwner: settings_win::handle().unwrap_or_default(),
            lpstrFilter: PCWSTR(filter.as_ptr()),
            lpstrFile: windows::core::PWSTR(buf.as_mut_ptr()),
            nMaxFile: buf.len() as u32,
            Flags: OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
            ..Default::default()
        };
        if GetOpenFileNameW(&mut ofn).as_bool() {
            let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            Some(std::path::PathBuf::from(String::from_utf16_lossy(&buf[..n])))
        } else {
            None
        }
    }
}

/// What the settings window asks for.
fn on_settings_action(a: Action) {
    match a {
        Action::Save(s, autostart) => {
            // the running island follows the saved settings right away (no restart)
            with_app(|app| app.apply_settings(s));
            if autostart != autostart_enabled() {
                set_autostart(autostart);
            }
        }
        Action::CheckNow(s) => {
            let hwnd = with_app(|app| {
                app.apply_settings(s.clone());
                app.hwnd.0 as isize
            });
            if let Some(h) = hwnd {
                if update::busy() {
                    settings_win::set_update_status("Already checking...", false);
                } else {
                    settings_win::set_busy("Checking (and downloading if there is a newer version)...");
                    update::spawn_check(h, s.update_repo, true);
                }
            }
        }
        Action::Install => install_update(),
        Action::InstallFile => {
            if let Some(path) = pick_exe() {
                match update::stage_file(&path) {
                    Ok(()) => install_update(),
                    Err(e) => {
                        log(&format!("update: file install failed: {e}"));
                        settings_win::set_update_status(&format!("Could not use that file: {e}"), false);
                    }
                }
            }
        }
        Action::Test => {
            with_app(|a| a.test_notification());
        }
        Action::Quit => unsafe {
            let h = MAIN_HWND.load(Ordering::Relaxed);
            let _ = DestroyWindow(HWND(h as *mut _));
        },
    }
}

const WM_CLIPBOARDUPDATE: u32 = 0x031D;

extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let xy = |lp: LPARAM| ((lp.0 & 0xFFFF) as i16 as f32, ((lp.0 >> 16) & 0xFFFF) as i16 as f32);
    unsafe {
        match msg {
            WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as isize),
            WM_MOUSEMOVE => {
                with_app(|a| {
                    if !a.tracking {
                        let mut t = TRACKMOUSEEVENT { cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: hwnd, dwHoverTime: 0 };
                        let _ = TrackMouseEvent(&mut t);
                        a.tracking = true;
                    }
                    let (x, y) = xy(lp);
                    a.ui.set_cursor(Some((x, y)));
                    a.ui.mouse_move(x);
                    a.apply_volume_request();
                    a.kick();
                });
                return LRESULT(0);
            }
            WM_CLIPBOARDUPDATE => {
                with_app(|a| a.on_clipboard());
                return LRESULT(0);
            }
            WM_MOUSELEAVE => {
                with_app(|a| {
                    a.tracking = false;
                    a.ui.set_cursor(None);
                    a.kick();
                });
                return LRESULT(0);
            }
            WM_LBUTTONDOWN => {
                let (x, y) = xy(lp);
                with_app(|a| a.on_down(x, y));
                return LRESULT(0);
            }
            WM_LBUTTONUP => {
                let (x, y) = xy(lp);
                with_app(|a| a.on_up(x, y));
                return LRESULT(0);
            }
            WM_MOUSEWHEEL => {
                let notches = ((wp.0 >> 16) as i16) as f32 / 120.0;
                with_app(|a| {
                    a.ui.wheel(notches);
                    a.present();
                    a.kick();
                });
                return LRESULT(0);
            }
            WM_SETCURSOR => {
                // a hand over the things that can be clicked
                if (lp.0 & 0xFFFF) as u32 == HTCLIENT {
                    let mut pt = POINT::default();
                    let _ = GetCursorPos(&mut pt);
                    let _ = ScreenToClient(hwnd, &mut pt);
                    let hand = with_app(|a| a.ui.wants_hand(pt.x as f32, pt.y as f32)).unwrap_or(false);
                    if let Ok(c) = LoadCursorW(None, if hand { IDC_HAND } else { IDC_ARROW }) {
                        SetCursor(c);
                        return LRESULT(1);
                    }
                }
            }
            // (no context menu: everything lives in the settings window, which the tray icon opens)
            WM_RBUTTONUP => return LRESULT(0),
            WM_TIMER if wp.0 == TIMER_ID => {
                with_app(|a| a.on_timer());
                return LRESULT(0);
            }
            WM_TIMER if wp.0 == TIMER_ENV => {
                with_app(|a| a.check_env());
                return LRESULT(0);
            }
            WM_TIMER if wp.0 == TIMER_UPD => {
                with_app(|a| a.on_update_timer());
                return LRESULT(0);
            }
            WM_TRAY => {
                match (lp.0 & 0xFFFF) as u32 {
                    WM_LBUTTONUP | WM_RBUTTONUP => open_settings(),
                    _ => {}
                }
                return LRESULT(0);
            }
            m if m != 0 && m == TASKBAR_CREATED.load(Ordering::Relaxed) => {
                add_tray_icon(hwnd);
                return LRESULT(0);
            }
            WM_SYS => {
                with_app(|a| a.on_sys());
                return LRESULT(0);
            }
            WM_UPDATE => {
                with_app(|a| a.on_update());
                return LRESULT(0);
            }
            WM_TIMER if wp.0 == TIMER_PEEK => {
                with_app(|a| a.check_peek());
                return LRESULT(0);
            }
            WM_NOTIF => {
                with_app(|a| a.on_notifs());
                return LRESULT(0);
            }
            WM_LYRICS => {
                with_app(|a| a.on_lyrics());
                return LRESULT(0);
            }
            WM_ENV => {
                with_app(|a| a.check_env());
                return LRESULT(0);
            }
            WM_MEDIA => {
                with_app(|a| a.on_media());
                return LRESULT(0);
            }
            WM_DISPLAYCHANGE => {
                with_app(|a| a.center());
                return LRESULT(0);
            }
            // Windows is shutting down / the user signs out: the banners must be back before we are gone
            WM_ENDSESSION if wp.0 != 0 => {
                crate::popups::restore();
                return LRESULT(0);
            }
            WM_DESTROY => {
                let _ = windows::Win32::System::DataExchange::RemoveClipboardFormatListener(hwnd);
                // Windows' own banners come back
                crate::popups::restore();
                remove_tray_icon(hwnd);
                PostQuitMessage(0);
                return LRESULT(0);
            }
            _ => {}
        }
        DefWindowProcW(hwnd, msg, wp, lp)
    }
}

pub fn run() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        // only one island at a time
        // (right after an update the old copy may still be closing: wait for it a few seconds)
        let after_update = std::env::args().any(|a| a == "--after-update");
        let mut tries = if after_update { 60 } else { 1 };
        let _mutex = loop {
            let m = CreateMutexW(None, true, w!("Local\\DynamicIslandNative"));
            if GetLastError() != ERROR_ALREADY_EXISTS {
                break m;
            }
            if let Ok(h) = m {
                let _ = CloseHandle(h);
            }
            tries -= 1;
            if tries <= 0 {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        };
        update::cleanup();
        repair_autostart();
        let _ = RoInitialize(RO_INIT_SINGLETHREADED);

        let hinst = GetModuleHandleW(None).unwrap_or_default();
        let class = w!("DynamicIslandNativeWindow");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinst.into(),
            lpszClassName: class,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            ..Default::default()
        };
        RegisterClassW(&wc);

        let dpi_scale = GetDpiForSystem() as f32 / 96.0;
        let settings = load_settings();
        crate::sound::set_enabled(settings.sounds);
        // (a banner switch left over from a run that could not tidy up is put back first)
        crate::popups::restore();
        if settings.hide_windows_popups {
            crate::popups::hide();
        }
        let scale = dpi_scale * settings.scale_pct as f32 / 100.0;
        let mut ui = Ui::new(scale);
        ui.set_settings(&settings);
        let (pw, ph) = ui.size_px();
        let (pw, ph) = (pw as i32, ph as i32);
        let sw = GetSystemMetrics(SM_CXSCREEN);
        let hwnd = match CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class,
            w!("Bloop"),
            WS_POPUP,
            (sw - pw) / 2,
            0,
            pw,
            ph,
            None,
            None,
            hinst,
            None,
        ) {
            Ok(h) => h,
            Err(e) => {
                log(&format!("CreateWindowExW failed: {e}"));
                return;
            }
        };

        // the picture the window shows: a 32-bit bitmap we draw into
        let screen = GetDC(None);
        let mem_dc = CreateCompatibleDC(screen);
        ReleaseDC(None, screen);
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: pw,
                biHeight: -ph,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let Ok(bmp) = CreateDIBSection(mem_dc, &bmi, DIB_RGB_COLORS, &mut bits, None, 0) else {
            log("CreateDIBSection failed");
            return;
        };
        SelectObject(mem_dc, bmp);

        let shared = Arc::new(Shared::default());
        crate::watch::PRIVACY_ON.store(settings.privacy_dots, Ordering::Relaxed);
        crate::watch::DOWNLOAD_ON.store(settings.download_pill, Ordering::Relaxed);
        APP.with(|a| {
            *a.borrow_mut() = Some(App { hwnd, ui, mem_dc, bits: bits as *mut u8, pw, ph, shared: shared.clone(), tracking: false, timer: Need::Rest, last: Instant::now(), bmp, dpi_scale, settings, update_ready: None, clock: String::new(), ticks: 0 });
        });
        with_app(|a| a.present());
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        media::start(hwnd.0 as isize, shared.clone());
        notify::start(hwnd.0 as isize, shared.clone());
        crate::watch::start(hwnd.0 as isize, shared.clone());
        sys::start(hwnd.0 as isize, shared);
        let _ = windows::Win32::System::DataExchange::AddClipboardFormatListener(hwnd);
        SetTimer(hwnd, TIMER_ENV, 1000, None);
        SetTimer(hwnd, TIMER_UPD, 25_000, None); // the first look for a new version, shortly after start
        settings_win::set_sink(Box::new(on_settings_action));
        TASKBAR_CREATED.store(RegisterWindowMessageW(w!("TaskbarCreated")), Ordering::Relaxed);
        add_tray_icon(hwnd);
        MAIN_HWND.store(hwnd.0 as isize, Ordering::Relaxed);
        let _ = SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, None, Some(on_foreground), 0, 0, WINEVENT_OUTOFCONTEXT);

        // While something moves, the loop draws exactly one frame per screen refresh (DwmFlush waits for the next one), which is what
        // makes it smooth. When nothing moves it just sleeps in GetMessage and uses no CPU at all.
        let mut msg = MSG::default();
        'outer: loop {
            if with_app(|a| a.is_animating()).unwrap_or(false) {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    if msg.message == WM_QUIT {
                        break 'outer;
                    }
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                if with_app(|a| a.is_animating()).unwrap_or(false) {
                    with_app(|a| a.on_timer());
                    if DwmFlush().is_err() {
                        std::thread::sleep(std::time::Duration::from_millis(8)); // no desktop composition: plain 120 fps pacing
                    }
                }
            } else {
                if !GetMessageW(&mut msg, None, 0, 0).as_bool() {
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        if autostart_enabled() {
            // (nothing to clean up: the entry stays until you switch it off in the menu)
        }
    }
}
