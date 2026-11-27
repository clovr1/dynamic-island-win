//! The settings window's contents, drawn by ourselves: a side list of pages on the left, the settings of the chosen page on the right,
//! like an ordinary Windows app. This file only knows about pixels and mouse positions; `settings_win.rs` puts it into a window.

use crate::gfx::{fill_rrect, Fonts, Weight};
use crate::settings::{MaxMode, Settings};
use tiny_skia::{Color, Pixmap};

/// width of the list of pages (logical pixels)
const SIDE: f32 = 224.0;
const WIN_BG: [u8; 3] = [32, 32, 32];
const PAGE_BG: [u8; 3] = [39, 39, 39];
const CARD: [u8; 3] = [49, 49, 49];
const CARD_HOVER: [u8; 3] = [55, 55, 55];
const EDGE: [u8; 3] = [64, 64, 64];
const TEXT: [u8; 3] = [255, 255, 255];
const DIM: [u8; 3] = [166, 166, 166];
const ACCENT: [u8; 3] = [96, 205, 255];
const NAV_ON: [u8; 3] = [55, 55, 55];
const NAV_HOVER: [u8; 3] = [45, 45, 45];

pub const PAGES: [&str; 6] = ["General", "Music", "Notifications", "Windows", "Look", "Updates"];

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Id {
    Nav(u8),
    Autostart,
    Pill,
    PlayerFront,
    Lyrics,
    Eq,
    Notif,
    Bubble,
    CardSecs,
    Test,
    MaxOpt(u8),
    FsHide,
    Scale,
    Speed,
    CheckUpd,
    CheckNow,
    Reset,
    Quit,
}

/// What the panel asks its window to do.
pub enum Act {
    Changed(Settings),
    Autostart(bool),
    CheckNow,
    Install,
    Reset,
    Test,
    Quit,
}

enum Row {
    /// id, title, description, on
    Toggle(Id, &'static str, &'static str, bool),
    /// id, title, description, value, min, max, text
    Slider(Id, &'static str, &'static str, f32, f32, f32, String),
    /// title, description, chosen, options
    Radio(&'static str, &'static str, usize, [&'static str; 3]),
    /// id, title, description, button text, style (0 normal / 1 primary / 2 danger)
    Button(Id, &'static str, String, String, u8),
}

impl Row {
    fn height(&self) -> f32 {
        match self {
            Row::Toggle(_, _, d, _) => if d.is_empty() { 56.0 } else { 68.0 },
            Row::Button(_, _, d, _, _) => if d.is_empty() { 56.0 } else { 68.0 },
            Row::Slider(..) => 72.0,
            Row::Radio(..) => 68.0 + 3.0 * 42.0,
        }
    }
}

struct Hot {
    id: Id,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

pub struct Panel {
    fonts: Fonts,
    scale: f32,
    /// logical size of the window's inside
    w: f32,
    h: f32,
    pm: Pixmap,
    pub s: Settings,
    autostart: bool,
    status: String,
    ready: bool,
    busy: bool,
    page: u8,
    scroll: f32,
    max_scroll: f32,
    hot: Vec<Hot>,
    hover: Option<Id>,
    down: Option<Id>,
    drag: Option<Id>,
    /// moving values (switch positions): id, now, target
    anim: Vec<(Id, f32, f32)>,
}

fn rr(pm: &mut Pixmap, s: f32, x: f32, y: f32, w: f32, h: f32, r: f32, rgb: [u8; 3], a: f32) {
    fill_rrect(pm, x * s, y * s, w * s, h * s, [r * s; 4], rgb, a);
}

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let m = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t).round().clamp(0.0, 255.0) as u8;
    [m(0), m(1), m(2)]
}

impl Panel {
    /// The default (and smallest) size of the window's inside, logical pixels.
    pub const DEFAULT_SIZE: (f32, f32) = (900.0, 600.0);
    pub const MIN_SIZE: (f32, f32) = (700.0, 460.0);

    pub fn new(scale: f32, s: Settings, autostart: bool, size_px: (u32, u32)) -> Panel {
        let mut p = Panel {
            fonts: Fonts::new_ui(),
            scale,
            w: 1.0,
            h: 1.0,
            pm: Pixmap::new(1, 1).unwrap(),
            s,
            autostart,
            status: String::new(),
            ready: false,
            busy: false,
            page: 0,
            scroll: 0.0,
            max_scroll: 0.0,
            hot: Vec::new(),
            hover: None,
            down: None,
            drag: None,
            anim: Vec::new(),
        };
        p.status = format!("Updates come from github.com/{}", p.s.update_repo);
        p.resize(size_px.0, size_px.1);
        p
    }

    /// The window changed size.
    pub fn resize(&mut self, w_px: u32, h_px: u32) {
        let (w_px, h_px) = (w_px.max(2), h_px.max(2));
        if let Some(pm) = Pixmap::new(w_px, h_px) {
            self.pm = pm;
            self.w = w_px as f32 / self.scale;
            self.h = h_px as f32 / self.scale;
        }
    }

    pub fn size_px(&self) -> (u32, u32) {
        (self.pm.width(), self.pm.height())
    }

    pub fn set_settings(&mut self, s: Settings, autostart: bool) {
        self.s = s;
        self.autostart = autostart;
    }

    pub fn set_page(&mut self, page: u8) {
        self.page = page.min(PAGES.len() as u8 - 1);
        self.scroll = 0.0;
    }

    pub fn set_status(&mut self, text: &str, ready: bool) {
        self.status = text.to_string();
        self.ready = ready;
        self.busy = false;
    }

    pub fn set_busy(&mut self, text: &str) {
        self.status = text.to_string();
        self.busy = true;
    }

    fn rows(&self) -> Vec<Row> {
        let s = &self.s;
        match self.page {
            0 => vec![
                Row::Toggle(Id::Autostart, "Start with Windows", "Open the island automatically when you sign in", self.autostart),
                Row::Button(Id::Reset, "Reset all settings", "Put every setting back to how it was at the start".into(), "Reset".into(), 0),
                Row::Button(Id::Quit, "Quit Dynamic Island", "Close the island completely".into(), "Quit".into(), 2),
            ],
            1 => vec![
                Row::Toggle(Id::Pill, "Small pill while music plays", "Show the cover and equalizer at the top while a song plays", s.show_music_pill),
                Row::Toggle(Id::PlayerFront, "Hide it while the music app is in front", "No need for the pill when the player's own window is open", s.hide_when_player_front),
                Row::Toggle(Id::Lyrics, "Lyrics button", "Show a lyrics button in the player (lyrics come from lrclib.net)", s.lyrics_button),
                Row::Toggle(Id::Eq, "Equalizer in the cover's colour", "Otherwise the equalizer is white", s.accent_eq),
            ],
            2 => vec![
                Row::Toggle(Id::Notif, "Show notifications in the island", "Messages from your apps appear at the top of the screen", s.notifications),
                Row::Toggle(Id::Bubble, "Small bubble next to the music pill", "A small bubble while music plays, otherwise a card", s.use_bubble),
                Row::Slider(Id::CardSecs, "How long a card stays", "Seconds before it goes away", s.card_secs as f32, 2.0, 20.0, format!("{} s", s.card_secs)),
                Row::Button(Id::Test, "Test", "Show a test notification now".into(), "Show".into(), 0),
            ],
            3 => vec![
                Row::Radio("Under a maximized window", "What the island does when a window covers the top of the screen", match s.maximized { MaxMode::Notch => 0, MaxMode::Hide => 1, MaxMode::Normal => 2 }, ["Become a slim notch", "Hide (move the mouse to the top edge to bring it back)", "Do nothing"]),
                Row::Toggle(Id::FsHide, "Hide for full-screen apps", "Videos and games get the whole screen", s.fullscreen_hide),
            ],
            4 => vec![
                Row::Slider(Id::Scale, "Size", "Makes the whole island bigger or smaller", s.scale_pct as f32, 80.0, 140.0, format!("{} %", s.scale_pct)),
                Row::Slider(Id::Speed, "Animation speed", "Slower is calmer, faster is snappier", s.speed_pct as f32, 50.0, 200.0, format!("{} %", s.speed_pct)),
            ],
            _ => {
                let (label, style) = if self.ready {
                    ("Restart now".to_string(), 1)
                } else if self.busy {
                    ("Checking...".to_string(), 0)
                } else {
                    ("Check now".to_string(), 0)
                };
                vec![
                    Row::Toggle(Id::CheckUpd, "Check for updates automatically", "A few seconds after start and then every 6 hours", s.check_updates),
                    Row::Button(Id::CheckNow, "Dynamic Island", format!("Version {} - {}", crate::version::VERSION, self.status), label, style),
                ]
            }
        }
    }

    /// Moves the switches towards where they belong. True while something still moves.
    pub fn tick(&mut self, dt: f32) -> bool {
        let mut moving = false;
        for a in self.anim.iter_mut() {
            let d = a.2 - a.1;
            if d.abs() > 0.001 {
                let step = dt * 10.0;
                a.1 = if d.abs() <= step { a.2 } else { a.1 + step * d.signum() };
                moving = true;
            }
        }
        moving
    }

    pub fn animating(&self) -> bool {
        self.anim.iter().any(|a| (a.2 - a.1).abs() > 0.001)
    }

    fn want(&mut self, id: Id, target: f32) -> f32 {
        if let Some(a) = self.anim.iter_mut().find(|a| a.0 == id) {
            a.2 = target;
            return a.1;
        }
        self.anim.push((id, target, target));
        target
    }

    /// Text whose vertical middle is at `cy`.
    #[allow(clippy::too_many_arguments)]
    fn text_mid(&mut self, pm: &mut Pixmap, w: Weight, t: &str, x: f32, cy: f32, px: f32, rgb: [u8; 3]) {
        let s = self.scale;
        self.fonts.draw(pm, w, t, x * s, (cy + px * 0.36) * s, px * s, rgb, 1.0);
    }

    pub fn render(&mut self) -> &Pixmap {
        let s = self.scale;
        let mut pm = std::mem::replace(&mut self.pm, Pixmap::new(1, 1).unwrap());
        pm.fill(Color::from_rgba8(PAGE_BG[0], PAGE_BG[1], PAGE_BG[2], 255));
        self.hot.clear();
        let (w, h) = (self.w, self.h);

        // ---- the list of pages ----
        rr(&mut pm, s, 0.0, 0.0, SIDE, h, 0.0, WIN_BG, 1.0);
        self.fonts.draw(&mut pm, Weight::Semibold, "Dynamic Island", 22.0 * s, 40.0 * s, 17.0 * s, TEXT, 1.0);
        self.fonts.draw(&mut pm, Weight::Regular, &format!("Settings - version {}", crate::version::VERSION), 22.0 * s, 60.0 * s, 12.0 * s, DIM, 1.0);
        for (i, name) in PAGES.iter().enumerate() {
            let id = Id::Nav(i as u8);
            let (x, y, bw, bh) = (10.0, 84.0 + i as f32 * 42.0, SIDE - 20.0, 38.0);
            let on = self.page == i as u8;
            let hov = self.hover == Some(id);
            if on {
                rr(&mut pm, s, x, y, bw, bh, 5.0, NAV_ON, 1.0);
                rr(&mut pm, s, x, y + 11.0, 3.0, bh - 22.0, 1.5, ACCENT, 1.0);
            } else if hov {
                rr(&mut pm, s, x, y, bw, bh, 5.0, NAV_HOVER, 1.0);
            }
            self.text_mid(&mut pm, if on { Weight::Semibold } else { Weight::Regular }, name, x + 18.0, y + bh / 2.0, 14.5, TEXT);
            self.hot.push(Hot { id, x, y, w: bw, h: bh });
        }
        // quit, at the bottom of the list
        {
            let (x, y, bw, bh) = (10.0, h - 52.0, SIDE - 20.0, 38.0);
            let hov = self.hover == Some(Id::Quit);
            if hov {
                rr(&mut pm, s, x, y, bw, bh, 5.0, NAV_HOVER, 1.0);
            }
            self.text_mid(&mut pm, Weight::Regular, "Quit Dynamic Island", x + 18.0, y + bh / 2.0, 14.0, [255, 138, 128]);
            self.hot.push(Hot { id: Id::Quit, x, y, w: bw, h: bh });
        }

        // ---- the settings of the page ----
        let cx = SIDE + 36.0;
        let cw = (w - SIDE - 72.0).clamp(300.0, 760.0);
        self.fonts.draw(&mut pm, Weight::Semibold, PAGES[self.page as usize], cx * s, (60.0 - self.scroll) * s, 28.0 * s, TEXT, 1.0);
        let mut y = 86.0 - self.scroll;
        let rows = self.rows();
        for row in &rows {
            let rh = row.height();
            self.draw_row(&mut pm, row, cx, y, cw, rh);
            y += rh + 6.0;
        }
        let total = y + self.scroll + 20.0;
        self.max_scroll = (total - h).max(0.0);
        self.scroll = self.scroll.min(self.max_scroll);
        // a thin scroll bar
        if self.max_scroll > 0.0 {
            let vh = h - 16.0;
            let th = (vh * h / total).max(30.0);
            let ty = 8.0 + (vh - th) * (self.scroll / self.max_scroll);
            rr(&mut pm, s, w - 8.0, ty, 3.0, th, 1.5, [110, 110, 110], 0.9);
        }
        self.pm = pm;
        &self.pm
    }

    fn draw_row(&mut self, pm: &mut Pixmap, row: &Row, cx: f32, y: f32, cw: f32, rh: f32) {
        let s = self.scale;
        let right = cx + cw - 18.0;
        let card = |me: &Self, pm: &mut Pixmap, id: Option<Id>| {
            let hov = id.is_some() && me.hover == id;
            rr(pm, s, cx - 0.5, y - 0.5, cw + 1.0, rh + 1.0, 6.5, EDGE, 1.0);
            rr(pm, s, cx, y, cw, rh, 6.0, if hov { CARD_HOVER } else { CARD }, 1.0);
        };
        match row {
            Row::Toggle(id, title, desc, on) => {
                card(self, pm, Some(*id));
                let t = self.want(*id, if *on { 1.0 } else { 0.0 });
                let ease = t * t * (3.0 - 2.0 * t);
                self.title_desc(pm, title, desc, cx + 18.0, y, rh, cw - 150.0);
                // the switch: an outline with a dot (off) / an accent fill with a dark dot (on)
                let (tw, th) = (40.0, 20.0);
                let (tx, ty) = (right - tw, y + rh / 2.0 - th / 2.0);
                if ease < 0.5 {
                    rr(pm, s, tx, ty, tw, th, th / 2.0, [200, 200, 200], 1.0 - ease * 2.0);
                    rr(pm, s, tx + 1.2, ty + 1.2, tw - 2.4, th - 2.4, th / 2.0 - 1.2, mix(CARD, CARD_HOVER, 0.0), 1.0 - ease * 2.0);
                }
                rr(pm, s, tx, ty, tw, th, th / 2.0, ACCENT, ease);
                let d = 12.0 + ease * 2.0;
                let kx = tx + 4.0 + ease * (tw - 8.0 - 14.0) + (14.0 - d) / 2.0;
                rr(pm, s, kx, ty + (th - d) / 2.0, d, d, d / 2.0, mix([210, 210, 210], [20, 20, 20], ease), 1.0);
                let lab = if *on { "On" } else { "Off" };
                let lw = self.fonts.measure(Weight::Regular, lab, 13.5 * s) / s;
                self.text_mid(pm, Weight::Regular, lab, tx - 14.0 - lw, y + rh / 2.0, 13.5, TEXT);
                self.hot.push(Hot { id: *id, x: cx, y, w: cw, h: rh });
            }
            Row::Slider(id, title, desc, v, lo, hi, text) => {
                card(self, pm, None);
                self.title_desc(pm, title, desc, cx + 18.0, y, rh, cw * 0.42);
                let vw = 56.0;
                let sw = (cw * 0.34).clamp(120.0, 260.0);
                let sx = right - vw - 14.0 - sw;
                let ty = y + rh / 2.0;
                let frac = ((v - lo) / (hi - lo)).clamp(0.0, 1.0);
                rr(pm, s, sx, ty - 2.0, sw, 4.0, 2.0, [150, 150, 150], 1.0);
                rr(pm, s, sx, ty - 2.0, sw * frac, 4.0, 2.0, ACCENT, 1.0);
                let held = self.drag == Some(*id);
                let hov = self.hover == Some(*id);
                let kx = sx + sw * frac;
                rr(pm, s, kx - 10.0, ty - 10.0, 20.0, 20.0, 10.0, [69, 69, 69], 1.0);
                rr(pm, s, kx - 10.0, ty - 10.0, 20.0, 20.0, 10.0, [255, 255, 255], 0.08);
                let d = if held { 8.0 } else if hov { 14.0 } else { 12.0 };
                rr(pm, s, kx - d / 2.0, ty - d / 2.0, d, d, d / 2.0, ACCENT, 1.0);
                let tw = self.fonts.measure(Weight::Regular, text, 14.0 * s) / s;
                self.text_mid(pm, Weight::Regular, text, right - tw, ty, 14.0, TEXT);
                self.hot.push(Hot { id: *id, x: sx - 12.0, y, w: sw + 24.0, h: rh });
            }
            Row::Button(id, title, desc, label, style) => {
                card(self, pm, None);
                self.title_desc(pm, title, desc, cx + 18.0, y, rh, cw - 190.0);
                let (bw, bh) = (116.0, 32.0);
                let (bx, by) = (right - bw, y + rh / 2.0 - bh / 2.0);
                let hov = self.hover == Some(*id);
                let pressed = hov && self.down == Some(*id);
                let base = if *style == 1 { ACCENT } else { [64, 64, 64] };
                let col = if pressed { mix(base, [0, 0, 0], 0.22) } else if hov { mix(base, [255, 255, 255], 0.12) } else { base };
                rr(pm, s, bx, by, bw, bh, 5.0, col, 1.0);
                let tcol = match style {
                    1 => [0, 0, 0],
                    2 => [255, 138, 128],
                    _ => TEXT,
                };
                let t = self.fonts.fit(Weight::Semibold, label, 13.5 * s, (bw - 14.0) * s);
                let tw = self.fonts.measure(Weight::Semibold, &t, 13.5 * s) / s;
                self.text_mid(pm, Weight::Semibold, &t, bx + bw / 2.0 - tw / 2.0, by + bh / 2.0, 13.5, tcol);
                self.hot.push(Hot { id: *id, x: bx, y: by, w: bw, h: bh });
            }
            Row::Radio(title, desc, chosen, options) => {
                card(self, pm, None);
                self.title_desc(pm, title, desc, cx + 18.0, y, 68.0, cw - 40.0);
                for (i, name) in options.iter().enumerate() {
                    let oy = y + 68.0 + i as f32 * 42.0;
                    let id = Id::MaxOpt(i as u8);
                    rr(pm, s, cx + 18.0, oy, cw - 36.0, 1.0, 0.0, EDGE, 1.0);
                    if self.hover == Some(id) {
                        rr(pm, s, cx + 1.0, oy + 1.0, cw - 2.0, 41.0, 0.0, CARD_HOVER, 1.0);
                    }
                    let on = *chosen == i;
                    let (rx, ry) = (cx + 18.0, oy + 21.0);
                    if on {
                        rr(pm, s, rx, ry - 10.0, 20.0, 20.0, 10.0, ACCENT, 1.0);
                        rr(pm, s, rx + 6.0, ry - 4.0, 8.0, 8.0, 4.0, [0, 0, 0], 1.0);
                    } else {
                        rr(pm, s, rx, ry - 10.0, 20.0, 20.0, 10.0, [200, 200, 200], 1.0);
                        rr(pm, s, rx + 1.4, ry - 8.6, 17.2, 17.2, 8.6, CARD, 1.0);
                    }
                    self.text_mid(pm, Weight::Regular, name, rx + 34.0, ry, 14.0, TEXT);
                    self.hot.push(Hot { id, x: cx, y: oy, w: cw, h: 42.0 });
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn title_desc(&mut self, pm: &mut Pixmap, title: &str, desc: &str, x: f32, y: f32, rh: f32, max_w: f32) {
        let s = self.scale;
        if desc.is_empty() {
            self.text_mid(pm, Weight::Regular, title, x, y + rh / 2.0, 14.5, TEXT);
        } else {
            self.text_mid(pm, Weight::Regular, title, x, y + rh / 2.0 - 11.0, 14.5, TEXT);
            let d = self.fonts.fit(Weight::Regular, desc, 12.5 * s, max_w * s);
            self.fonts.draw(pm, Weight::Regular, &d, x * s, (y + rh / 2.0 + 12.0 + 4.5) * s, 12.5 * s, DIM, 1.0);
        }
    }

    // ---------- mouse ----------

    fn hit_at(&self, x: f32, y: f32) -> Option<Id> {
        let (x, y) = (x / self.scale, y / self.scale);
        self.hot.iter().find(|r| x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h).map(|r| r.id)
    }

    pub fn hand(&self) -> bool {
        self.hover.is_some()
    }

    pub fn leave(&mut self) {
        self.hover = None;
    }

    pub fn cancel(&mut self) {
        self.down = None;
        self.drag = None;
    }

    pub fn mouse_move(&mut self, x: f32, y: f32) -> Option<Act> {
        self.hover = self.hit_at(x, y);
        if let Some(id) = self.drag {
            return self.set_slider(id, x);
        }
        None
    }

    pub fn mouse_down(&mut self, x: f32, y: f32) -> Option<Act> {
        let id = self.hit_at(x, y);
        self.hover = id;
        self.down = id;
        match id {
            Some(Id::Nav(n)) => {
                self.set_page(n);
                self.down = None;
                None
            }
            Some(id @ (Id::CardSecs | Id::Scale | Id::Speed)) => {
                self.drag = Some(id);
                self.set_slider(id, x)
            }
            _ => None,
        }
    }

    pub fn mouse_up(&mut self, x: f32, y: f32) -> Option<Act> {
        let released = self.hit_at(x, y);
        let pressed = self.down.take();
        self.drag = None;
        if released.is_none() || released != pressed {
            return None;
        }
        let mut s = self.s.clone();
        match released? {
            Id::Autostart => {
                self.autostart = !self.autostart;
                return Some(Act::Autostart(self.autostart));
            }
            Id::Pill => s.show_music_pill = !s.show_music_pill,
            Id::PlayerFront => s.hide_when_player_front = !s.hide_when_player_front,
            Id::Lyrics => s.lyrics_button = !s.lyrics_button,
            Id::Eq => s.accent_eq = !s.accent_eq,
            Id::Notif => s.notifications = !s.notifications,
            Id::Bubble => s.use_bubble = !s.use_bubble,
            Id::FsHide => s.fullscreen_hide = !s.fullscreen_hide,
            Id::CheckUpd => s.check_updates = !s.check_updates,
            Id::MaxOpt(n) => {
                s.maximized = [MaxMode::Notch, MaxMode::Hide, MaxMode::Normal][n as usize % 3];
                if s.maximized == self.s.maximized {
                    return None;
                }
            }
            Id::Test => return Some(Act::Test),
            Id::CheckNow => return Some(if self.ready { Act::Install } else { Act::CheckNow }),
            Id::Reset => return Some(Act::Reset),
            Id::Quit => return Some(Act::Quit),
            _ => return None,
        }
        self.s = s.clone();
        Some(Act::Changed(s))
    }

    fn set_slider(&mut self, id: Id, x: f32) -> Option<Act> {
        let r = self.hot.iter().find(|r| r.id == id)?;
        let (sx, sw) = (r.x + 12.0, r.w - 24.0);
        let frac = ((x / self.scale - sx) / sw).clamp(0.0, 1.0);
        let (lo, hi) = match id {
            Id::CardSecs => (2.0, 20.0),
            Id::Scale => (80.0, 140.0),
            _ => (50.0, 200.0),
        };
        let v = (lo + frac * (hi - lo)).round() as u32;
        let slot = match id {
            Id::CardSecs => &mut self.s.card_secs,
            Id::Scale => &mut self.s.scale_pct,
            _ => &mut self.s.speed_pct,
        };
        if *slot == v {
            return None;
        }
        *slot = v;
        Some(Act::Changed(self.s.clone()))
    }

    pub fn wheel(&mut self, notches: f32) {
        self.scroll = (self.scroll - notches * 56.0).clamp(0.0, self.max_scroll);
    }
}
