//! The settings panel, drawn by ourselves in the island's own look (dark, rounded, Inter) instead of Windows' standard controls.
//! This file only knows about pixels and mouse positions; `settings_win.rs` puts it into a window.

use crate::gfx::{fill_rrect, Fonts, Weight};
use crate::settings::{MaxMode, Settings};
use tiny_skia::{Color, FillRule, LineCap, Mask, Paint, PathBuilder, Pixmap, Stroke, Transform};

/// logical width of the panel
pub const W: f32 = 440.0;
const HEADER: f32 = 80.0;
const BG: [u8; 3] = [17, 17, 19];
const CARD: [u8; 3] = [29, 29, 32];
const LINE: [u8; 3] = [48, 48, 53];
const DIM: [u8; 3] = [142, 142, 150];
const WHITE: [u8; 3] = [255, 255, 255];
const GREEN: [u8; 3] = [48, 209, 88];
const BLUE: [u8; 3] = [10, 132, 255];
const OFF: [u8; 3] = [72, 72, 78];

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Id {
    Drag,
    Close,
    Autostart,
    Pill,
    PlayerFront,
    Lyrics,
    Eq,
    Notif,
    Bubble,
    CardSecs,
    Test,
    Max,
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
    Close,
    /// the title area was pressed: let Windows move the window
    DragWindow,
}

enum Item {
    Section(&'static str),
    Toggle(Id, &'static str, bool),
    /// id, label, value, min, max, text
    Slider(Id, &'static str, f32, f32, f32, String),
    Segment(Id, &'static str, usize),
    /// (id, text, 0 normal / 1 primary / 2 danger)
    Buttons(Vec<(Id, String, u8)>),
    Note(String),
}

fn height(i: &Item) -> f32 {
    match i {
        Item::Toggle(..) => 46.0,
        Item::Slider(..) => 62.0,
        Item::Segment(..) => 84.0,
        Item::Buttons(_) => 56.0,
        Item::Note(_) => 38.0,
        Item::Section(_) => 0.0,
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
    height: f32,
    pm: Pixmap,
    pub s: Settings,
    autostart: bool,
    status: String,
    ready: bool,
    busy: bool,
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
    /// `max_h`: the tallest the window may be (logical pixels); a taller content scrolls.
    pub fn new(scale: f32, s: Settings, autostart: bool, max_h: f32) -> Panel {
        let mut p = Panel {
            fonts: Fonts::new(),
            scale,
            height: 0.0,
            pm: Pixmap::new(1, 1).unwrap(),
            s,
            autostart,
            status: String::new(),
            ready: false,
            busy: false,
            scroll: 0.0,
            max_scroll: 0.0,
            hot: Vec::new(),
            hover: None,
            down: None,
            drag: None,
            anim: Vec::new(),
        };
        p.status = format!("Updates from {}", p.s.update_repo);
        p.height = p.natural_height().min(max_h.max(300.0)).ceil();
        p.pm = Pixmap::new((W * scale).ceil() as u32, (p.height * scale).ceil() as u32).unwrap();
        p
    }

    pub fn size_px(&self) -> (u32, u32) {
        (self.pm.width(), self.pm.height())
    }

    pub fn set_settings(&mut self, s: Settings, autostart: bool) {
        self.s = s;
        self.autostart = autostart;
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

    fn items(&self) -> Vec<Item> {
        let s = &self.s;
        let check = if self.ready {
            ("Restart now".to_string(), 1)
        } else if self.busy {
            ("Checking...".to_string(), 0)
        } else {
            ("Check for updates".to_string(), 0)
        };
        vec![
            Item::Section("General"),
            Item::Toggle(Id::Autostart, "Start with Windows", self.autostart),
            Item::Section("Music"),
            Item::Toggle(Id::Pill, "Small pill while music plays", s.show_music_pill),
            Item::Toggle(Id::PlayerFront, "Hide it while the music app is in front", s.hide_when_player_front),
            Item::Toggle(Id::Lyrics, "Lyrics button in the player", s.lyrics_button),
            Item::Toggle(Id::Eq, "Equalizer in the cover's colour", s.accent_eq),
            Item::Section("Notifications"),
            Item::Toggle(Id::Notif, "Show notifications in the island", s.notifications),
            Item::Toggle(Id::Bubble, "Small bubble beside the music pill", s.use_bubble),
            Item::Slider(Id::CardSecs, "A card stays for", s.card_secs as f32, 2.0, 20.0, format!("{} s", s.card_secs)),
            Item::Buttons(vec![(Id::Test, "Show a test notification".into(), 0)]),
            Item::Section("Windows"),
            Item::Segment(Id::Max, "Under a maximized window", match s.maximized { MaxMode::Notch => 0, MaxMode::Hide => 1, MaxMode::Normal => 2 }),
            Item::Toggle(Id::FsHide, "Hide for full-screen apps", s.fullscreen_hide),
            Item::Section("Look"),
            Item::Slider(Id::Scale, "Size", s.scale_pct as f32, 80.0, 140.0, format!("{} %", s.scale_pct)),
            Item::Slider(Id::Speed, "Animation speed", s.speed_pct as f32, 50.0, 200.0, format!("{} %", s.speed_pct)),
            Item::Section("Updates"),
            Item::Toggle(Id::CheckUpd, "Check for updates automatically", s.check_updates),
            Item::Note(self.status.clone()),
            Item::Buttons(vec![(Id::CheckNow, check.0, check.1), (Id::Reset, "Reset to defaults".into(), 0)]),
            Item::Section(""),
            Item::Buttons(vec![(Id::Quit, "Quit Dynamic Island".into(), 2)]),
        ]
    }

    /// The height the whole content needs.
    fn natural_height(&self) -> f32 {
        let items = self.items();
        let mut y = HEADER + 4.0;
        let mut i = 0;
        while i < items.len() {
            if let Item::Section(t) = &items[i] {
                y += if t.is_empty() { 8.0 } else { 30.0 };
                let mut j = i + 1;
                while j < items.len() && !matches!(items[j], Item::Section(_)) {
                    y += height(&items[j]);
                    j += 1;
                }
                y += 18.0;
                i = j;
            } else {
                i += 1;
            }
        }
        y + 26.0
    }

    /// Moves the switches towards where they belong. True while something still moves.
    pub fn tick(&mut self, dt: f32) -> bool {
        let mut moving = false;
        for a in self.anim.iter_mut() {
            let d = a.2 - a.1;
            if d.abs() > 0.001 {
                let step = dt * 9.0;
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

    pub fn render(&mut self) -> &Pixmap {
        let s = self.scale;
        let mut pm = std::mem::replace(&mut self.pm, Pixmap::new(1, 1).unwrap());
        pm.fill(Color::TRANSPARENT);
        self.hot.clear();
        let h = self.height;
        rr(&mut pm, s, 0.0, 0.0, W, h, 22.0, [60, 60, 66], 1.0);
        rr(&mut pm, s, 1.0, 1.0, W - 2.0, h - 2.0, 21.0, BG, 1.0);

        let items = self.items();
        let (cx, cw) = (20.0, W - 40.0);
        let top = HEADER + 4.0 - self.scroll;
        let mut y = top;
        let mut i = 0;
        while i < items.len() {
            let Item::Section(title) = &items[i] else {
                i += 1;
                continue;
            };
            if title.is_empty() {
                y += 8.0;
            } else {
                self.fonts.draw(&mut pm, Weight::Semibold, &title.to_uppercase(), (cx + 8.0) * s, (y + 19.0) * s, 11.5 * s, DIM, 1.0);
                y += 30.0;
            }
            let mut j = i + 1;
            let mut ch = 0.0;
            while j < items.len() && !matches!(items[j], Item::Section(_)) {
                ch += height(&items[j]);
                j += 1;
            }
            rr(&mut pm, s, cx, y, cw, ch, 14.0, CARD, 1.0);
            let mut ry = y;
            for k in i + 1..j {
                let it = &items[k];
                let ih = height(it);
                let after_note = matches!(items[k - 1], Item::Note(_));
                if k > i + 1 && !after_note && !matches!(it, Item::Buttons(_)) {
                    rr(&mut pm, s, cx + 16.0, ry, cw - 32.0, 1.0, 0.0, LINE, 1.0);
                }
                self.draw_item(&mut pm, it, cx, ry, cw, ih);
                ry += ih;
            }
            y += ch + 18.0;
            i = j;
        }
        let total = y + self.scroll + 26.0;
        self.max_scroll = (total - h).max(0.0);

        // the title area covers what scrolls under it
        fill_rrect(&mut pm, 0.0, 0.0, W * s, HEADER * s, [22.0 * s, 22.0 * s, 0.0, 0.0], BG, 1.0);
        self.fonts.draw(&mut pm, Weight::Bold, "Settings", 28.0 * s, 44.0 * s, 24.0 * s, WHITE, 1.0);
        let sub = format!("Dynamic Island {}", crate::version::VERSION);
        self.fonts.draw(&mut pm, Weight::Regular, &sub, 28.0 * s, 64.0 * s, 12.5 * s, DIM, 1.0);
        if self.scroll > 1.0 {
            rr(&mut pm, s, 0.0, HEADER - 1.0, W, 1.0, 0.0, LINE, 1.0);
        }
        // close button
        let (bx, by, bd) = (W - 28.0 - 32.0, 24.0, 32.0);
        let hov = self.hover == Some(Id::Close);
        let down = self.down == Some(Id::Close) && hov;
        rr(&mut pm, s, bx, by, bd, bd, bd / 2.0, if down { [86, 86, 94] } else if hov { [66, 66, 72] } else { [44, 44, 48] }, 1.0);
        {
            let mut paint = Paint::default();
            paint.set_color_rgba8(235, 235, 240, 255);
            paint.anti_alias = true;
            let (mx, my, r) = ((bx + bd / 2.0) * s, (by + bd / 2.0) * s, 5.0 * s);
            let mut pb = PathBuilder::new();
            pb.move_to(mx - r, my - r);
            pb.line_to(mx + r, my + r);
            pb.move_to(mx + r, my - r);
            pb.line_to(mx - r, my + r);
            if let Some(p) = pb.finish() {
                let st = Stroke { width: 1.8 * s, line_cap: LineCap::Round, ..Default::default() };
                pm.stroke_path(&p, &paint, &st, Transform::identity(), None);
            }
        }
        self.hot.push(Hot { id: Id::Close, x: bx, y: by, w: bd, h: bd });
        self.hot.push(Hot { id: Id::Drag, x: 0.0, y: 0.0, w: W - 70.0, h: HEADER });

        // a thin scroll bar
        if self.max_scroll > 0.0 {
            let vh = h - HEADER - 12.0;
            let th = (vh * h / total).max(30.0);
            let ty = HEADER + 6.0 + (vh - th) * (self.scroll / self.max_scroll);
            rr(&mut pm, s, W - 8.0, ty, 3.0, th, 1.5, [90, 90, 98], 0.8);
        }

        // round the corners (everything that stuck out is cut away)
        if let Some(mut mask) = Mask::new(pm.width(), pm.height()) {
            if let Some(path) = crate::gfx::rrect(0.0, 0.0, W * s, h * s, [22.0 * s; 4]) {
                mask.fill_path(&path, FillRule::Winding, true, Transform::identity());
                pm.apply_mask(&mask);
            }
        }
        self.pm = pm;
        &self.pm
    }

    fn draw_item(&mut self, pm: &mut Pixmap, it: &Item, cx: f32, y: f32, cw: f32, ih: f32) {
        let s = self.scale;
        let (ix, iw) = (cx + 18.0, cw - 36.0);
        match it {
            Item::Toggle(id, label, on) => {
                let t = self.want(*id, if *on { 1.0 } else { 0.0 });
                let ease = t * t * (3.0 - 2.0 * t);
                self.fonts.draw(pm, Weight::Regular, label, ix * s, (y + ih / 2.0 + 5.2) * s, 14.5 * s, WHITE, 1.0);
                let (tw, th) = (46.0, 28.0);
                let (tx, ty) = (cx + cw - 18.0 - tw, y + (ih - th) / 2.0);
                rr(pm, s, tx, ty, tw, th, th / 2.0, mix(OFF, GREEN, ease), 1.0);
                let d = th - 6.0;
                rr(pm, s, tx + 3.0 + ease * (tw - d - 6.0), ty + 3.0, d, d, d / 2.0, WHITE, 1.0);
                self.hot.push(Hot { id: *id, x: cx, y, w: cw, h: ih });
            }
            Item::Slider(id, label, v, lo, hi, text) => {
                self.fonts.draw(pm, Weight::Regular, label, ix * s, (y + 24.0) * s, 14.5 * s, WHITE, 1.0);
                let tw = self.fonts.measure(Weight::Semibold, text, 14.0 * s);
                self.fonts.draw(pm, Weight::Semibold, text, (ix + iw) * s - tw, (y + 24.0) * s, 14.0 * s, DIM, 1.0);
                let frac = ((v - lo) / (hi - lo)).clamp(0.0, 1.0);
                let ty = y + 45.0;
                rr(pm, s, ix, ty - 2.0, iw, 4.0, 2.0, [58, 58, 64], 1.0);
                rr(pm, s, ix, ty - 2.0, iw * frac, 4.0, 2.0, BLUE, 1.0);
                let grown = self.drag == Some(*id);
                let kd = if grown { 22.0 } else { 20.0 };
                let kx = ix + iw * frac;
                rr(pm, s, kx - kd / 2.0 - 1.0, ty - kd / 2.0 - 0.5, kd + 2.0, kd + 2.0, kd / 2.0 + 1.0, [0, 0, 0], 0.35);
                rr(pm, s, kx - kd / 2.0, ty - kd / 2.0, kd, kd, kd / 2.0, WHITE, 1.0);
                self.hot.push(Hot { id: *id, x: ix - 10.0, y: y + 28.0, w: iw + 20.0, h: 34.0 });
            }
            Item::Segment(id, label, idx) => {
                self.fonts.draw(pm, Weight::Regular, label, ix * s, (y + 27.0) * s, 14.5 * s, WHITE, 1.0);
                let p = self.want(*id, *idx as f32);
                let (sx, sy, sh) = (ix, y + 40.0, 34.0);
                rr(pm, s, sx, sy, iw, sh, 10.0, [46, 46, 51], 1.0);
                let sw = iw / 3.0;
                rr(pm, s, sx + 3.0 + p * sw, sy + 3.0, sw - 6.0, sh - 6.0, 8.0, [240, 240, 244], 1.0);
                for (n, name) in ["Slim notch", "Hide", "Nothing"].iter().enumerate() {
                    let tw = self.fonts.measure(Weight::Semibold, name, 13.0 * s);
                    let near = (1.0 - (p - n as f32).abs()).clamp(0.0, 1.0);
                    let col = mix([190, 190, 198], [20, 20, 24], near);
                    self.fonts.draw(pm, Weight::Semibold, name, (sx + sw * (n as f32 + 0.5)) * s - tw / 2.0, (sy + sh / 2.0 + 4.6) * s, 13.0 * s, col, 1.0);
                }
                self.hot.push(Hot { id: *id, x: sx, y: sy, w: iw, h: sh });
            }
            Item::Buttons(list) => {
                let n = list.len() as f32;
                let gap = 10.0;
                let bw = (iw - gap * (n - 1.0)) / n;
                for (k, (id, text, style)) in list.iter().enumerate() {
                    let bx = ix + k as f32 * (bw + gap);
                    let (by, bh) = (y + 10.0, ih - 20.0);
                    let hov = self.hover == Some(*id);
                    let pressed = hov && self.down == Some(*id);
                    let base = if *style == 1 { BLUE } else { [56, 56, 62] };
                    let col = if pressed { mix(base, [0, 0, 0], 0.25) } else if hov { mix(base, WHITE, 0.14) } else { base };
                    rr(pm, s, bx, by, bw, bh, 10.0, col, 1.0);
                    let tcol = if *style == 2 { [255, 105, 97] } else { WHITE };
                    let t = self.fonts.fit(Weight::Semibold, text, 13.5 * s, (bw - 16.0) * s);
                    let tw = self.fonts.measure(Weight::Semibold, &t, 13.5 * s);
                    self.fonts.draw(pm, Weight::Semibold, &t, (bx + bw / 2.0) * s - tw / 2.0, (by + bh / 2.0 + 4.8) * s, 13.5 * s, tcol, 1.0);
                    self.hot.push(Hot { id: *id, x: bx, y: by, w: bw, h: bh });
                }
            }
            Item::Note(text) => {
                let t = self.fonts.fit(Weight::Regular, text, 12.5 * s, iw * s);
                self.fonts.draw(pm, Weight::Regular, &t, ix * s, (y + ih / 2.0 + 4.5) * s, 12.5 * s, DIM, 1.0);
            }
            Item::Section(_) => {}
        }
    }

    // ---------- mouse ----------

    fn hit_at(&self, x: f32, y: f32) -> Option<Id> {
        let (x, y) = (x / self.scale, y / self.scale);
        let header = y < HEADER;
        self.hot
            .iter()
            .find(|r| {
                let is_head = matches!(r.id, Id::Drag | Id::Close);
                is_head == header && x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h
            })
            .map(|r| r.id)
    }

    pub fn hand(&self) -> bool {
        !matches!(self.hover, None | Some(Id::Drag))
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
            Some(Id::Drag) => {
                self.down = None;
                Some(Act::DragWindow)
            }
            Some(id @ (Id::CardSecs | Id::Scale | Id::Speed)) => {
                self.drag = Some(id);
                self.set_slider(id, x)
            }
            Some(Id::Max) => {
                let r = self.hot.iter().find(|r| r.id == Id::Max)?;
                let rel = ((x / self.scale - r.x) / r.w).clamp(0.0, 0.999);
                let mode = [MaxMode::Notch, MaxMode::Hide, MaxMode::Normal][(rel * 3.0) as usize];
                if mode != self.s.maximized {
                    self.s.maximized = mode;
                    return Some(Act::Changed(self.s.clone()));
                }
                None
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
            Id::Close => return Some(Act::Close),
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
        let (ix, iw) = (r.x + 10.0, r.w - 20.0);
        let frac = ((x / self.scale - ix) / iw).clamp(0.0, 1.0);
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
