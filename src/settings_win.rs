//! The settings window: a borderless, rounded window that shows the self-drawn `Panel` (see panel.rs).
//! It exists only while it is open, so it costs nothing the rest of the time.

use crate::panel::{Act, Panel};
use crate::settings::Settings;
use std::cell::RefCell;
use std::ffi::c_void;
use std::time::Instant;
use windows::core::w;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForSystem;
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT};
use windows::Win32::UI::WindowsAndMessaging::*;

const WM_MOUSELEAVE: u32 = 0x02A3;
const TIMER_ANIM: usize = 1;
const VK_ESCAPE: usize = 0x1B;

struct Win {
    hwnd: HWND,
    panel: Panel,
    dc: HDC,
    bmp: HBITMAP,
    bits: *mut u8,
    tracking: bool,
    timer: bool,
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
    /// the settings changed: apply and save them
    Changed(Settings),
    CheckNow(Settings),
    Reset,
    Autostart(bool),
    /// swap the downloaded update in and restart
    Install,
    Test,
    Quit,
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

/// Put new values into the open window (after "Reset to defaults").
pub fn refresh(s: &Settings, autostart: bool) {
    with_win(|w| {
        w.panel.set_settings(s.clone(), autostart);
        w.draw();
    });
}

impl Win {
    /// Draw the panel and hand the picture to Windows.
    fn draw(&mut self) {
        let pm = self.panel.render();
        let (pw, ph) = (pm.width() as i32, pm.height() as i32);
        unsafe {
            let dst = std::slice::from_raw_parts_mut(self.bits, (pw * ph * 4) as usize);
            for (d, s) in dst.chunks_exact_mut(4).zip(pm.data().chunks_exact(4)) {
                d[0] = s[2];
                d[1] = s[1];
                d[2] = s[0];
                d[3] = s[3];
            }
            let size = SIZE { cx: pw, cy: ph };
            let src = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8 };
            let _ = UpdateLayeredWindow(self.hwnd, None, None, Some(&size), self.dc, Some(&src), COLORREF(0), Some(&blend), ULW_ALPHA);
        }
        // switches that are still sliding need a 60 fps timer
        let moving = self.panel.animating();
        if moving != self.timer {
            unsafe {
                if moving {
                    self.last = Instant::now();
                    SetTimer(self.hwnd, TIMER_ANIM, 16, None);
                } else {
                    let _ = KillTimer(self.hwnd, TIMER_ANIM);
                }
            }
            self.timer = moving;
        }
    }
}

pub fn open(current: &Settings, autostart: bool) {
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
        let wc = WNDCLASSW { lpfnWndProc: Some(proc), hInstance: inst.into(), lpszClassName: class, hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(), ..Default::default() };
        RegisterClassW(&wc);

        let scale = GetDpiForSystem() as f32 / 96.0;
        let (sw, sh) = (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
        // at most 88 % of the screen height (the rest scrolls)
        let mut panel = Panel::new(scale, current.clone(), autostart, sh as f32 * 0.88 / scale);
        let (st, ready) = STATUS.with(|s| s.borrow().clone());
        if !st.is_empty() {
            panel.set_status(&st, ready);
        }
        let (pw, ph) = panel.size_px();
        let (pw, ph) = (pw as i32, ph as i32);

        let Ok(hwnd) = CreateWindowExW(WS_EX_LAYERED, class, w!("Dynamic Island - Settings"), WS_POPUP | WS_MINIMIZEBOX | WS_SYSMENU, (sw - pw) / 2, ((sh - ph) / 2).max(0), pw, ph, None, None, inst, None) else {
            return;
        };
        let screen = GetDC(None);
        let dc = CreateCompatibleDC(screen);
        ReleaseDC(None, screen);
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, biWidth: pw, biHeight: -ph, biPlanes: 1, biBitCount: 32, biCompression: BI_RGB.0, ..Default::default() },
            ..Default::default()
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let Ok(bmp) = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &mut bits, None, 0) else {
            let _ = DestroyWindow(hwnd);
            return;
        };
        SelectObject(dc, bmp);
        WIN.with(|w| *w.borrow_mut() = Some(Win { hwnd, panel, dc, bmp, bits: bits as *mut u8, tracking: false, timer: false, last: Instant::now() }));
        with_win(|w| w.draw());
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }
}

/// What the panel asked for.
fn handle_act(hwnd: HWND, act: Act) {
    match act {
        Act::Changed(s) => emit(Action::Changed(s)),
        Act::Autostart(on) => emit(Action::Autostart(on)),
        Act::CheckNow => {
            let s = with_win(|w| w.panel.s.clone());
            if let Some(s) = s {
                emit(Action::CheckNow(s));
            }
        }
        Act::Install => emit(Action::Install),
        Act::Reset => emit(Action::Reset),
        Act::Test => emit(Action::Test),
        Act::Quit => emit(Action::Quit),
        Act::Close => unsafe {
            let _ = DestroyWindow(hwnd);
        },
        Act::DragWindow => unsafe {
            let _ = ReleaseCapture();
            with_win(|w| w.panel.cancel());
            SendMessageW(hwnd, WM_NCLBUTTONDOWN, WPARAM(HTCAPTION as usize), LPARAM(0));
        },
    }
}

extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let xy = |lp: LPARAM| ((lp.0 & 0xFFFF) as i16 as f32, ((lp.0 >> 16) & 0xFFFF) as i16 as f32);
    unsafe {
        match msg {
            // the window is a popup with no frame: every pixel belongs to the client area
            WM_NCHITTEST => {
                let r = DefWindowProcW(hwnd, msg, wp, lp);
                return if r.0 == HTNOWHERE as isize { LRESULT(HTCLIENT as isize) } else { r };
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
                    handle_act(hwnd, a);
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
                    handle_act(hwnd, a);
                    with_win(|w| w.draw());
                }
                return LRESULT(0);
            }
            WM_LBUTTONUP => {
                let (x, y) = xy(lp);
                let _ = ReleaseCapture();
                let act = with_win(|w| w.panel.mouse_up(x, y)).flatten();
                if let Some(a) = act {
                    handle_act(hwnd, a);
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
            WM_KEYDOWN if wp.0 == VK_ESCAPE => {
                let _ = DestroyWindow(hwnd);
                return LRESULT(0);
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
