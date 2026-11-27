//! The settings window: an ordinary Windows window with check boxes, sliders and lists. Every change is applied at once and saved.
//! It exists only while it is open.

use crate::settings::{MaxMode, Settings};
use std::cell::RefCell;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{InitCommonControlsEx, ICC_BAR_CLASSES, INITCOMMONCONTROLSEX};
use windows::Win32::UI::HiDpi::GetDpiForSystem;
use windows::Win32::UI::WindowsAndMessaging::*;

// control ids
const CHK_AUTOSTART: i32 = 101;
const CHK_PILL: i32 = 102;
const CHK_PF: i32 = 103;
const CHK_LYR: i32 = 104;
const CHK_EQ: i32 = 105;
const CHK_NOTIF: i32 = 106;
const CHK_BUB: i32 = 107;
const TRK_CARD: i32 = 108;
const LBL_CARD: i32 = 109;
const CMB_MAX: i32 = 110;
const CHK_FS: i32 = 111;
const TRK_SCALE: i32 = 112;
const LBL_SCALE: i32 = 113;
const TRK_SPEED: i32 = 114;
const LBL_SPEED: i32 = 115;
const CHK_UPD: i32 = 116;
const EDT_REPO: i32 = 117;
const BTN_CHECK: i32 = 118;
const LBL_UPD: i32 = 119;
const BTN_RESET: i32 = 120;
const BTN_CLOSE: i32 = 121;

// raw window-message / style numbers (comctl32 and the stock controls)
const BM_GETCHECK: u32 = 0x00F0;
const BM_SETCHECK: u32 = 0x00F1;
const TBM_GETPOS: u32 = 0x0400;
const TBM_SETPOS: u32 = 0x0405;
const TBM_SETRANGE: u32 = 0x0406;
const CB_ADDSTRING: u32 = 0x0143;
const CB_GETCURSEL: u32 = 0x0147;
const CB_SETCURSEL: u32 = 0x014E;
const BS_AUTOCHECKBOX: u32 = 0x0003;
const BS_PUSHBUTTON: u32 = 0x0000;
const CBS_DROPDOWNLIST: u32 = 0x0003;
const ES_AUTOHSCROLL: u32 = 0x0080;
const EN_CHANGE: u32 = 0x0300;
const CBN_SELCHANGE: u32 = 1;

struct Win {
    hwnd: HWND,
    font: HFONT,
    bold: HFONT,
}

thread_local! {
    static WIN: RefCell<Option<Win>> = const { RefCell::new(None) };
}

/// Is the window open? (the message loop needs this for the Tab key)
pub fn handle() -> Option<HWND> {
    WIN.with(|w| w.try_borrow().ok().and_then(|g| g.as_ref().map(|x| x.hwnd)))
}

fn child(parent: HWND, class: PCWSTR, text: &str, style: u32, x: i32, y: i32, wd: i32, ht: i32, id: i32) -> HWND {
    let t: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let inst = GetModuleHandleW(None).unwrap_or_default();
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            PCWSTR(t.as_ptr()),
            WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | style),
            x,
            y,
            wd,
            ht,
            parent,
            HMENU(id as isize as *mut _),
            inst,
            None,
        )
        .unwrap_or_default()
    }
}

fn item(h: HWND, id: i32) -> HWND {
    unsafe { GetDlgItem(h, id).unwrap_or_default() }
}

fn send(h: HWND, id: i32, msg: u32, wp: usize, lp: isize) -> isize {
    unsafe { SendMessageW(item(h, id), msg, WPARAM(wp), LPARAM(lp)).0 }
}

fn set_check(h: HWND, id: i32, on: bool) {
    send(h, id, BM_SETCHECK, on as usize, 0);
}

fn get_check(h: HWND, id: i32) -> bool {
    send(h, id, BM_GETCHECK, 0, 0) == 1
}

fn set_text(h: HWND, id: i32, text: &str) {
    let t: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let _ = SetWindowTextW(item(h, id), PCWSTR(t.as_ptr()));
    }
}

fn get_text(h: HWND, id: i32) -> String {
    unsafe {
        let ctl = item(h, id);
        let n = GetWindowTextLengthW(ctl).max(0) as usize;
        let mut buf = vec![0u16; n + 1];
        let got = GetWindowTextW(ctl, &mut buf) as usize;
        String::from_utf16_lossy(&buf[..got])
    }
}

/// Put the values of `s` into the controls.
fn fill(h: HWND, s: &Settings, autostart: bool) {
    set_check(h, CHK_AUTOSTART, autostart);
    set_check(h, CHK_PILL, s.show_music_pill);
    set_check(h, CHK_PF, s.hide_when_player_front);
    set_check(h, CHK_LYR, s.lyrics_button);
    set_check(h, CHK_EQ, s.accent_eq);
    set_check(h, CHK_NOTIF, s.notifications);
    set_check(h, CHK_BUB, s.use_bubble);
    send(h, TRK_CARD, TBM_SETPOS, 1, s.card_secs as isize);
    send(h, CMB_MAX, CB_SETCURSEL, match s.maximized { MaxMode::Notch => 0, MaxMode::Hide => 1, MaxMode::Normal => 2 }, 0);
    set_check(h, CHK_FS, s.fullscreen_hide);
    send(h, TRK_SCALE, TBM_SETPOS, 1, s.scale_pct as isize);
    send(h, TRK_SPEED, TBM_SETPOS, 1, s.speed_pct as isize);
    set_check(h, CHK_UPD, s.check_updates);
    set_text(h, EDT_REPO, &s.update_repo);
    labels(h);
}

/// The value texts next to the sliders.
fn labels(h: HWND) {
    set_text(h, LBL_CARD, &format!("{} s", send(h, TRK_CARD, TBM_GETPOS, 0, 0)));
    set_text(h, LBL_SCALE, &format!("{} %", send(h, TRK_SCALE, TBM_GETPOS, 0, 0)));
    set_text(h, LBL_SPEED, &format!("{} %", send(h, TRK_SPEED, TBM_GETPOS, 0, 0)));
}

/// What the controls say right now.
fn read(h: HWND, base: &Settings) -> Settings {
    let mut s = base.clone();
    s.show_music_pill = get_check(h, CHK_PILL);
    s.hide_when_player_front = get_check(h, CHK_PF);
    s.lyrics_button = get_check(h, CHK_LYR);
    s.accent_eq = get_check(h, CHK_EQ);
    s.notifications = get_check(h, CHK_NOTIF);
    s.use_bubble = get_check(h, CHK_BUB);
    s.card_secs = send(h, TRK_CARD, TBM_GETPOS, 0, 0).max(0) as u32;
    s.maximized = match send(h, CMB_MAX, CB_GETCURSEL, 0, 0) {
        1 => MaxMode::Hide,
        2 => MaxMode::Normal,
        _ => MaxMode::Notch,
    };
    s.fullscreen_hide = get_check(h, CHK_FS);
    s.scale_pct = send(h, TRK_SCALE, TBM_GETPOS, 0, 0).max(0) as u32;
    s.speed_pct = send(h, TRK_SPEED, TBM_GETPOS, 0, 0).max(0) as u32;
    s.check_updates = get_check(h, CHK_UPD);
    s.update_repo = get_text(h, EDT_REPO).trim().to_string();
    s.clamp();
    s
}

/// Show the status line of the update check. `ready`: a downloaded update waits (the button then installs it).
pub fn set_update_status(text: &str, ready: bool) {
    READY.with(|r| r.set(ready));
    if let Some(h) = handle() {
        set_text(h, LBL_UPD, text);
        set_text(h, BTN_CHECK, if ready { "Restart now" } else { "Check now" });
    }
}

pub fn open(current: &Settings, autostart: bool) {
    if let Some(h) = handle() {
        unsafe {
            let _ = ShowWindow(h, SW_RESTORE);
            let _ = SetForegroundWindow(h);
        }
        return;
    }
    unsafe {
        let icc = INITCOMMONCONTROLSEX { dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32, dwICC: ICC_BAR_CLASSES };
        let _ = InitCommonControlsEx(&icc);
        let inst = GetModuleHandleW(None).unwrap_or_default();
        let class = w!("DynamicIslandSettings");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(proc),
            hInstance: inst.into(),
            lpszClassName: class,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: HBRUSH((COLOR_BTNFACE.0 + 1) as isize as *mut _),
            ..Default::default()
        };
        RegisterClassW(&wc);

        let k = GetDpiForSystem() as f32 / 96.0;
        let p = |v: i32| (v as f32 * k).round() as i32;
        let (cw, ch) = (p(470), p(620));
        let sw = GetSystemMetrics(SM_CXSCREEN);
        let sh = GetSystemMetrics(SM_CYSCREEN);
        // a client area of cw x ch -> the outer size
        let style = WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
        let mut r = RECT { left: 0, top: 0, right: cw, bottom: ch };
        let _ = AdjustWindowRectEx(&mut r, style, false, WINDOW_EX_STYLE(0));
        let (ow, oh) = (r.right - r.left, r.bottom - r.top);
        let Ok(hwnd) = CreateWindowExW(WINDOW_EX_STYLE(0), class, w!("Dynamic Island - Settings"), style, (sw - ow) / 2, ((sh - oh) / 2).max(0), ow, oh, None, None, inst, None) else {
            return;
        };

        // the system's own message font (Segoe UI), and a bold one for the headings
        let mut ncm = NONCLIENTMETRICSW { cbSize: std::mem::size_of::<NONCLIENTMETRICSW>() as u32, ..Default::default() };
        let _ = SystemParametersInfoW(SPI_GETNONCLIENTMETRICS, ncm.cbSize, Some(&mut ncm as *mut _ as *mut _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));
        let font = CreateFontIndirectW(&ncm.lfMessageFont);
        let mut lf = ncm.lfMessageFont;
        lf.lfWeight = 700;
        let bold = CreateFontIndirectW(&lf);

        // layout
        let left = p(20);
        let full = cw - 2 * left;
        let mut y = p(14);
        let row = p(24);
        let mut all: Vec<HWND> = Vec::new();
        let mut heads: Vec<HWND> = Vec::new();
        let heading = |y: &mut i32, text: &str, heads: &mut Vec<HWND>| {
            heads.push(child(hwnd, w!("STATIC"), text, 0, left, *y, full, p(20), 0));
            *y += p(24);
        };
        let check = |y: &mut i32, text: &str, id: i32, all: &mut Vec<HWND>| {
            all.push(child(hwnd, w!("BUTTON"), text, BS_AUTOCHECKBOX | WS_TABSTOP.0, left + p(8), *y, full - p(8), p(22), id));
            *y += row;
        };
        let slider = |y: &mut i32, text: &str, id: i32, lbl: i32, lo: i32, hi: i32, all: &mut Vec<HWND>| {
            all.push(child(hwnd, w!("STATIC"), text, 0, left + p(8), *y + p(3), p(150), p(20), 0));
            let t = child(hwnd, w!("msctls_trackbar32"), "", WS_TABSTOP.0, left + p(160), *y, p(200), p(26), id);
            { SendMessageW(t, TBM_SETRANGE, WPARAM(1), LPARAM(((hi as isize) << 16) | lo as isize)) };
            all.push(t);
            all.push(child(hwnd, w!("STATIC"), "", 0, left + p(370), *y + p(3), p(70), p(20), lbl));
            *y += p(30);
        };

        check(&mut y, "Start with Windows", CHK_AUTOSTART, &mut all);
        y += p(6);
        heading(&mut y, "Music", &mut heads);
        check(&mut y, "Show the small pill while music plays", CHK_PILL, &mut all);
        check(&mut y, "Hide it while the music app itself is in front", CHK_PF, &mut all);
        check(&mut y, "Lyrics button in the player", CHK_LYR, &mut all);
        check(&mut y, "Equalizer in the cover's colour", CHK_EQ, &mut all);
        y += p(6);
        heading(&mut y, "Notifications", &mut heads);
        check(&mut y, "Show notifications in the island", CHK_NOTIF, &mut all);
        check(&mut y, "Small bubble beside the music pill (otherwise a card)", CHK_BUB, &mut all);
        slider(&mut y, "A card stays for", TRK_CARD, LBL_CARD, 2, 20, &mut all);
        y += p(6);
        heading(&mut y, "Windows", &mut heads);
        all.push(child(hwnd, w!("STATIC"), "Under a maximized window", 0, left + p(8), y + p(4), p(170), p(20), 0));
        let combo = child(hwnd, w!("COMBOBOX"), "", CBS_DROPDOWNLIST | WS_VSCROLL.0 | WS_TABSTOP.0, left + p(180), y, p(220), p(120), CMB_MAX);
        for t in ["Notch (slim, hangs from the top)", "Hide (top edge brings it back)", "Do nothing"] {
            let wide: Vec<u16> = t.encode_utf16().chain(std::iter::once(0)).collect();
            SendMessageW(combo, CB_ADDSTRING, WPARAM(0), LPARAM(wide.as_ptr() as isize));
        }
        all.push(combo);
        y += p(32);
        check(&mut y, "Hide for full-screen apps (videos, games)", CHK_FS, &mut all);
        y += p(6);
        heading(&mut y, "Look", &mut heads);
        slider(&mut y, "Size", TRK_SCALE, LBL_SCALE, 80, 140, &mut all);
        slider(&mut y, "Animation speed", TRK_SPEED, LBL_SPEED, 50, 200, &mut all);
        y += p(6);
        heading(&mut y, "Updates", &mut heads);
        check(&mut y, "Check for updates automatically", CHK_UPD, &mut all);
        all.push(child(hwnd, w!("STATIC"), "GitHub repository", 0, left + p(8), y + p(4), p(120), p(20), 0));
        all.push(child(hwnd, w!("EDIT"), "", WS_BORDER.0 | ES_AUTOHSCROLL | WS_TABSTOP.0, left + p(130), y, p(210), p(24), EDT_REPO));
        all.push(child(hwnd, w!("BUTTON"), "Check now", BS_PUSHBUTTON | WS_TABSTOP.0, left + p(350), y - p(1), p(90), p(26), BTN_CHECK));
        y += p(30);
        all.push(child(hwnd, w!("STATIC"), &format!("Version {}", crate::version::VERSION), 0, left + p(8), y, full - p(8), p(20), LBL_UPD));
        y += p(34);
        all.push(child(hwnd, w!("BUTTON"), "Reset to defaults", BS_PUSHBUTTON | WS_TABSTOP.0, left, y, p(140), p(28), BTN_RESET));
        all.push(child(hwnd, w!("BUTTON"), "Close", BS_PUSHBUTTON | WS_TABSTOP.0, cw - left - p(100), y, p(100), p(28), BTN_CLOSE));
        for h in &all {
            if h.0 != std::ptr::null_mut() {
                let _ = SendMessageW(*h, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));
            }
        }
        for h in &heads {
            let _ = SendMessageW(*h, WM_SETFONT, WPARAM(bold.0 as usize), LPARAM(1));
        }
        // the window may be a little taller than planned
        let mut cr = RECT { left: 0, top: 0, right: cw, bottom: y + p(28) + p(16) };
        let _ = AdjustWindowRectEx(&mut cr, style, false, WINDOW_EX_STYLE(0));
        let _ = SetWindowPos(hwnd, None, (sw - (cr.right - cr.left)) / 2, ((sh - (cr.bottom - cr.top)) / 2).max(0), cr.right - cr.left, cr.bottom - cr.top, SWP_NOZORDER);

        WIN.with(|w| *w.borrow_mut() = Some(Win { hwnd, font, bold }));
        fill(hwnd, current, autostart);
        if READY.with(|r| r.get()) {
            set_text(hwnd, BTN_CHECK, "Restart now");
            set_text(hwnd, LBL_UPD, "An update is downloaded and ready.");
        }
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }
}

pub enum Action {
    /// the settings changed: apply and save them
    Changed(Settings),
    CheckNow(Settings),
    Reset,
    Autostart(bool),
    /// swap the downloaded update in and restart
    Install,
}

thread_local! {
    static READY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static SINK: RefCell<Option<Box<dyn FnMut(Action)>>> = const { RefCell::new(None) };
    static BASE: RefCell<Settings> = RefCell::new(Settings::default());
}

/// Where the window reports its changes to (set once by the main window).
pub fn set_sink(f: Box<dyn FnMut(Action)>) {
    SINK.with(|s| *s.borrow_mut() = Some(f));
}

pub fn set_base(s: &Settings) {
    BASE.with(|b| *b.borrow_mut() = s.clone());
}

/// Put new values into the open window (after "Reset to defaults").
pub fn refresh(s: &Settings, autostart: bool) {
    if let Some(h) = handle() {
        set_base(s);
        fill(h, s, autostart);
    }
}

fn emit(a: Action) {
    // take the callback out while it runs (it may open message loops of its own)
    let cb = SINK.with(|s| s.borrow_mut().take());
    if let Some(mut f) = cb {
        f(a);
        SINK.with(|s| *s.borrow_mut() = Some(f));
    }
}

extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_COMMAND => {
                let id = (wp.0 & 0xFFFF) as i32;
                let note = ((wp.0 >> 16) & 0xFFFF) as u32;
                match id {
                    BTN_CLOSE => {
                        let _ = DestroyWindow(hwnd);
                    }
                    BTN_RESET => emit(Action::Reset),
                    BTN_CHECK if READY.with(|r| r.get()) => emit(Action::Install),
                    BTN_CHECK => {
                        let s = BASE.with(|b| read(hwnd, &b.borrow()));
                        emit(Action::CheckNow(s));
                    }
                    CHK_AUTOSTART => emit(Action::Autostart(get_check(hwnd, CHK_AUTOSTART))),
                    EDT_REPO if note != EN_CHANGE => {}
                    CMB_MAX if note != CBN_SELCHANGE => {}
                    _ => {
                        let s = BASE.with(|b| read(hwnd, &b.borrow()));
                        BASE.with(|b| *b.borrow_mut() = s.clone());
                        emit(Action::Changed(s));
                    }
                }
                return LRESULT(0);
            }
            WM_HSCROLL => {
                labels(hwnd);
                let s = BASE.with(|b| read(hwnd, &b.borrow()));
                BASE.with(|b| *b.borrow_mut() = s.clone());
                emit(Action::Changed(s));
                return LRESULT(0);
            }
            WM_CLOSE => {
                let _ = DestroyWindow(hwnd);
                return LRESULT(0);
            }
            WM_DESTROY => {
                WIN.with(|w| {
                    if let Some(x) = w.borrow_mut().take() {
                        let _ = DeleteObject(x.font);
                        let _ = DeleteObject(x.bold);
                    }
                });
                return LRESULT(0);
            }
            _ => {}
        }
        DefWindowProcW(hwnd, msg, wp, lp)
    }
}
