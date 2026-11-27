//! The settings window: a borderless, rounded window that shows the self-drawn `Panel` (see panel.rs).
//! It exists only while it is open, so it costs nothing the rest of the time.

use crate::panel::{Act, Panel};
use crate::settings::Settings;
use std::cell::RefCell;
use std::ffi::c_void;
use std::time::Instant;
use windows::core::w;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForSystem};
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT};
use windows::Win32::UI::WindowsAndMessaging::*;

const WM_MOUSELEAVE: u32 = 0x02A3;
const TIMER_ANIM: usize = 1;

struct Win {
    hwnd: HWND,
    panel: Panel,
    dc: HDC,
    bmp: HBITMAP,
    bits: *mut u8,
    tracking: bool,
    timer: Option<u32>,
    /// the title bar colours last given to Windows
    title: (bool, u32),
    last: Instant,
}

thread_local! {
    static WIN: RefCell<Option<Win>> = const { RefCell::new(None) };
    static SINK: RefCell<Option<Box<dyn FnMut(Action)>>> = const { RefCell::new(None) };
    /// what the update check last said (kept while the window is closed)
    static STATUS: RefCell<(String, bool)> = const { RefCell::new((String::new(), false)) };
}

fn with_win<R>(f: impl FnOnce(&mut Win) -> R) -> Option<R> {
    WIN.with(|w| w.try_borrow_mut().ok().and_then(|mut g| g.as_mut().map(f)))
}

/// Is the window open?
pub fn handle() -> Option<HWND> {
    WIN.with(|w| w.try_borrow().ok().and_then(|g| g.as_ref().map(|x| x.hwnd)))
}

pub enum Action {
    /// "Save": apply the settings to the running island, keep them, set Start with Windows
    Save(Settings, bool),
    CheckNow(Settings),
    /// swap the downloaded update in and restart
    Install,
    Test,
    Quit,
    InstallFile,
}

/// Where the window reports its changes to (set once by the main window).
pub fn set_sink(f: Box<dyn FnMut(Action)>) {
    SINK.with(|s| *s.borrow_mut() = Some(f));
}

fn emit(a: Action) {
    // take the callback out while it runs (it may open message loops of its own)
    let cb = SINK.with(|s| s.borrow_mut().take());
    if let Some(mut f) = cb {
        f(a);
        SINK.with(|s| *s.borrow_mut() = Some(f));
    }
}

/// Show the status line of the update check. `ready`: a downloaded update waits (the button then installs it).
pub fn set_update_status(text: &str, ready: bool) {
    STATUS.with(|s| *s.borrow_mut() = (text.to_string(), ready));
    with_win(|w| {
        w.panel.set_status(text, ready);
        w.draw();
    });
}

/// "Checking..." while the check runs.
pub fn set_busy(text: &str) {
    with_win(|w| {
        w.panel.set_busy(text);
        w.draw();
    });
}

impl Win {
    /// Draw the panel and show it in the window.
    fn draw(&mut self) {
        let tc = self.panel.title_colors();
        if tc != self.title {
            self.title = tc;
            unsafe {
                let dark = BOOL(tc.0 as i32);
                let _ = DwmSetWindowAttribute(self.hwnd, DWMWINDOWATTRIBUTE(20), &dark as *const BOOL as *const c_void, 4);
                let _ = DwmSetWindowAttribute(self.hwnd, DWMWINDOWATTRIBUTE(35), &tc.1 as *const u32 as *const c_void, 4);
            }
        }
        let pm = self.panel.render();
        let (pw, ph) = (pm.width() as i32, pm.height() as i32);
        unsafe {
            let dst = std::slice::from_raw_parts_mut(self.bits, (pw * ph * 4) as usize);
            for (d, s) in dst.chunks_exact_mut(4).zip(pm.data().chunks_exact(4)) {
                d[0] = s[2];
                d[1] = s[1];
                d[2] = s[0];
                d[3] = 255;
            }
            let wdc = GetDC(self.hwnd);
            let _ = BitBlt(wdc, 0, 0, pw, ph, self.dc, 0, 0, SRCCOPY);
            ReleaseDC(self.hwnd, wdc);
        }
        // the live island and the sliding switches need a timer (60 fps while something moves fast, 30 fps for the equalizer ...)
        let want = self.panel.frame_ms();
        if want != self.timer {
            unsafe {
                match want {
                    Some(ms) => {
                        if self.timer.is_none() {
                            self.last = Instant::now();
                        }
                        SetTimer(self.hwnd, TIMER_ANIM, ms, None);
                    }
                    None => {
                        let _ = KillTimer(self.hwnd, TIMER_ANIM);
                    }
                }
            }
            self.timer = want;
        }
    }
}

fn make_dib(dc: HDC, w: i32, h: i32) -> Option<(HBITMAP, *mut u8)> {
    unsafe {
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: w, biHeight: -h, biPlanes: 1, biBitCount: 32, biCompression: BI_RGB.0, ..Default::default() },
            ..Default::default()
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let bmp = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &mut bits, None, 0).ok()?;
        Some((bmp, bits as *mut u8))
    }
}

impl Win {
    /// The window's inside changed size.
    fn resize(&mut self, w: i32, h: i32) {
        if w < 2 || h < 2 {
            return;
        }
        self.panel.resize(w as u32, h as u32);
        if let Some((bmp, bits)) = make_dib(self.dc, w, h) {
            unsafe {
                SelectObject(self.dc, bmp);
                let _ = DeleteObject(self.bmp);
            }
            self.bmp = bmp;
            self.bits = bits;
        }
    }
}

pub fn open(current: &Settings, autostart: bool, icon: Option<HICON>) {
    if let Some(h) = handle() {
        unsafe {
            let _ = ShowWindow(h, SW_SHOW);
            let _ = SetForegroundWindow(h);
        }
        return;
    }
    unsafe {
        let inst = GetModuleHandleW(None).unwrap_or_default();
        let class = w!("DynamicIslandSettingsPanel");
        let wc = WNDCLASSW { lpfnWndProc: Some(proc), hInstance: inst.into(), lpszClassName: class, hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(), hIcon: icon.unwrap_or_default(), ..Default::default() };
        RegisterClassW(&wc);

        let dpi = GetDpiForSystem();
        let scale = dpi as f32 / 96.0;
        let (sw, sh) = (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
        // an ordinary window: title bar with minimize / maximize / close, can be resized; 900 x 600 inside (less on a small screen)
        let style = WS_OVERLAPPEDWINDOW;
        let want_w = (Panel::DEFAULT_SIZE.0 * scale).min(sw as f32 * 0.92);
        let want_h = (Panel::DEFAULT_SIZE.1 * scale).min(sh as f32 * 0.88);
        let mut r = RECT { left: 0, top: 0, right: want_w as i32, bottom: want_h as i32 };
        let _ = AdjustWindowRectExForDpi(&mut r, style, false, WINDOW_EX_STYLE(0), dpi);
        let (ow, oh) = (r.right - r.left, r.bottom - r.top);
        let Ok(hwnd) = CreateWindowExW(WINDOW_EX_STYLE(0), class, w!("Dynamic Island - Settings"), style, (sw - ow) / 2, ((sh - oh) / 2).max(0), ow, oh, None, None, inst, None) else {
            return;
        };
        // dark title bar (and its colour like the window), where Windows supports it
        let dark = BOOL(1);
        let _ = DwmSetWindowAttribute(hwnd, DWMWINDOWATTRIBUTE(20), &dark as *const BOOL as *const c_void, 4);
        let caption: u32 = 0x0020_2020;
        let _ = DwmSetWindowAttribute(hwnd, DWMWINDOWATTRIBUTE(35), &caption as *const u32 as *const c_void, 4);
        if let Some(i) = icon {
            SendMessageW(hwnd, WM_SETICON, WPARAM(0), LPARAM(i.0 as isize));
            SendMessageW(hwnd, WM_SETICON, WPARAM(1), LPARAM(i.0 as isize));
        }
        let mut cr = RECT::default();
        let _ = GetClientRect(hwnd, &mut cr);
        let (pw, ph) = ((cr.right - cr.left).max(2), (cr.bottom - cr.top).max(2));
        let mut panel = Panel::new(scale, current.clone(), autostart, (pw as u32, ph as u32));
        let (st, ready) = STATUS.with(|s| s.borrow().clone());
        if !st.is_empty() {
            panel.set_status(&st, ready);
        }
        let screen = GetDC(None);
        let dc = CreateCompatibleDC(screen);
        ReleaseDC(None, screen);
        let Some((bmp, bits)) = make_dib(dc, pw, ph) else {
            let _ = DestroyWindow(hwnd);
            return;
        };
        SelectObject(dc, bmp);
        WIN.with(|w| *w.borrow_mut() = Some(Win { hwnd, panel, dc, bmp, bits, tracking: false, timer: None, title: (true, 0), last: Instant::now() }));
        with_win(|w| w.draw());
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }
}

/// What the panel asked for.
fn handle_act(act: Act) {
    match act {
        // (editing only changes the draft in the window; the running island follows when "Save" is pressed)
        Act::Changed(_) => {}
        Act::Save(s, auto) => {
            emit(Action::Save(s, auto));
            with_win(|w| w.panel.mark_saved());
        }
        Act::CheckNow => {
            let s = with_win(|w| w.panel.s.clone());
            if let Some(s) = s {
                emit(Action::CheckNow(s));
            }
        }
        Act::Install => emit(Action::Install),
        Act::Test => emit(Action::Test),
        Act::Quit => emit(Action::Quit),
        Act::InstallFile => emit(Action::InstallFile),
    }
}

extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let xy = |lp: LPARAM| ((lp.0 & 0xFFFF) as i16 as f32, ((lp.0 >> 16) & 0xFFFF) as i16 as f32);
    unsafe {
        match msg {
            WM_ERASEBKGND => return LRESULT(1),
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                with_win(|w| {
                    let (pw, ph) = w.panel.size_px();
                    let _ = BitBlt(hdc, 0, 0, pw as i32, ph as i32, w.dc, 0, 0, SRCCOPY);
                });
                let _ = EndPaint(hwnd, &ps);
                return LRESULT(0);
            }
            WM_SIZE => {
                if wp.0 != 1 {
                    // (1 = minimized)
                    let (cw, ch) = ((lp.0 & 0xFFFF) as i32, ((lp.0 >> 16) & 0xFFFF) as i32);
                    with_win(|w| {
                        w.resize(cw, ch);
                        w.draw();
                    });
                }
                return LRESULT(0);
            }
            WM_GETMINMAXINFO => {
                let dpi = GetDpiForSystem();
                let k = dpi as f32 / 96.0;
                let mut r = RECT { left: 0, top: 0, right: (Panel::MIN_SIZE.0 * k) as i32, bottom: (Panel::MIN_SIZE.1 * k) as i32 };
                let _ = AdjustWindowRectExForDpi(&mut r, WS_OVERLAPPEDWINDOW, false, WINDOW_EX_STYLE(0), dpi);
                let info = &mut *(lp.0 as *mut MINMAXINFO);
                info.ptMinTrackSize = POINT { x: r.right - r.left, y: r.bottom - r.top };
                return LRESULT(0);
            }
            WM_MOUSEMOVE => {
                let (x, y) = xy(lp);
                let act = with_win(|w| {
                    if !w.tracking {
                        let mut t = TRACKMOUSEEVENT { cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: hwnd, dwHoverTime: 0 };
                        let _ = TrackMouseEvent(&mut t);
                        w.tracking = true;
                    }
                    let a = w.panel.mouse_move(x, y);
                    w.draw();
                    a
                })
                .flatten();
                if let Some(a) = act {
                    handle_act(a);
                    with_win(|w| w.draw());
                }
                return LRESULT(0);
            }
            WM_MOUSELEAVE => {
                with_win(|w| {
                    w.tracking = false;
                    w.panel.leave();
                    w.draw();
                });
                return LRESULT(0);
            }
            WM_LBUTTONDOWN => {
                let (x, y) = xy(lp);
                SetCapture(hwnd);
                let act = with_win(|w| {
                    let a = w.panel.mouse_down(x, y);
                    w.draw();
                    a
                })
                .flatten();
                if let Some(a) = act {
                    handle_act(a);
                    with_win(|w| w.draw());
                }
                return LRESULT(0);
            }
            WM_LBUTTONUP => {
                let (x, y) = xy(lp);
                let _ = ReleaseCapture();
                let act = with_win(|w| w.panel.mouse_up(x, y)).flatten();
                if let Some(a) = act {
                    handle_act(a);
                }
                with_win(|w| w.draw());
                return LRESULT(0);
            }
            WM_MOUSEWHEEL => {
                let notches = ((wp.0 >> 16) as i16) as f32 / 120.0;
                with_win(|w| {
                    w.panel.wheel(notches);
                    w.draw();
                });
                return LRESULT(0);
            }
            WM_SETCURSOR => {
                if (lp.0 & 0xFFFF) as u32 == HTCLIENT {
                    let hand = with_win(|w| w.panel.hand()).unwrap_or(false);
                    if let Ok(c) = LoadCursorW(None, if hand { IDC_HAND } else { IDC_ARROW }) {
                        SetCursor(c);
                        return LRESULT(1);
                    }
                }
            }
            WM_TIMER if wp.0 == TIMER_ANIM => {
                with_win(|w| {
                    let now = Instant::now();
                    let dt = now.duration_since(w.last).as_secs_f32().min(0.1);
                    w.last = now;
                    w.panel.tick(dt);
                    w.draw();
                });
                return LRESULT(0);
            }
            WM_CLOSE => {
                let dirty = with_win(|w| w.panel.dirty()).unwrap_or(false);
                if dirty {
                    let r = MessageBoxW(hwnd, w!("Save your changes to the island?"), w!("Dynamic Island - Settings"), MB_YESNOCANCEL | MB_ICONQUESTION);
                    if r == IDCANCEL {
                        return LRESULT(0);
                    }
                    if r == IDYES {
                        let save = with_win(|w| (w.panel.s.clone(), w.panel.autostart_draft()));
                        if let Some((s, a)) = save {
                            emit(Action::Save(s, a));
                        }
                    }
                }
                let _ = DestroyWindow(hwnd);
                return LRESULT(0);
            }
            WM_DESTROY => {
                WIN.with(|w| {
                    if let Some(x) = w.borrow_mut().take() {
                        let _ = KillTimer(hwnd, TIMER_ANIM);
                        let _ = DeleteObject(x.bmp);
                        let _ = DeleteDC(x.dc);
                    }
                });
                return LRESULT(0);
            }
            _ => {}
        }
        DefWindowProcW(hwnd, msg, wp, lp)
    }
}
