//! The island itself: its state, its springs and how it is drawn. Nothing in here knows about Windows,
//! so the whole look can be rendered (and tested) anywhere.

use crate::anim::{clamp01, lerp, smooth, Curve, Tween, EASE, H_EXPAND, SPRING, SUCK, W_EXPAND};
use crate::gfx::{self, Fonts, Weight};
use crate::lyrics::{self, Found, Line};
use crate::settings::{MaxMode, Settings};
use std::collections::VecDeque;
use std::time::Instant;
use tiny_skia::{Color, FillRule, Mask, Pixmap, PixmapPaint, Transform};

/// Logical size of the (transparent) window, and the gap between the screen edge and the island.
pub const W: f32 = 470.0;
pub const H: f32 = 230.0;
pub const TOP: f32 = 10.0;

/// the slim notch under a maximized window (a real notch is wide and low)
const NOTCH_W: f32 = 196.0;
const BUBBLE_SECS: f32 = 7.0;

#[derive(Clone, Default, Debug)]
pub struct MediaInfo {
    pub active: bool,
    pub title: String,
    pub artist: String,
    pub playing: bool,
    /// position in seconds at the moment `Ui::set_media` is called
    pub pos: f64,
    pub dur: f64,
}

/// An image with straight (non-premultiplied) RGBA pixels.
pub struct ArtData {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

/// A notification as it arrives from Windows.
pub struct NotifData {
    pub app: String,
    pub aumid: String,
    pub title: String,
    pub body: String,
    pub icon: Option<ArtData>,
}

#[derive(Clone, Copy, PartialEq)]
enum Shape {
    /// full-bleed picture: a rounded square
    Square,
    /// picture with see-through corners: a circle
    Round,
    /// a shape on nothing (white glyph): small, on a dark disc
    Glyph,
}

struct Icon {
    pm: Pixmap,
    shape: Shape,
}

/// A system change shown as the slim status pill.
#[derive(Clone, Debug, PartialEq)]
pub enum StatusData {
    /// a quick setting (see sysicons::info for the keys) switched on / off
    Sys { key: String, on: bool },
    /// battery: "Charging" / "Unplugged" / "Low Battery"
    Battery { label: String, level: i32, charging: bool, low: bool },
}

#[derive(Clone)]
struct Notif {
    app: String,
    aumid: String,
    title: String,
    body: String,
    icon: Option<std::rc::Rc<Icon>>,
    count: u32,
    status: Option<StatusData>,
}

/// One line of the notification list in the home view.
struct LogEntry {
    n: Notif,
    at: Instant,
}

/// What the window must look up (title / artist / length of the song).
pub struct LyricRequest {
    pub key: String,
    pub title: String,
    pub artist: String,
    pub dur: f64,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum LyrStatus {
    None,
    Loading,
    Missing,
    Ready,
}

/// One lyric line, already wrapped to the width of the lyrics view.
struct LyrBlock {
    t: f32,
    rows: Vec<String>,
    top: f32,
    h: f32,
}

struct Card {
    n: Notif,
    left: f32,
    /// seconds on screen in total: a card never stays longer than 25 s, whatever the mouse does
    age: f32,
    /// it grew out of the small bubble (the mouse went over it): it shrinks back into the bubble when the mouse leaves
    from_bubble: bool,
    /// seconds the mouse has been away from it
    away: f32,
}

struct Bubble {
    n: Notif,
    left: f32,
    hover: f32,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Hit {
    None,
    Pill,
    Prev,
    PlayPause,
    Next,
    /// fraction 0..1 along the progress bar
    Seek(f32),
    /// the notification card; carries the app to open ("" when unknown)
    Card(String),
    Bubble,
    /// the lyrics button / the way back from the lyrics view
    LyricsToggle,
    /// a lyric line was clicked: jump there (seconds)
    LyricLine(f32),
    /// the small pill, or the cover / title in the player: bring the music app forward
    OpenSource,
    /// a line of the notification list (home view)
    HomeRow(usize),
    /// "Clear" in the home view
    HomeClear,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    Idle,
    Play,
    Open,
    Card,
}

/// What the timer has to do next.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Need {
    /// nothing moves: stop the timer, draw nothing
    Rest,
    /// the equalizer / progress bar / a notification countdown: ~30 frames a second
    Slow,
    /// a transition: ~60 frames a second
    Fast,
}

/// What the rest of the screen is doing.
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct Env {
    /// a full-screen app (video, game, presentation)
    pub fullscreen: bool,
    /// the window in front is maximized (it would sit right under the island)
    pub maximized: bool,
    /// the cursor is at the very top edge: show the island again for a moment
    pub peek: bool,
    /// the window in front belongs to the music app that is playing (Spotify ...): its own window already shows the song
    pub player_front: bool,
}

pub struct Ui {
    scale: f32,
    /// the island, drawn on its own layer
    pm: Pixmap,
    /// what the window shows (the layer, possibly tucked away)
    out: Pixmap,
    fonts: Fonts,
    art: Option<Icon>,
    /// colour of the equalizer, taken from the cover
    accent: [u8; 3],
    media: MediaInfo,
    media_at: Instant,
    w: Tween,
    h: Tween,
    y: Tween,
    rt: Tween,
    rb: Tween,
    /// 0..1: how far the island is tucked away (movement) and how invisible it is (opacity)
    tuck: Tween,
    tuck_a: Tween,
    shift: Tween,
    bubble_k: Tween,
    /// extra space above the content while the island hangs from the top edge (it has no gap above it then)
    pad: Tween,
    /// 0..1: is there music to show in the small pill (0 = just the empty island)
    content: Tween,
    // ----- lyrics -----
    lyrics_on: bool,
    lyr_status: LyrStatus,
    lyr_key: String,
    lyr_synced: bool,
    lyr_lines: Vec<Line>,
    lyr_blocks: Vec<LyrBlock>,
    lyr_total: f32,
    lyr_layout_w: f32,
    lyr_idx: i32,
    lyr_prev: i32,
    lyr_since: f32,
    lyr_scroll: Tween,
    /// 0..1: the open player shows lyrics instead of the controls
    lyr_k: Tween,
    /// the line shown inside the small pill, and how wide the pill is for it
    line_text: String,
    line_w: f32,
    /// the notch's width for the same line (smaller text, so narrower)
    notch_line_w: f32,
    line_since: f32,
    lyr_req: Option<LyricRequest>,
    /// notifications shown lately (so the very same one is not shown again for a minute)
    recent: Vec<(String, Instant)>,
    cfg: Settings,
    /// animation speed (1.0 = normal); real time for things that must not speed up (the equalizer)
    speed: f32,
    t_real: f32,
    /// the lyrics are being scrolled by hand (mouse wheel) until this time
    lyr_manual_until: f32,
    // ----- touch feedback (all of it as in the Electron version) -----
    /// 0..1: the whole island is pressed in a little
    press_pill: Tween,
    /// prev, play/pause, next, lyrics button, way back: hover highlight / pressed-in
    btn_hover: [Tween; 5],
    btn_press: [Tween; 5],
    /// 0..1: the progress bar is thick (hovered or dragged)
    track_k: Tween,
    down: Option<Hit>,
    drag_frac: Option<f32>,
    env: Env,
    cursor: Option<(f32, f32)>,
    open: bool,
    hover_acc: f32,
    leave_acc: f32,
    card: Option<Card>,
    queue: VecDeque<Notif>,
    /// the notification list of the home view (newest first)
    log: Vec<LogEntry>,
    dnd: bool,
    sys_last: Vec<(String, bool, Instant)>,
    status_since: f32,
    /// the date and time shown in the home view (set by the window)
    clock: String,
    /// the home view was closed by a click: stays closed until the mouse has left
    open_block: bool,
    /// seconds the mouse has rested on the card (a long message then unfolds)
    card_hold: f32,
    card_lines: Vec<String>,
    gap: f32,
    bubble: Option<Bubble>,
    t: f32,
}

const BLACK: [u8; 3] = [0, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];

fn premultiplied(a: ArtData) -> Option<Pixmap> {
    let mut pm = Pixmap::new(a.w, a.h)?;
    for (d, s) in pm.data_mut().chunks_exact_mut(4).zip(a.rgba.chunks_exact(4)) {
        let al = s[3] as u32;
        d[0] = (s[0] as u32 * al / 255) as u8;
        d[1] = (s[1] as u32 * al / 255) as u8;
        d[2] = (s[2] as u32 * al / 255) as u8;
        d[3] = s[3];
    }
    Some(pm)
}

/// The cover's colour: 16x16 samples taken like a browser's canvas does (bilinear at each cell centre), weighted towards the colourful
/// samples and brightened. Same maths as the Electron version.
fn accent_of(a: &ArtData) -> [u8; 3] {
    let n = 16u32;
    let px = |x: i32, y: i32| -> [f32; 3] {
        let (x, y) = (x.clamp(0, a.w as i32 - 1) as u32, y.clamp(0, a.h as i32 - 1) as u32);
        let i = ((y * a.w + x) * 4) as usize;
        [a.rgba[i] as f32, a.rgba[i + 1] as f32, a.rgba[i + 2] as f32]
    };
    let (mut r, mut g, mut b, mut w) = (0f32, 0f32, 0f32, 0f32);
    for cy in 0..n {
        for cx in 0..n {
            let fx = (cx as f32 + 0.5) * a.w as f32 / n as f32 - 0.5;
            let fy = (cy as f32 + 0.5) * a.h as f32 / n as f32 - 0.5;
            let (x0, y0) = (fx.floor() as i32, fy.floor() as i32);
            let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
            let mut c = [0f32; 3];
            for (k, ch) in c.iter_mut().enumerate() {
                let top = px(x0, y0)[k] * (1.0 - tx) + px(x0 + 1, y0)[k] * tx;
                let bot = px(x0, y0 + 1)[k] * (1.0 - tx) + px(x0 + 1, y0 + 1)[k] * tx;
                *ch = top * (1.0 - ty) + bot * ty;
            }
            let wt = c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2]) + 8.0;
            r += c[0] * wt;
            g += c[1] * wt;
            b += c[2] * wt;
            w += wt;
        }
    }
    if w == 0.0 {
        return [255, 255, 255];
    }
    let (r, g, b) = (r / w, g / w, b / w);
    let k = 215.0 / r.max(g).max(b).max(1.0);
    [(r * k).min(255.0) as u8, (g * k).min(255.0) as u8, (b * k).min(255.0) as u8]
}

/// hue and saturation (%) of an rgb colour (HSL)
fn hue_sat(c: [u8; 3]) -> (f32, f32) {
    let (r, g, b) = (c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0);
    let (mx, mn) = (r.max(g).max(b), r.min(g).min(b));
    let (d, l) = (mx - mn, (mx + mn) / 2.0);
    let mut h = 0.0;
    if d > 0.0 {
        h = if mx == r { ((g - b) / d) % 6.0 } else if mx == g { (b - r) / d + 2.0 } else { (r - g) / d + 4.0 };
        h *= 60.0;
        if h < 0.0 {
            h += 360.0;
        }
    }
    let sat = if d > 0.0 { d / (1.0 - (2.0 * l - 1.0).abs()) } else { 0.0 };
    (h, sat * 100.0)
}

fn make_icon(a: ArtData) -> Option<Icon> {
    // classify like the Electron version: look at a 32x32 version of the picture
    const N: u32 = 32;
    let mut alpha = vec![0f32; (N * N) as usize];
    for cy in 0..N {
        for cx in 0..N {
            let (x0, x1) = (cx * a.w / N, ((cx + 1) * a.w / N).max(cx * a.w / N + 1));
            let (y0, y1) = (cy * a.h / N, ((cy + 1) * a.h / N).max(cy * a.h / N + 1));
            let (mut sum, mut c) = (0f32, 0f32);
            for y in y0..y1.min(a.h) {
                for x in x0..x1.min(a.w) {
                    sum += a.rgba[((y * a.w + x) * 4 + 3) as usize] as f32;
                    c += 1.0;
                }
            }
            alpha[(cy * N + cx) as usize] = if c > 0.0 { sum / c } else { 0.0 };
        }
    }
    let ratio = alpha.iter().filter(|&&v| v > 40.0).count() as f32 / (N * N) as f32;
    let corner = |cx: u32, cy: u32| -> f32 {
        let mut t = 0.0;
        for j in 0..3 {
            for k in 0..3 {
                t += alpha[((cy + j) * N + cx + k) as usize];
            }
        }
        t / 9.0
    };
    let clear = [corner(0, 0), corner(N - 3, 0), corner(0, N - 3), corner(N - 3, N - 3)].iter().filter(|&&v| v < 60.0).count();
    let shape = if ratio >= 0.88 {
        Shape::Square
    } else if ratio > 0.6 && clear >= 3 {
        Shape::Round
    } else {
        Shape::Glyph
    };
    Some(Icon { pm: premultiplied(a)?, shape })
}

impl Ui {
    pub fn new(scale: f32) -> Ui {
        let pw = (W * scale).ceil() as u32;
        let ph = (H * scale).ceil() as u32;
        let mut ui = Ui {
            scale,
            pm: Pixmap::new(pw, ph).expect("pixmap"),
            out: Pixmap::new(pw, ph).expect("pixmap"),
            fonts: Fonts::new(),
            art: None,
            accent: [255, 255, 255],
            media: MediaInfo::default(),
            media_at: Instant::now(),
            w: Tween::new(120.0),
            h: Tween::new(36.0),
            y: Tween::new(TOP),
            rt: Tween::new(18.0),
            rb: Tween::new(18.0),
            tuck: Tween::new(0.0),
            tuck_a: Tween::new(0.0),
            shift: Tween::new(0.0),
            bubble_k: Tween::new(0.0),
            pad: Tween::new(0.0),
            content: Tween::new(0.0),
            lyrics_on: false,
            lyr_status: LyrStatus::None,
            lyr_key: String::new(),
            lyr_synced: false,
            lyr_lines: Vec::new(),
            lyr_blocks: Vec::new(),
            lyr_total: 0.0,
            lyr_layout_w: 0.0,
            lyr_idx: -2,
            lyr_prev: -2,
            lyr_since: 0.0,
            lyr_scroll: Tween::new(0.0),
            lyr_k: Tween::new(0.0),
            line_text: String::new(),
            line_w: 240.0,
            notch_line_w: NOTCH_W,
            line_since: 0.0,
            lyr_req: None,
            recent: Vec::new(),
            cfg: Settings::default(),
            speed: 1.0,
            t_real: 0.0,
            lyr_manual_until: 0.0,
            press_pill: Tween::new(0.0),
            btn_hover: [Tween::new(0.0); 5],
            btn_press: [Tween::new(0.0); 5],
            track_k: Tween::new(0.0),
            down: None,
            drag_frac: None,
            env: Env::default(),
            cursor: None,
            open: false,
            hover_acc: 0.0,
            leave_acc: 0.0,
            card: None,
            queue: VecDeque::new(),
            log: Vec::new(),
            dnd: false,
            sys_last: Vec::new(),
            status_since: 0.0,
            clock: String::new(),
            open_block: false,
            card_hold: 0.0,
            card_lines: Vec::new(),
            gap: 0.0,
            bubble: None,
            t: 0.0,
        };
        ui.retarget();
        for s in ui.tweens_mut() {
            let t = s.target;
            s.jump(t);
        }
        ui
    }

    fn tweens_mut(&mut self) -> Vec<&mut Tween> {
        let mut v: Vec<&mut Tween> = vec![&mut self.w, &mut self.h, &mut self.y, &mut self.rt, &mut self.rb, &mut self.tuck, &mut self.tuck_a, &mut self.shift, &mut self.bubble_k, &mut self.pad, &mut self.content, &mut self.lyr_scroll, &mut self.lyr_k, &mut self.press_pill, &mut self.track_k];
        v.extend(self.btn_hover.iter_mut());
        v.extend(self.btn_press.iter_mut());
        v
    }

    fn tweens(&self) -> Vec<&Tween> {
        let mut v: Vec<&Tween> = vec![&self.w, &self.h, &self.y, &self.rt, &self.rb, &self.tuck, &self.tuck_a, &self.shift, &self.bubble_k, &self.pad, &self.content, &self.lyr_scroll, &self.lyr_k, &self.press_pill, &self.track_k];
        v.extend(self.btn_hover.iter());
        v.extend(self.btn_press.iter());
        v
    }

    /// Apply the user's settings (everything except the size, see `set_scale`).
    pub fn set_settings(&mut self, s: &Settings) {
        self.cfg = s.clone();
        self.speed = s.speed_pct as f32 / 100.0;
        if !s.lyrics_button && self.lyrics_on {
            self.lyrics_on = false;
        }
        if !s.notifications {
            self.card = None;
            self.bubble = None;
            self.queue.clear();
        }
        self.retarget();
    }

    /// A different size of the whole island: the picture is made again. Returns the new size in pixels.
    pub fn set_scale(&mut self, scale: f32) -> (u32, u32) {
        let (pw, ph) = ((W * scale).ceil() as u32, (H * scale).ceil() as u32);
        if let (Some(pm), Some(out)) = (Pixmap::new(pw, ph), Pixmap::new(pw, ph)) {
            self.scale = scale;
            self.pm = pm;
            self.out = out;
            self.lyr_layout_w = 0.0; // the lyrics are wrapped again
        }
        self.size_px()
    }

    fn fs(&self) -> bool {
        self.env.fullscreen && self.cfg.fullscreen_hide
    }

    fn mx(&self) -> bool {
        self.env.maximized && self.cfg.maximized != MaxMode::Normal
    }

    fn pf(&self) -> bool {
        self.env.player_front && self.cfg.hide_when_player_front
    }

    pub fn size_px(&self) -> (u32, u32) {
        (self.out.width(), self.out.height())
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    // ---------- input ----------

    pub fn set_media(&mut self, m: MediaInfo) {
        self.media = m;
        self.media_at = Instant::now();
        if !self.media.active {
            self.art = None;
        }
        let key = if self.media.active { format!("{}|{}", self.media.title, self.media.artist) } else { String::new() };
        if key != self.lyr_key {
            // a different song: forget the old lyrics (and look the new ones up if lyrics are switched on)
            self.lyr_key = key;
            self.lyr_status = LyrStatus::None;
            self.lyr_lines.clear();
            self.lyr_blocks.clear();
            self.lyr_idx = -2;
            self.lyr_prev = -2;
            self.line_text.clear();
            self.line_w = 240.0;
            self.notch_line_w = NOTCH_W;
            self.lyr_scroll.jump(0.0);
            self.lyr_req = None;
            if self.lyrics_on {
                self.request_lyrics();
            }
        }
        self.retarget();
    }

    // ---------- lyrics ----------

    fn request_lyrics(&mut self) {
        if self.media.active && self.lyr_status == LyrStatus::None && !self.media.title.is_empty() {
            self.lyr_status = LyrStatus::Loading;
            self.lyr_req = Some(LyricRequest { key: self.lyr_key.clone(), title: self.media.title.clone(), artist: self.media.artist.clone(), dur: self.media.dur });
        }
    }

    /// The lookup the window should start (once per song, and only while lyrics are switched on).
    pub fn take_lyrics_request(&mut self) -> Option<LyricRequest> {
        self.lyr_req.take()
    }

    pub fn toggle_lyrics(&mut self) {
        self.lyrics_on = !self.lyrics_on;
        if self.lyrics_on {
            self.request_lyrics();
        }
        self.retarget();
    }

    pub fn set_lyrics(&mut self, key: &str, found: Option<Found>) {
        if key != self.lyr_key {
            return; // an answer for a song that is no longer playing
        }
        match found {
            Some(f) if !f.lines.is_empty() => {
                self.lyr_synced = f.synced;
                self.lyr_lines = f.lines;
                self.lyr_status = LyrStatus::Ready;
                self.lyr_layout_w = 0.0; // laid out on the next tick
                self.lyr_idx = -2;
            }
            _ => self.lyr_status = LyrStatus::Missing,
        }
        self.retarget();
    }

    /// Wrap every line to the width of the lyrics view.
    fn layout_lyrics(&mut self) {
        let s = self.scale;
        let aw = self.open_w() - 52.0;
        let (px, lh) = if self.lyr_synced { (19.0, 23.75) } else { (15.0, 18.75) };
        let mut top = 0.0;
        self.lyr_blocks.clear();
        for l in &self.lyr_lines {
            let text = if l.text.is_empty() { "♪" } else { l.text.as_str() };
            let rows = wrap(&mut self.fonts, text, px * s, aw * s);
            let h = rows.len() as f32 * lh + 10.0;
            self.lyr_blocks.push(LyrBlock { t: l.t, rows, top, h });
            top += h;
        }
        self.lyr_total = top;
        self.lyr_layout_w = aw;
    }

    fn lyric_line_active(&self) -> bool {
        self.lyrics_on && self.lyr_status == LyrStatus::Ready && self.lyr_synced
    }

    fn lyr_view(&self) -> bool {
        self.lyrics_on && self.mode() == Mode::Open
    }

    fn update_lyric_line(&mut self) {
        if self.lyr_status != LyrStatus::Ready {
            return;
        }
        if (self.open_w() - 52.0 - self.lyr_layout_w).abs() > 0.5 {
            self.layout_lyrics();
        }
        if self.lyr_manual_until > 0.0 && self.t >= self.lyr_manual_until {
            self.lyr_manual_until = 0.0;
            self.lyr_idx = -2; // glide back to the current line
        }
        if !self.lyr_synced {
            return;
        }
        let idx = lyrics::index_at(&self.lyr_lines, self.position() as f32 + 0.15);
        if idx == self.lyr_idx {
            return;
        }
        self.lyr_prev = self.lyr_idx;
        self.lyr_idx = idx;
        self.lyr_since = self.t;
        // scroll: the current line sits near the top of the lyrics view, the next ones below it
        let area_h = self.open_height() - 72.0 - 12.0;
        let target = if idx >= 0 {
            let b = &self.lyr_blocks[idx as usize];
            (b.top + b.h / 2.0 - area_h / 2.0).max(0.0)
        } else {
            0.0
        };
        if self.t >= self.lyr_manual_until {
            self.lyr_scroll.go(target, self.t, 0.6, 0.0, Curve(0.3, 0.9, 0.3, 1.0));
        }
        // the pill shows this line, and is as wide as the line needs
        let text = if idx >= 0 && !self.lyr_lines[idx as usize].text.is_empty() { self.lyr_lines[idx as usize].text.clone() } else { "♪".to_string() };
        let tw = self.fonts.measure(Weight::Semibold, &text, 14.0 * self.scale) / self.scale;
        self.line_w = (tw + 108.0).clamp(220.0, 400.0);
        let tn = self.fonts.measure(Weight::Semibold, &text, 12.0 * self.scale) / self.scale;
        self.notch_line_w = (tn + 78.0).clamp(NOTCH_W, 400.0);
        self.line_text = text;
        self.line_since = self.t;
    }

    pub fn set_art(&mut self, art: Option<ArtData>) {
        self.accent = art.as_ref().map(accent_of).unwrap_or([255, 255, 255]);
        self.art = art.and_then(make_icon);
    }

    pub fn set_env(&mut self, env: Env) {
        self.env = env;
        self.retarget();
    }

    pub fn env(&self) -> Env {
        self.env
    }

    /// `Some((x, y))` in window pixels while the mouse is over the island, `None` when it left.
    /// Does the island think the mouse is over it?
    pub fn has_cursor(&self) -> bool {
        self.cursor.is_some()
    }

    pub fn set_cursor(&mut self, c: Option<(f32, f32)>) {
        self.cursor = c;
    }

    pub fn duration(&self) -> f64 {
        self.media.dur
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Is the whole island being moved / scaled right now (tucked away, pressed)? Then it is not where its rectangle says.
    pub fn is_transformed(&self) -> bool {
        self.tuck.x.abs() > 0.001 || self.tuck_a.x.abs() > 0.001 || self.press_pill.x.abs() > 0.001
    }

    pub fn is_hidden(&self) -> bool {
        self.tuck_a.x > 0.995
    }

    /// Optimistic update when a button is pressed (the real state follows from Windows a moment later).
    pub fn toggle_playing_locally(&mut self) {
        let now = self.position();
        self.media.pos = now;
        self.media_at = Instant::now();
        self.media.playing = !self.media.playing;
    }

    /// Do Not Disturb switched on / off: while it is on, notifications only go to the list.
    pub fn set_dnd(&mut self, on: bool) {
        self.dnd = on;
        if on && self.cfg.dnd_quiet {
            self.bubble = None;
            self.queue.retain(|n| n.status.is_some());
            if self.card.as_ref().map(|c| c.n.status.is_none()).unwrap_or(false) {
                self.card = None;
            }
        }
    }

    /// The date / time line of the home view.
    pub fn set_clock(&mut self, text: String) {
        self.clock = text;
    }

    pub fn has_log(&self) -> bool {
        !self.log.is_empty()
    }

    /// A row of the home list was clicked: drop it, close the view, and give back the app to open.
    pub fn home_take(&mut self, i: usize) -> String {
        let aumid = if i < self.log.len() { self.log.remove(i).n.aumid } else { String::new() };
        self.open = false;
        self.open_block = true;
        self.retarget();
        aumid
    }

    pub fn clear_log(&mut self) {
        self.log.clear();
        self.retarget();
    }

    /// How much taller the card is while a long message is unfolded.
    fn card_extra(&self) -> f32 {
        if self.card_hold > 0.35 && self.card_lines.len() > 1 {
            17.0 * (self.card_lines.len() - 1) as f32
        } else {
            0.0
        }
    }

    /// Forget which notifications were seen lately (the preview in the settings window repeats the same ones).
    pub fn forget_recent(&mut self) {
        self.recent.clear();
    }

    fn home_h(&self) -> f32 {
        let n = self.log.len().min(3);
        if n > 0 { 66.0 + 44.0 * n as f32 } else { 112.0 }
    }

    /// A system change (Wi-Fi, battery ...): the slim status pill.
    pub fn push_status(&mut self, st: StatusData) {
        if !self.cfg.status_pills {
            return;
        }
        if let StatusData::Sys { key, on } = &st {
            // the same change reported twice (our own detector and Windows' toast)
            self.sys_last.retain(|(_, _, at)| at.elapsed().as_secs() < 10);
            if self.sys_last.iter().any(|(k, o, at)| k == key && o == on && at.elapsed().as_secs_f32() < 3.0) {
                return;
            }
            self.sys_last.push((key.clone(), *on, Instant::now()));
        }
        let n = Notif { app: String::new(), aumid: String::new(), title: String::new(), body: String::new(), icon: None, count: 1, status: Some(st) };
        self.queue.retain(|q| q.status.is_none());
        match &mut self.card {
            Some(c) if c.n.status.is_some() => {
                // a pill is already up: it changes in place
                c.n = n;
                c.left = 3.5;
                c.age = 0.0;
                self.status_since = self.t;
            }
            Some(_) => self.queue.push_back(n),
            None => {
                self.bubble = None;
                self.card = Some(Card { n, left: 3.5, age: 0.0, from_bubble: false, away: 0.0 });
                self.status_since = self.t;
            }
        }
        self.retarget();
    }

    pub fn push_notification(&mut self, d: NotifData) {
        if !self.cfg.notifications {
            return;
        }
        let icon = d.icon.and_then(make_icon).map(std::rc::Rc::new);
        let mut n = Notif { app: d.app, aumid: d.aumid, title: d.title, body: d.body, icon, count: 1, status: None };
        // the very same notification again (a browser re-sends "You have unseen notifications" over and over): nothing new to show
        let same = |o: &Notif| o.app == n.app && o.title == n.title && o.body == n.body;
        if self.card.as_ref().map(|c| same(&c.n)).unwrap_or(false) || self.bubble.as_ref().map(|b| same(&b.n)).unwrap_or(false) || self.queue.back().map(same).unwrap_or(false) {
            return;
        }
        // ... and the same one that was shown a moment ago is not shown again either
        let sig = format!("{}|{}|{}", n.app, n.title, n.body);
        self.recent.retain(|(_, at)| at.elapsed().as_secs() < 60);
        if self.recent.iter().any(|(s, _)| *s == sig) {
            return;
        }
        self.recent.push((sig, Instant::now()));
        if self.recent.len() > 24 {
            self.recent.remove(0);
        }
        // the list in the home view keeps everything (also what Do Not Disturb kept quiet)
        self.log.insert(0, LogEntry { n: n.clone(), at: Instant::now() });
        self.log.truncate(8);
        if self.dnd && self.cfg.dnd_quiet {
            return;
        }
        // while music plays in the small pill: a small bubble beside it. Otherwise: a card.
        if self.mode() == Mode::Play && !self.concealed() && self.cfg.use_bubble {
            match &mut self.bubble {
                Some(b) if b.n.app == n.app => {
                    n.count = b.n.count + 1;
                    b.n = n;
                    b.left = BUBBLE_SECS;
                }
                _ => self.bubble = Some(Bubble { n, left: BUBBLE_SECS, hover: 0.0 }),
            }
        } else if let Some(c) = &mut self.card {
            self.bubble = None;
            // another one while a card is up: the same card gets the new content, no queue of cards
            let count = c.n.count + 1;
            c.n = n;
            c.n.count = count;
            c.left = self.cfg.card_secs as f32;
        } else if let Some(last) = { self.bubble = None; self.queue.back_mut() }.filter(|q| q.app == n.app) {
            let count = last.count + 1;
            *last = n;
            last.count = count;
        } else {
            self.queue.push_back(n);
        }
        self.retarget();
    }

    // ---------- state ----------

    fn concealed(&self) -> bool {
        (self.fs() || self.mx()) && !self.env.peek
    }

    /// The island hangs from the top edge (flat top, no gap) instead of floating: used under a maximized window.
    fn attached(&self) -> bool {
        self.cfg.maximized == MaxMode::Notch && self.mx() && !self.fs() && !self.env.peek
    }

    /// Is the island hidden right now, so that the top-edge "peek" is how you get it back?
    pub fn needs_peek(&self) -> bool {
        self.fs() || (self.mx() && (self.cfg.maximized == MaxMode::Hide || !(self.media.active && self.media.playing) || self.pf()) && self.card.is_none())
    }

    /// maximized window + music: the island shrinks into a slim notch on the top edge
    fn notch(&self) -> bool {
        self.cfg.maximized == MaxMode::Notch && self.mx() && !self.fs() && !self.env.peek && self.media.active && self.media.playing && !self.pf() && self.card.is_none()
    }

    pub fn mode(&self) -> Mode {
        if self.card.is_some() {
            Mode::Card
        } else if self.open {
            Mode::Open
        } else if self.media.active && self.media.playing && !self.pf() && self.cfg.show_music_pill {
            Mode::Play
        } else {
            Mode::Idle
        }
    }

    fn has_time(&self) -> bool {
        self.media.dur > 0.5
    }

    /// the open player: 400 wide, a bit smaller when it hangs from the top edge as an enlarged notch
    fn open_w(&self) -> f32 {
        // the lyrics need the full width (as in the Electron version); only the plain player is smaller when it hangs from the edge
        if self.attached() { 424.0 } else { 400.0 }
    }

    /// the player hanging from the top edge: a lower, more compact version (wide and low, like a notch)
    fn compact(&self) -> bool {
        false // (m7 squeezed the rows together to make it lower: too tight. Same spacing as the Electron player now.)
    }

    fn open_height(&self) -> f32 {
        if self.lyrics_on {
            200.0
        } else if self.compact() {
            if self.has_time() { 164.0 } else { 136.0 }
        } else if self.has_time() {
            180.0
        } else {
            150.0
        }
    }

    fn retarget(&mut self) {
        let notch = self.notch() && self.mode() == Mode::Play;
        let hang = self.attached();
        let home = !self.media.active;
        let status = self.card.as_ref().map(|c| c.n.status.is_some()).unwrap_or(false);
        let (w, h, y, rt, rb) = match self.mode() {
            Mode::Idle => (120.0, 36.0, TOP, 18.0, 18.0),
            Mode::Play if notch => (if self.lyric_line_active() && !self.line_text.is_empty() { self.notch_line_w } else { NOTCH_W }, 28.0, 0.0, 0.0, 17.0),
            Mode::Play if self.lyric_line_active() && !self.line_text.is_empty() => (self.line_w, 40.0, TOP, 20.0, 20.0),
            Mode::Play => (240.0, 40.0, TOP, 20.0, 20.0),
            // under a maximized window the big island is an enlarged notch: it stays on the top edge
            Mode::Open if home && hang => (380.0, self.home_h() + 10.0, 0.0, 0.0, 30.0),
            Mode::Open if home => (380.0, self.home_h(), TOP, 36.0, 36.0),
            Mode::Open if hang => (self.open_w(), self.open_height() + 10.0, 0.0, 0.0, 34.0),
            Mode::Open => (400.0, self.open_height(), TOP, 42.0, 42.0),
            Mode::Card if status && hang => (272.0, 48.0, 0.0, 0.0, 22.0),
            Mode::Card if status => (272.0, 44.0, TOP, 22.0, 22.0),
            Mode::Card if hang => (360.0, 74.0 + self.card_extra(), 0.0, 0.0, 30.0),
            Mode::Card => (360.0, 70.0 + self.card_extra(), TOP, 35.0, 35.0),
        };
        let pad = match (hang, self.mode()) {
            (true, Mode::Open) => 10.0,
            (true, Mode::Card) => 4.0,
            _ => 0.0,
        };
        self.pad.go(pad, self.t, 0.5, 0.0, SPRING);
        self.lyr_k.go(if self.lyr_view() { 1.0 } else { 0.0 }, self.t, 0.4, 0.0, EASE);
        let has_content = self.media.active && matches!(self.mode(), Mode::Play | Mode::Open);
        self.content.go(if has_content { 1.0 } else { 0.0 }, self.t, 0.25, 0.0, EASE);
        // growing uses the bouncy curves; shrinking is "sucked in" (ease in, out, faster)
        let now = self.t;
        let shrink = w < self.w.target - 0.5 || h < self.h.target - 0.5;
        let line_pill = self.mode() == Mode::Play && self.lyric_line_active() && !self.line_text.is_empty();
        if line_pill {
            // the pill follows the lyric: a calmer curve while it stretches / shrinks from line to line
            self.w.go(w, now, 0.55, 0.0, Curve(0.25, 1.06, 0.4, 1.0));
            self.h.go(h, now, 0.60, 0.03, H_EXPAND);
            self.rt.go(rt, now, 0.60, 0.0, SPRING);
            self.rb.go(rb, now, 0.60, 0.0, SPRING);
            self.y.go(y, now, 0.50, 0.0, SPRING);
        } else if shrink {
            self.w.go(w, now, 0.36, 0.0, SUCK);
            self.h.go(h, now, 0.40, 0.0, SUCK);
            self.rt.go(rt, now, 0.40, 0.0, SUCK);
            self.rb.go(rb, now, 0.40, 0.0, SUCK);
            self.y.go(y, now, 0.40, 0.0, SUCK);
        } else {
            self.w.go(w, now, 0.50, 0.0, W_EXPAND);
            self.h.go(h, now, 0.60, 0.03, H_EXPAND);
            self.rt.go(rt, now, 0.60, 0.0, SPRING);
            self.rb.go(rb, now, 0.60, 0.0, SPRING);
            self.y.go(y, now, 0.50, 0.0, SPRING);
        }
        // tucked away: only the resting island hides; cards and the open player stay
        let hide = matches!(self.mode(), Mode::Idle | Mode::Play)
            && self.concealed()
            && !(self.cfg.maximized == MaxMode::Notch && self.mx() && !self.fs() && self.media.active && self.media.playing && !self.pf() && self.cfg.show_music_pill);
        if hide {
            self.tuck.go(1.0, now, 0.45, 0.0, Curve(0.5, 0.0, 0.8, 0.3));
            self.tuck_a.go(1.0, now, 0.28, 0.12, EASE);
        } else {
            self.tuck.go(0.0, now, 0.45, 0.0, SPRING);
            self.tuck_a.go(0.0, now, 0.22, 0.0, EASE);
        }
        let split = self.bubble.is_some() && matches!(self.mode(), Mode::Play | Mode::Open);
        self.shift.go(if split { -24.0 } else { 0.0 }, now, 0.85, 0.0, SPRING);
        if split {
            self.bubble_k.go(1.0, now, 1.05, 0.0, Curve(0.34, 1.5, 0.5, 1.0));
        } else {
            self.bubble_k.go(0.0, now, 0.58, 0.0, Curve(0.5, 0.0, 0.9, 0.4));
        }
    }

    pub fn position(&self) -> f64 {
        let mut p = self.media.pos;
        if self.media.playing {
            p += self.media_at.elapsed().as_secs_f64();
        }
        if self.media.dur > 0.0 {
            p = p.min(self.media.dur);
        }
        p.max(0.0)
    }

    // ---------- geometry (logical px) ----------

    fn pill_rect(&self) -> (f32, f32, f32, f32) {
        let (w, h) = (self.w.x, self.h.x);
        (W / 2.0 + self.shift.x - w / 2.0, self.y.x, w, h)
    }

    /// The bubble: a circle beside the pill, or a small tab hanging from the top edge beside the notch.
    fn bubble_rect(&self) -> (f32, f32, f32, f32) {
        let (px, py, pw, _) = self.pill_rect();
        let notch = self.y.x < 4.0;
        let (bw, bh) = if notch { (38.0, 28.0) } else { (40.0, 40.0) };
        (px + pw + 8.0, if notch { 0.0 } else { py }, bw, bh)
    }

    fn inside(r: (f32, f32, f32, f32), x: f32, y: f32, m: f32) -> bool {
        x >= r.0 - m && x <= r.0 + r.2 + m && y >= r.1 - m && y <= r.1 + r.3 + m
    }

    /// The island's rectangle in window pixels (x, y, w, h), including the bubble.
    pub fn rect_px(&self) -> (f32, f32, f32, f32) {
        let p = self.pill_rect();
        let (mut x1, mut y1, mut x2, mut y2) = (p.0, p.1, p.0 + p.2, p.1 + p.3);
        if self.bubble.is_some() {
            let b = self.bubble_rect();
            x2 = x2.max(b.0 + b.2);
            y2 = y2.max(b.1 + b.3);
            y1 = y1.min(b.1);
            x1 = x1.min(b.0);
        }
        let s = self.scale;
        (x1 * s, y1 * s, (x2 - x1) * s, (y2 - y1) * s)
    }

    /// Advance the animation by `dt` seconds.
    pub fn tick(&mut self, dt: f32) -> Need {
        self.t += dt * self.speed;
        self.t_real += dt;
        let (over_pill, over_bubble) = match self.cursor {
            Some((cx, cy)) if !self.is_hidden() => {
                let (x, y) = (cx / self.scale, cy / self.scale);
                (Self::inside(self.pill_rect(), x, y, 0.0), self.bubble.is_some() && Self::inside(self.bubble_rect(), x, y, 0.0))
            }
            _ => (false, false),
        };

        // notification countdowns
        let music = self.media.active && self.media.playing && !self.pf() && self.cfg.show_music_pill && self.cfg.use_bubble;
        if let Some(c) = &mut self.card {
            if over_pill {
                c.left = c.left.max(1.4); // reading it: it stays
            } else {
                c.left -= dt;
            }
            c.age += dt;
            c.away = if over_pill { 0.0 } else { c.away + dt };
            if c.from_bubble && c.away > 0.8 && music && c.age < 25.0 {
                // the mouse left: the card shrinks back into the small bubble beside the music pill
                let c = self.card.take().unwrap();
                self.bubble = Some(Bubble { n: c.n, left: 3.5, hover: 0.0 });
            } else if c.left <= 0.0 || c.age > 25.0 {
                self.card = None;
                self.gap = 0.6;
            }
        } else if self.gap > 0.0 {
            self.gap -= dt;
        } else if let Some(n) = self.queue.pop_front() {
            let secs = if n.status.is_some() { 3.5 } else { self.cfg.card_secs as f32 };
            if n.status.is_some() {
                self.status_since = self.t;
            }
            self.card = Some(Card { n, left: secs, age: 0.0, from_bubble: false, away: 0.0 });
        }
        if let Some(b) = &mut self.bubble {
            b.left -= dt;
            b.hover = if over_bubble { b.hover + dt } else { 0.0 };
            if b.left <= 0.0 {
                self.bubble = None;
            } else if b.hover > 0.15 {
                // hover the small bubble: it grows into the card
                let b = self.bubble.take().unwrap();
                self.card = Some(Card { n: b.n, left: self.cfg.card_secs as f32 + 2.0, age: 0.0, from_bubble: true, away: 0.0 });
            }
        }

        // a long message unfolds while the mouse rests on its card
        let readable = self.card.as_ref().map(|c| c.n.status.is_none()).unwrap_or(false);
        self.card_hold = if readable && over_pill { self.card_hold + dt } else { 0.0 };
        if readable && self.card_hold > 0.35 && self.card_lines.is_empty() {
            let body = self.card.as_ref().map(|c| c.n.body.clone()).unwrap_or_default();
            let lines = wrap(&mut self.fonts, &body, 13.0 * self.scale, (360.0 - 71.0 - 22.0) * self.scale);
            if lines.len() > 1 {
                self.card_lines = lines.into_iter().take(5).collect();
            }
        }
        if !self.card_lines.is_empty() && (!readable || (self.card_hold == 0.0 && self.h.x < 74.0)) {
            self.card_lines.clear();
        }
        // hovering opens the player after a short moment; leaving closes it after a longer one (a flick of the mouse does nothing)
        if !over_pill {
            self.open_block = false;
        }
        if over_pill && self.card.is_none() {
            self.hover_acc += dt;
            self.leave_acc = 0.0;
            if self.hover_acc >= 0.06 && !self.open_block {
                self.open = true;
            }
        } else {
            self.leave_acc += dt;
            self.hover_acc = 0.0;
            if self.leave_acc >= 0.30 {
                self.open = false;
            }
        }
        self.update_lyric_line();
        self.retarget();
        let now = self.t;
        // touch feedback: hover highlight and press on the buttons, a thick progress bar
        let cur = self.cursor;
        let hov = cur.map(|(cx, cy)| self.hit(cx, cy));
        let hov_btn = hov.as_ref().and_then(|h| self.btn_index(h));
        let on_track = self.drag_frac.is_some() || cur.map(|(cx, cy)| self.on_track(cx, cy)).unwrap_or(false);
        let down_btn = self.down.as_ref().and_then(|h| self.btn_index(h));
        for i in 0..5 {
            self.btn_hover[i].go(if hov_btn == Some(i) { 1.0 } else { 0.0 }, now, 0.15, 0.0, EASE);
            self.btn_press[i].go(if down_btn == Some(i) { 1.0 } else { 0.0 }, now, 0.12, 0.0, EASE);
        }
        self.track_k.go(if on_track { 1.0 } else { 0.0 }, now, 0.18, 0.0, Curve(0.22, 1.0, 0.36, 1.0));
        for s in self.tweens_mut() {
            s.step(now);
        }
        self.need()
    }

    pub fn need(&self) -> Need {
        let springs = !self.tweens().iter().all(|s| s.settled());
        let over = self.cursor.is_some();
        let delay = (over && !self.open && !self.open_block) || (!over && self.open);
        if springs || delay {
            return Need::Fast;
        }
        if self.card.is_some() || self.bubble.is_some() || !self.queue.is_empty() || self.gap > 0.0 {
            return Need::Slow;
        }
        if self.media.active && matches!(self.mode(), Mode::Play | Mode::Open) && (self.media.playing || self.open) && !self.is_hidden() {
            return Need::Slow;
        }
        Need::Rest
    }

    // ---------- the open player ----------

    /// Positions (relative to the island's top-left), copied from the Electron version's CSS: the times sit left and right of the bar,
    /// the buttons are big and solid white.
    fn open_layout(&self) -> OpenLayout {
        let ow = self.open_w();
        let time = self.has_time();
        let c = self.compact();
        let yc = match (time, c) {
            (true, false) => 145.0,
            (true, true) => 132.0,
            (false, false) => 118.0,
            (false, true) => 108.0,
        };
        OpenLayout {
            ow,
            // track: x, y (centre line), width (the two times take 34 px + 10 px gap on each side)
            track: if time { Some((26.0 + 34.0 + 10.0, if c { 98.0 } else { 106.0 }, ow - 52.0 - 88.0)) } else { None },
            prev: (ow / 2.0 - 88.0, yc),
            play: (ow / 2.0, yc),
            next: (ow / 2.0 + 88.0, yc),
            lyr_btn: (ow - 38.0, yc),
        }
    }

    pub fn hit(&self, px: f32, py: f32) -> Hit {
        if self.tuck_a.x > 0.5 {
            return Hit::None;
        }
        let (x, y) = (px / self.scale, py / self.scale);
        if self.bubble.is_some() && Self::inside(self.bubble_rect(), x, y, 0.0) {
            return Hit::Bubble;
        }
        let r = self.pill_rect();
        if !Self::inside(r, x, y, 0.0) {
            return Hit::None;
        }
        if self.mode() == Mode::Card {
            let app = self.card.as_ref().map(|c| c.n.aumid.clone()).unwrap_or_default();
            return Hit::Card(app);
        }
        if self.mode() == Mode::Play {
            return Hit::OpenSource;
        }
        if self.mode() == Mode::Open && !self.media.active {
            if self.w.x > 340.0 {
                let (rx, ry) = (x - r.0, y - r.1 - self.pad.x);
                if ry < 46.0 && rx > r.2 - 100.0 && !self.log.is_empty() {
                    return Hit::HomeClear;
                }
                if ry >= 50.0 {
                    let i = ((ry - 50.0) / 44.0) as usize;
                    if i < self.log.len().min(3) {
                        return Hit::HomeRow(i);
                    }
                }
            }
            return Hit::Pill;
        }
        if self.mode() == Mode::Open && self.w.x > 340.0 {
            let l = self.open_layout();
            let (rx, ry) = (x - r.0, y - r.1 - self.pad.x);
            if rx >= 20.0 && rx <= l.ow - 70.0 && ry >= 14.0 && ry <= 72.0 {
                return Hit::OpenSource; // the cover and the title
            }
            let near = |c: (f32, f32), rr: f32| (rx - c.0).abs() <= rr && (ry - c.1).abs() <= rr;
            if self.lyr_k.x > 0.5 {
                // the lyrics view: the way back, or a line to jump to
                if near((l.ow - 37.0, 39.0), 22.0) {
                    return Hit::LyricsToggle;
                }
                if self.lyr_synced && self.lyr_status == LyrStatus::Ready {
                    let (top, bottom) = (72.0, self.open_height() - 12.0);
                    if ry >= top && ry <= bottom {
                        for b in &self.lyr_blocks {
                            let by = top + b.top - self.lyr_scroll.x;
                            if ry >= by && ry < by + b.h {
                                return Hit::LyricLine(b.t);
                            }
                        }
                    }
                }
                return Hit::Pill;
            }
            if self.cfg.lyrics_button && near(l.lyr_btn, 24.0) {
                return Hit::LyricsToggle;
            }
            if near(l.play, 28.0) {
                return Hit::PlayPause;
            }
            if near(l.prev, 26.0) {
                return Hit::Prev;
            }
            if near(l.next, 26.0) {
                return Hit::Next;
            }
            if let Some((tx, ty, tw)) = l.track {
                if (ry - ty).abs() <= 11.0 && rx >= tx - 6.0 && rx <= tx + tw + 6.0 {
                    return Hit::Seek(clamp01((rx - tx) / tw));
                }
            }
        }
        Hit::Pill
    }

    pub fn has_bubble(&self) -> bool {
        self.bubble.is_some()
    }

    pub fn bubble_aumid(&self) -> String {
        self.bubble.as_ref().map(|b| b.n.aumid.clone()).unwrap_or_default()
    }

    fn btn_index(&self, h: &Hit) -> Option<usize> {
        match h {
            Hit::Prev => Some(0),
            Hit::PlayPause => Some(1),
            Hit::Next => Some(2),
            Hit::LyricsToggle => Some(if self.lyr_k.x > 0.5 { 4 } else { 3 }),
            _ => None,
        }
    }

    fn on_track(&self, px: f32, py: f32) -> bool {
        if self.mode() != Mode::Open || self.w.x < 340.0 || self.lyr_k.x > 0.5 {
            return false;
        }
        let l = self.open_layout();
        let Some((tx, ty, tw)) = l.track else { return false };
        let r = self.pill_rect();
        let (rx, ry) = (px / self.scale - r.0, py / self.scale - r.1 - self.pad.x);
        (ry - ty).abs() <= 11.0 && rx >= tx - 6.0 && rx <= tx + tw + 6.0
    }

    fn frac_at(&self, px: f32) -> f32 {
        let l = self.open_layout();
        let r = self.pill_rect();
        match l.track {
            Some((tx, _, tw)) => clamp01((px / self.scale - r.0 - tx) / tw),
            None => 0.0,
        }
    }

    /// The mouse went down: buttons press in, the island itself gives a little, the progress bar can be dragged.
    pub fn mouse_down(&mut self, px: f32, py: f32) -> Hit {
        let h = self.hit(px, py);
        self.down = Some(h.clone());
        match &h {
            Hit::Seek(f) => self.drag_frac = Some(*f),
            Hit::Pill | Hit::Card(_) | Hit::Bubble | Hit::OpenSource => self.press_pill.go(1.0, self.t, 0.12, 0.0, EASE),
            _ => {}
        }
        h
    }

    pub fn mouse_move(&mut self, px: f32) {
        if self.drag_frac.is_some() {
            self.drag_frac = Some(self.frac_at(px));
        }
    }

    pub fn is_dragging(&self) -> bool {
        self.drag_frac.is_some()
    }

    /// The mouse went up: the click happens if it is still on what it went down on (a drag of the bar seeks to where it ends).
    pub fn mouse_up(&mut self, px: f32, py: f32) -> Hit {
        self.press_pill.go(0.0, self.t, 0.35, 0.0, SPRING);
        let down = self.down.take();
        if let Some(f) = self.drag_frac.take() {
            return Hit::Seek(f);
        }
        let up = self.hit(px, py);
        match down {
            Some(d) if d == up => up,
            _ => Hit::None,
        }
    }

    /// Should the mouse cursor be a hand here?
    pub fn wants_hand(&self, px: f32, py: f32) -> bool {
        matches!(self.hit(px, py), Hit::Prev | Hit::PlayPause | Hit::Next | Hit::LyricsToggle | Hit::Seek(_) | Hit::Card(_) | Hit::Bubble | Hit::OpenSource | Hit::LyricLine(_) | Hit::HomeRow(_) | Hit::HomeClear)
    }

    /// Mouse wheel over the lyrics: scroll by hand (the lyrics stop following the song for a few seconds).
    pub fn wheel(&mut self, notches: f32) {
        if !self.lyr_view() || self.lyr_status != LyrStatus::Ready || !self.lyr_synced {
            return;
        }
        let area_h = self.open_height() - 84.0;
        let max = (self.lyr_total - area_h).max(0.0);
        let v = (self.lyr_scroll.x - notches * 34.0).clamp(0.0, max);
        self.lyr_scroll.jump(v);
        self.lyr_manual_until = self.t + 3.0;
    }

    /// Dismiss the card (after a click).
    pub fn dismiss_card(&mut self) {
        self.card = None;
        self.gap = 0.3;
        self.retarget();
    }

    /// Opened the source app from the bubble / card.
    pub fn dismiss_bubble(&mut self) {
        self.bubble = None;
        self.retarget();
    }

    // ---------- drawing ----------

    pub fn render(&mut self) -> &Pixmap {
        self.out.fill(Color::TRANSPARENT);
        if self.is_hidden() {
            return &self.out;
        }
        self.pm.fill(Color::TRANSPARENT);
        self.draw_island();
        // like `overflow: hidden` on the island: nothing may show outside its (moving) shape
        if let Some(mask) = self.island_mask() {
            self.pm.apply_mask(&mask);
        }
        self.draw_ears();
        let s = self.scale;
        let k = self.tuck.x;
        let paint = PixmapPaint { opacity: clamp01(1.0 - self.tuck_a.x), ..PixmapPaint::default() };
        // tucked away = it slides up and shrinks a little towards the top edge
        let sc = 1.0 - 0.18 * k;
        let (cx, cy) = (W / 2.0 * s, TOP * s);
        let t = Transform::from_row(sc, 0.0, 0.0, sc, cx - cx * sc, cy - cy * sc - 64.0 * s * k);
        let pr = 1.0 - 0.05 * self.press_pill.x;
        let (px, py, _, ph) = self.pill_rect();
        let (pcx, pcy) = ((px + self.w.x / 2.0) * s, (py + ph / 2.0) * s);
        let t = t.pre_concat(Transform::from_row(pr, 0.0, 0.0, pr, pcx - pcx * pr, pcy - pcy * pr));
        if sc == 1.0 && pr == 1.0 && paint.opacity >= 0.999 {
            // nothing moves the whole island (the usual case): the picture is simply copied
            self.out.data_mut().copy_from_slice(self.pm.data());
        } else {
            self.out.draw_pixmap(0, 0, self.pm.as_ref(), &paint, t, None);
        }
        &self.out
    }

    /// The island's shape (and the bubble's, with a margin for its count badge): what is allowed to show.
    fn island_mask(&self) -> Option<Mask> {
        let s = self.scale;
        let mut mask = Mask::new(self.pm.width(), self.pm.height())?;
        let (x0, y0, w, h) = self.pill_rect();
        let (rt, rb) = (self.rt.x, self.rb.x);
        let p = gfx::rrect(x0 * s, y0 * s, w * s, h * s, [rt * s, rt * s, rb * s, rb * s])?;
        mask.fill_path(&p, FillRule::Winding, true, Transform::identity());
        if self.bubble.is_some() && self.bubble_k.x > 0.02 {
            let (bx, by, bw, bh) = self.bubble_rect();
            let m = 10.0;
            if let Some(b) = gfx::rrect((bx - m) * s, (by - m).max(0.0) * s, (bw + 2.0 * m) * s, (bh + 2.0 * m) * s, [0.0; 4]) {
                mask.fill_path(&b, FillRule::Winding, true, Transform::identity());
            }
        }
        Some(mask)
    }

    /// A notch is not a box hanging from the edge: at its top corners the black flares outwards in a concave curve and melts into the
    /// screen's edge. Only when the island really touches the top (flat top corners).
    fn draw_ears(&mut self) {
        let hang = (1.0 - clamp01(self.y.x / 5.0)) * (1.0 - clamp01(self.rt.x / 5.0));
        if hang <= 0.02 || self.tuck_a.x > 0.5 {
            return;
        }
        let (x0, _, w, h) = self.pill_rect();
        let r = (if h > 40.0 { 13.0 } else { 8.0 }) * hang;
        let s = self.scale;
        const K: f32 = 0.552_284_75;
        let mut pb = tiny_skia::PathBuilder::new();
        // left ear: the corner between the top edge and the island's left wall
        pb.move_to((x0 - r) * s, 0.0);
        pb.line_to(x0 * s + 0.5, 0.0);
        pb.line_to(x0 * s + 0.5, r * s);
        pb.cubic_to(x0 * s, (r - K * r) * s, (x0 - r + K * r) * s, 0.0, (x0 - r) * s, 0.0);
        pb.close();
        // right ear: the mirror image
        let x1 = x0 + w;
        pb.move_to((x1 + r) * s, 0.0);
        pb.line_to(x1 * s - 0.5, 0.0);
        pb.line_to(x1 * s - 0.5, r * s);
        pb.cubic_to(x1 * s, (r - K * r) * s, (x1 + r - K * r) * s, 0.0, (x1 + r) * s, 0.0);
        pb.close();
        if let Some(path) = pb.finish() {
            let mut p = tiny_skia::Paint::default();
            p.set_color_rgba8(0, 0, 0, 255);
            p.anti_alias = true;
            self.pm.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None);
        }
    }

    fn draw_island(&mut self) {
        let s = self.scale;
        let (x0, ytop, w, h_full) = self.pill_rect();
        let (rt, rb) = (self.rt.x, self.rb.x);
        gfx::fill_rrect(&mut self.pm, x0 * s, ytop * s, w * s, h_full * s, [rt * s, rt * s, rb * s, rb * s], BLACK, 1.0);
        // the content starts below `pad` when the island hangs from the top edge
        let (y0, h) = (ytop + self.pad.x, h_full - self.pad.x);

        // the small notification bubble
        if self.bubble.is_some() && self.bubble_k.x > 0.02 {
            self.draw_bubble();
        }

        if let Some(StatusData::Sys { .. } | StatusData::Battery { .. }) = self.card.as_ref().and_then(|c| c.n.status.clone()) {
            let st = self.card.as_ref().and_then(|c| c.n.status.clone()).unwrap();
            let a = smooth((w - 200.0) / 60.0);
            self.draw_status(x0, y0, w, h, a, &st);
            return;
        }
        if !self.media.active && self.mode() == Mode::Open {
            let a = smooth((w - 330.0) / 40.0) * smooth((h - 60.0) / 36.0);
            self.draw_home(x0, y0, w, h, a);
            return;
        }
        if let Some(card) = self.card.as_ref() {
            let a = smooth((w - 250.0) / 100.0);
            // (borrowed fields are copied out first)
            let (name, title, body, icon, count) = (if card.n.app.is_empty() { card.n.title.clone() } else { card.n.app.clone() }, card.n.title.clone(), card.n.body.clone(), card.n.icon.clone(), card.n.count);
            self.draw_card(x0, y0, w, h, a, &name, &title, &body, icon.as_deref(), count);
            return;
        }

        if self.media.active && self.content.x > 0.01 {
            // how open the player is comes from the HEIGHT: a wide pill that shows a lyric line is still the small pill
            let p_open = smooth((h - 48.0) / 70.0);
            let show = smooth((w - 112.0) / 24.0) * self.content.x;
            // the album art is ONE element that grows from the small pill into the open player (like the iPhone)
            let ts = clamp01((h - 28.0) / 12.0); // notch -> small pill (by height: a wide notch showing a lyric is still a notch)
            let k = self.lyr_k.x;
            let (ax, ay, asz, ar) = (
                lerp(lerp(12.0, 14.0, ts), 26.0, p_open),
                lerp(lerp(5.0, 6.0, ts), lerp(20.0, 18.0, k), p_open),
                lerp(lerp(16.0, 28.0, ts), lerp(64.0, 42.0, k), p_open),
                lerp(lerp(5.0, 9.0, ts), lerp(16.0, 11.0, k), p_open),
            );
            match &self.art {
                Some(art) => gfx::draw_art(&mut self.pm, &art.pm, (x0 + ax) * s, (y0 + ay) * s, asz * s, ar * s, show),
                None => gfx::fill_rrect(&mut self.pm, (x0 + ax) * s, (y0 + ay) * s, asz * s, asz * s, [ar * s; 4], [44, 44, 46], show),
            }
            let playing = self.media.playing;
            // equalizer: tiny in the notch, small in the pill, large in the player
            let a_mini = show * clamp01(1.0 - p_open * 1.6);
            let t = clamp01((h - 28.0) / 12.0);
            let (eh, ebw, egap, eright) = (lerp(12.0, 18.0, t), lerp(2.2, 3.0, t), lerp(2.0, 2.5, t), lerp(13.0, 14.0, t));
            let ewidth = 4.0 * ebw + 3.0 * egap;
            self.eq(x0 + w - eright - ewidth, y0 + h / 2.0 + eh / 2.0, ebw, egap, eh, 4, a_mini, playing);
            let a_wide = smooth((p_open - 0.5) / 0.5) * clamp01((0.5 - self.lyr_k.x) / 0.5);
            self.eq(x0 + w - 26.0 - 39.0, y0 + 24.0 + 26.0, 3.0, 3.0, 26.0, 7, a_wide, playing);

            // lyrics on: the pill shows the line that is being sung
            if self.lyric_line_active() && !self.line_text.is_empty() && self.mode() == Mode::Play {
                let t = smooth((self.t - self.line_since) / 0.45);
                // in the slim notch the line is a little smaller and closer to the cover / equalizer
                let sz = clamp01((h - 28.0) / 12.0);
                let (px, l, r, base) = (lerp(12.0, 14.0, sz), lerp(36.0, 54.0, sz), lerp(34.0, 46.0, sz), lerp(4.3, 5.0, sz));
                let text = self.fonts.fit(Weight::Semibold, &self.line_text, px * s, (w - l - r) * s);
                let tw = self.fonts.measure(Weight::Semibold, &text, px * s);
                let cx = (x0 + l + x0 + w - r) / 2.0 * s;
                self.fonts.draw(&mut self.pm, Weight::Semibold, &text, cx - tw / 2.0, (y0 + h / 2.0 + base + (1.0 - t) * 9.0 * (px / 14.0)) * s, px * s, WHITE, show * t * (1.0 - p_open));
            }

            let a_open = smooth((p_open - 0.5) / 0.5);
            if a_open > 0.01 {
                self.draw_open(x0, y0, w, h, a_open);
            }
        }
    }

    /// A logo in a badge of `size`: a circle (round / glyph), or a rounded square (`sq_radius`). No logo: the first letter on a coloured disc.
    fn draw_icon(&mut self, icon: Option<&Icon>, name: &str, x: f32, y: f32, size: f32, sq_radius: f32, alpha: f32) {
        let s = self.scale;
        match icon {
            Some(i) if i.shape == Shape::Glyph => {
                gfx::fill_rrect(&mut self.pm, x * s, y * s, size * s, size * s, [size / 2.0 * s; 4], [38, 38, 42], alpha);
                let g = size * 0.62;
                gfx::draw_art(&mut self.pm, &i.pm, (x + (size - g) / 2.0) * s, (y + (size - g) / 2.0) * s, g * s, 0.0, alpha);
            }
            Some(i) if i.shape == Shape::Round => gfx::draw_art(&mut self.pm, &i.pm, x * s, y * s, size * s, size / 2.0 * s, alpha),
            Some(i) => gfx::draw_art(&mut self.pm, &i.pm, x * s, y * s, size * s, sq_radius * s, alpha),
            None => {
                let mut hue = 0u32;
                for ch in name.chars() {
                    hue = (hue * 31 + ch as u32) % 360;
                }
                let h = hue as f32;
                gfx::fill_gradient_circle(&mut self.pm, x * s, y * s, size * s, gfx::hsl(h, 80.0, 62.0), gfx::hsl(h + 28.0, 75.0, 45.0), alpha);
                if let Some(c) = name.chars().next() {
                    let letter = c.to_uppercase().to_string();
                    let px = size * 0.476 * s; // 20px in a 42px badge
                    let w = self.fonts.measure(Weight::Bold, &letter, px);
                    self.fonts.draw(&mut self.pm, Weight::Bold, &letter, (x + size / 2.0) * s - w / 2.0, (y + size / 2.0) * s + px * 0.36, px, WHITE, alpha);
                }
            }
        }
    }

    fn draw_bubble(&mut self) {
        let s = self.scale;
        let k = self.bubble_k.x;
        let (bx, by, bw, bh) = self.bubble_rect();
        let notch = self.y.x < 4.0;
        // it pinches off the pill: starts hidden inside it and slides out
        let (px, _, pw, _) = self.pill_rect();
        let from = px + pw - bw;
        let x = lerp(from, bx, clamp01(k));
        let r = if notch { [0.0, 0.0, bw / 2.0 * s, bw / 2.0 * s] } else { [bw / 2.0 * s; 4] };
        let scale_k = clamp01(k * 1.4);
        let (cw, ch) = (bw * scale_k, bh * scale_k);
        gfx::fill_rrect(&mut self.pm, (x + (bw - cw) / 2.0) * s, by * s + (if notch { 0.0 } else { (bh - ch) / 2.0 * s }), cw * s, ch * s, if notch { [0.0, 0.0, cw / 2.0 * s, cw / 2.0 * s] } else { [cw / 2.0 * s; 4] }, BLACK, 1.0);
        let _ = r;
        let a = smooth((k - 0.55) / 0.45);
        if a > 0.01 {
            let (icon, count) = match &self.bubble {
                Some(b) => (b.n.icon.clone(), b.n.count),
                None => (None, 0),
            };
            let isz = if notch { 18.0 } else { 30.0 };
            let (ix, iy) = (x + (bw - isz) / 2.0, by + (bh - isz) / 2.0 - if notch { 1.0 } else { 0.0 });
            let nm = self.bubble.as_ref().map(|b| b.n.app.clone()).unwrap_or_default();
            let sq = if notch { 5.0 } else { 9.0 };
            self.draw_icon(icon.as_deref(), &nm, ix, iy, isz, sq, a);
            if count > 1 {
                let label = if count > 9 { "9+".to_string() } else { count.to_string() };
                let (rx, ry) = (ix + isz - 8.0, iy - 6.0);
                gfx::fill_rrect(&mut self.pm, rx * s, ry * s, 17.0 * s, 17.0 * s, [8.5 * s; 4], [255, 69, 58], a);
                let tw = self.fonts.measure(Weight::Semibold, &label, 11.0 * s);
                self.fonts.draw(&mut self.pm, Weight::Semibold, &label, (rx + 8.5) * s - tw / 2.0, (ry + 12.5) * s, 11.0 * s, WHITE, a);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_card(&mut self, x0: f32, y0: f32, w: f32, h: f32, a: f32, name: &str, title: &str, body: &str, icon: Option<&Icon>, count: u32) {
        if a <= 0.01 {
            return;
        }
        let s = self.scale;
        self.draw_icon(icon, name, x0 + 16.0, y0 + 14.0, 42.0, 11.0, a);
        let tx = x0 + 71.0;
        let label = if count > 1 { format!("+{}", count - 1) } else { "now".to_string() };
        let lw = self.fonts.measure(Weight::Regular, &label, 12.0 * s);
        // the text column ends where the "now" label begins (the two sit side by side, 13px apart)
        let max_w = (x0 + w - 20.0 - 13.0) - tx - lw / s;
        let title = self.fonts.fit(Weight::Semibold, title, 15.0 * s, max_w * s);
        let body = self.fonts.fit(Weight::Regular, body, 13.0 * s, max_w * s);
        self.fonts.draw(&mut self.pm, Weight::Semibold, &title, tx * s, (y0 + 32.0) * s, 15.0 * s, WHITE, a);
        if self.card_lines.len() > 1 && h > 78.0 {
            let lines = self.card_lines.clone();
            for (i, l) in lines.iter().enumerate() {
                self.fonts.draw(&mut self.pm, Weight::Regular, l, tx * s, (y0 + 51.0 + 17.0 * i as f32) * s, 13.0 * s, [142, 142, 147], a);
            }
        } else {
            self.fonts.draw(&mut self.pm, Weight::Regular, &body, tx * s, (y0 + 51.0) * s, 13.0 * s, [142, 142, 147], a);
        }
        self.fonts.draw(&mut self.pm, Weight::Regular, &label, (x0 + w - 20.0) * s - lw, (y0 + 27.0) * s, 12.0 * s, [99, 99, 102], a);
    }

    /// The slim status pill: icon and name on the left, On / Off (or the battery level) on the right.
    fn draw_status(&mut self, x0: f32, y0: f32, w: f32, h: f32, a: f32, st: &StatusData) {
        let s = self.scale;
        let cy = y0 + h / 2.0;
        let pop = clamp01((self.t - self.status_since) / 0.45);
        const GRAY: [u8; 3] = [142, 142, 147];
        match st {
            StatusData::Sys { key, on } => {
                let (label, color) = crate::sysicons::info(key);
                let col = if *on { color } else { GRAY };
                crate::sysicons::draw(&mut self.pm, key, (x0 + 31.0) * s, cy * s, 22.0 * s, col, a, *on, pop);
                self.fonts.draw(&mut self.pm, Weight::Semibold, label, (x0 + 52.0) * s, (cy + 5.4) * s, 15.0 * s, WHITE, a);
                let t = if *on { "On" } else { "Off" };
                let tw = self.fonts.measure(Weight::Semibold, t, 14.5 * s);
                self.fonts.draw(&mut self.pm, Weight::Semibold, t, (x0 + w - 22.0) * s - tw, (cy + 5.2) * s, 14.5 * s, if *on { color } else { GRAY }, a);
            }
            StatusData::Battery { label, level, charging, low } => {
                let green = [48, 209, 88];
                let red = [255, 69, 58];
                let mut lx = x0 + 22.0;
                if *charging && !*low {
                    crate::sysicons::draw(&mut self.pm, "bolt", (lx + 8.0) * s, cy * s, 18.0 * s, green, a, true, pop);
                    lx += 22.0;
                }
                let fill = if *low { red } else if *charging { green } else { WHITE };
                self.fonts.draw(&mut self.pm, Weight::Semibold, label, lx * s, (cy + 5.4) * s, 15.0 * s, if *low { red } else { WHITE }, a);
                let pct = format!("{}%", level);
                let pw = self.fonts.measure(Weight::Semibold, &pct, 14.5 * s);
                let right = x0 + w - 22.0;
                self.fonts.draw(&mut self.pm, Weight::Semibold, &pct, right * s - pw, (cy + 5.2) * s, 14.5 * s, GRAY, a);
                // the battery: outline, level, cap
                let (bw, bh) = (28.0, 14.0);
                let bx = right - pw / s - 10.0 - bw - 2.5;
                let by = cy - bh / 2.0;
                gfx::fill_rrect(&mut self.pm, bx * s, by * s, bw * s, bh * s, [4.2 * s; 4], [99, 99, 102], a);
                gfx::fill_rrect(&mut self.pm, (bx + 1.4) * s, (by + 1.4) * s, (bw - 2.8) * s, (bh - 2.8) * s, [3.0 * s; 4], BLACK, a);
                let fw = ((bw - 4.6) * (*level as f32 / 100.0).clamp(0.0, 1.0)).max(2.0);
                gfx::fill_rrect(&mut self.pm, (bx + 2.3) * s, (by + 2.3) * s, fw * s, (bh - 4.6) * s, [2.2 * s; 4], fill, a);
                gfx::fill_rrect(&mut self.pm, (bx + bw + 1.0) * s, (cy - 2.5) * s, 2.2 * s, 5.0 * s, [1.1 * s; 4], [99, 99, 102], a);
            }
        }
        let _ = h;
    }

    /// The home view (nothing is playing): the date and the latest notifications.
    fn draw_home(&mut self, x0: f32, y0: f32, w: f32, h: f32, a: f32) {
        let s = self.scale;
        const GRAY: [u8; 3] = [142, 142, 147];
        let _ = h;
        let rows = self.log.len().min(3);
        if rows == 0 {
            let t = "All quiet";
            let tw = self.fonts.measure(Weight::Semibold, t, 15.5 * s);
            self.fonts.draw(&mut self.pm, Weight::Semibold, t, (x0 + w / 2.0) * s - tw / 2.0, (y0 + 52.0) * s, 15.5 * s, WHITE, a);
            let sub = if self.clock.is_empty() { "No notifications".to_string() } else { format!("No notifications - {}", self.clock) };
            let sub = self.fonts.fit(Weight::Regular, &sub, 12.5 * s, (w - 36.0) * s);
            let sw = self.fonts.measure(Weight::Regular, &sub, 12.5 * s);
            self.fonts.draw(&mut self.pm, Weight::Regular, &sub, (x0 + w / 2.0) * s - sw / 2.0, (y0 + 73.0) * s, 12.5 * s, GRAY, a);
            return;
        }
        // header: the date on the left, "Clear" on the right
        let date = self.fonts.fit(Weight::Regular, &self.clock.clone(), 13.0 * s, (w - 110.0) * s);
        self.fonts.draw(&mut self.pm, Weight::Regular, &date, (x0 + 22.0) * s, (y0 + 32.0) * s, 13.0 * s, GRAY, a);
        let clear_hover = self.cursor.map(|(cx, cy)| matches!(self.hit(cx, cy), Hit::HomeClear)).unwrap_or(false);
        let cw = self.fonts.measure(Weight::Regular, "Clear", 12.5 * s);
        self.fonts.draw(&mut self.pm, Weight::Regular, "Clear", (x0 + w - 22.0) * s - cw, (y0 + 32.0) * s, 12.5 * s, if clear_hover { WHITE } else { GRAY }, a);
        let hov_row = self.cursor.and_then(|(cx, cy)| if let Hit::HomeRow(i) = self.hit(cx, cy) { Some(i) } else { None });
        for i in 0..rows {
            let top = y0 + 50.0 + 44.0 * i as f32;
            if hov_row == Some(i) {
                gfx::fill_rrect(&mut self.pm, (x0 + 10.0) * s, top * s, (w - 20.0) * s, 42.0 * s, [14.0 * s; 4], [28, 28, 30], a);
            }
            let (name, title, body, icon, at) = {
                let e = &self.log[i];
                (if e.n.app.is_empty() { e.n.title.clone() } else { e.n.app.clone() }, e.n.title.clone(), e.n.body.clone(), e.n.icon.clone(), e.at)
            };
            self.draw_icon(icon.as_deref(), &name, x0 + 20.0, top + 7.0, 28.0, 8.0, a);
            let secs = at.elapsed().as_secs();
            let ago = if secs < 45 { "now".to_string() } else if secs < 3600 { format!("{}m", (secs / 60).max(1)) } else if secs < 86400 { format!("{}h", secs / 3600) } else { format!("{}d", secs / 86400) };
            let aw = self.fonts.measure(Weight::Regular, &ago, 12.0 * s);
            self.fonts.draw(&mut self.pm, Weight::Regular, &ago, (x0 + w - 22.0) * s - aw, (top + 18.0) * s, 12.0 * s, [99, 99, 102], a);
            let max_w = (w - 20.0 - 58.0 - 18.0) - aw / s - 10.0;
            let t = self.fonts.fit(Weight::Semibold, &title, 13.5 * s, max_w * s);
            let b = self.fonts.fit(Weight::Regular, &body, 12.0 * s, max_w * s);
            self.fonts.draw(&mut self.pm, Weight::Semibold, &t, (x0 + 58.0) * s, (top + 18.0) * s, 13.5 * s, WHITE, a);
            self.fonts.draw(&mut self.pm, Weight::Regular, &b, (x0 + 58.0) * s, (top + 34.0) * s, 12.0 * s, GRAY, a);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn eq(&mut self, x: f32, bottom: f32, bw: f32, gap: f32, height: f32, n: usize, alpha: f32, playing: bool) {
        let base = if self.cfg.accent_eq { self.accent } else { WHITE };
        let (hue, sat) = hue_sat(base);
        let accent2 = gfx::hsl(hue + 42.0, sat.max(55.0), 68.0);
        if alpha <= 0.01 {
            return;
        }
        let s = self.scale;
        for i in 0..n {
            let speed = [5.2, 7.4, 4.1, 6.3, 5.7, 8.1, 4.6][i % 7];
            let phase = [0.0, 1.7, 0.6, 2.4, 3.1, 0.9, 2.0][i % 7];
            let level = if playing { 0.22 + 0.78 * (0.5 + 0.5 * (self.t_real * speed + phase).sin()) } else { 0.22 };
            let bh = (height * level).max(2.5);
            // the 7-bar equalizer fades from the accent (first bar) to the second accent (last bar); the small one is a single colour
            let accent = if n >= 7 {
                let f = 1.0 - i as f32 / (n - 1) as f32;
                let m = |a: u8, b: u8| (a as f32 * f + b as f32 * (1.0 - f)).round() as u8;
                [m(base[0], accent2[0]), m(base[1], accent2[1]), m(base[2], accent2[2])]
            } else {
                base
            };
            gfx::fill_rrect(&mut self.pm, (x + i as f32 * (bw + gap)) * s, (bottom - bh) * s, bw * s, bh * s, [bw * 0.5 * s; 4], accent, alpha);
        }
    }

    fn draw_open(&mut self, x0: f32, y0: f32, w: f32, h: f32, a: f32) {
        let s = self.scale;
        let lay = self.open_layout();
        let k = self.lyr_k.x; // 0 = the player, 1 = the lyrics
        // an element appears once the bottom edge of the (still growing) island has passed it
        let reach = |y: f32| smooth((h - y) / 18.0);
        let dy = (1.0 - a) * 4.0; // the content drifts up into place while it fades in
        const GRAY: [u8; 3] = [142, 142, 147];

        let left = x0 + lerp(104.0, 82.0, k);
        let max_w = (x0 + w - 26.0 - 44.0) - left;
        let (tpx, apx) = (lerp(17.0, 14.5, k), lerp(15.0, 12.0, k));
        let title = self.fonts.fit(Weight::Semibold, &self.media.title, tpx * s, max_w * s);
        let artist = self.fonts.fit(Weight::Regular, &self.media.artist, apx * s, max_w * s);
        self.fonts.draw(&mut self.pm, Weight::Semibold, &title, left * s, (y0 + lerp(47.0, 38.0, k) + dy) * s, tpx * s, WHITE, a);
        self.fonts.draw(&mut self.pm, Weight::Regular, &artist, left * s, (y0 + lerp(69.0, 55.0, k) + dy) * s, apx * s, GRAY, a);

        // lyrics and controls never show at the same time: one is gone before the other appears (like the Electron version)
        let a_player = a * clamp01((0.5 - k) / 0.5);
        let k_in = clamp01((k - 0.5) / 0.5);
        if let Some((tx, ty, tw)) = lay.track {
            let frac = self.drag_frac.unwrap_or(if self.media.dur > 0.0 { (self.position() / self.media.dur) as f32 } else { 0.0 });
            let (bx, by) = (x0 + tx, y0 + ty + dy);
            let ar = a_player * reach(ty + 8.0);
            let th = lerp(6.0, 10.0, self.track_k.x); // thick while hovered or dragged
            gfx::fill_rrect(&mut self.pm, bx * s, (by - th / 2.0) * s, tw * s, th * s, [th / 2.0 * s; 4], [44, 44, 46], ar);
            gfx::fill_rrect(&mut self.pm, bx * s, (by - th / 2.0) * s, (tw * frac).max(th) * s, th * s, [th / 2.0 * s; 4], WHITE, ar);
            let cur = fmt_time(self.position());
            let rem = format!("-{}", fmt_time((self.media.dur - self.position()).max(0.0)));
            let rw = self.fonts.measure(Weight::Regular, &rem, 12.0 * s);
            self.fonts.draw(&mut self.pm, Weight::Regular, &cur, (x0 + 26.0) * s, (by + 4.3) * s, 12.0 * s, GRAY, ar);
            self.fonts.draw(&mut self.pm, Weight::Regular, &rem, (x0 + lay.ow - 26.0) * s - rw, (by + 4.3) * s, 12.0 * s, GRAY, ar);
        }

        let cy = y0 + lay.play.1 + dy;
        let ac = a_player * reach(lay.play.1 + 16.0);
        let ic = gfx::icons();
        // each button: a soft grey square behind it while the mouse is over it, and it shrinks a little while pressed
        let btn_bg = |this: &mut Ui, i: usize, cx: f32, bw: f32| -> f32 {
            let sc = 1.0 - 0.12 * this.btn_press[i].x;
            let al = this.btn_hover[i].x * ac;
            if al > 0.01 {
                gfx::fill_rrect(&mut this.pm, (cx - bw * sc / 2.0) * s, (cy - 24.0 * sc) * s, bw * sc * s, 48.0 * sc * s, [14.0 * sc * s; 4], [28, 28, 30], al);
            }
            sc
        };
        let sc = btn_bg(self, 0, x0 + lay.prev.0, 52.0);
        gfx::fill_icon(&mut self.pm, &ic.fwd, (x0 + lay.prev.0) * s, cy * s, 32.0 * sc * s, true, WHITE, ac);
        let sc = btn_bg(self, 2, x0 + lay.next.0, 52.0);
        gfx::fill_icon(&mut self.pm, &ic.fwd, (x0 + lay.next.0) * s, cy * s, 32.0 * sc * s, false, WHITE, ac);
        let sc = btn_bg(self, 1, x0 + lay.play.0, 56.0);
        let main = if self.media.playing { &ic.pause } else { &ic.play };
        gfx::fill_icon(&mut self.pm, main, (x0 + lay.play.0) * s, cy * s, 38.0 * sc * s, false, WHITE, ac);
        // the lyrics button, at the right end of the controls (22px icon)
        if self.cfg.lyrics_button {
            let sc = btn_bg(self, 3, x0 + lay.lyr_btn.0, 44.0);
            let (lx, ly) = ((x0 + lay.lyr_btn.0) * s, cy * s);
            gfx::stroke_icon(&mut self.pm, &ic.lyrics_bubble, lx, ly, 22.0 * sc * s, 1.7, WHITE, ac * 0.9);
            gfx::stroke_icon(&mut self.pm, &ic.lyrics_lines, lx, ly, 22.0 * sc * s, 1.7, WHITE, ac * 0.9);
        }

        // lyrics view: the way back, and the lines
        if k_in > 0.01 {
            // the way back: the filled lyrics bubble (as in the Electron version)
            let (bx, by) = ((x0 + lay.ow - 37.0) * s, (y0 + 39.0) * s);
            let sc = 1.0 - 0.12 * self.btn_press[4].x;
            let al = self.btn_hover[4].x * a * k_in;
            if al > 0.01 {
                gfx::fill_rrect(&mut self.pm, bx - 17.0 * sc * s, by - 17.0 * sc * s, 34.0 * sc * s, 34.0 * sc * s, [11.0 * sc * s; 4], [28, 28, 30], al);
            }
            gfx::fill_icon(&mut self.pm, &ic.lyrics_bubble, bx, by, 22.0 * sc * s, false, WHITE, a * k_in);
            gfx::stroke_icon(&mut self.pm, &ic.lyrics_lines, bx, by, 22.0 * sc * s, 1.7, BLACK, a * k_in);
            self.draw_lyrics_area(x0, y0, h, a * k_in);
        }
    }

    /// The lines of the song, scrolling so that the one being sung is lit.
    fn draw_lyrics_area(&mut self, x0: f32, y0: f32, h: f32, a: f32) {
        let s = self.scale;
        let (top, bottom) = (y0 + 72.0, y0 + h - 12.0);
        let fade = (bottom - top) * 0.24;
        let win = Some((top * s, bottom * s, fade * s));
        const GRAY: [u8; 3] = [142, 142, 147];
        let mid = (top + bottom) / 2.0;
        let message = |this: &mut Ui, text: &str| {
            let tw = this.fonts.measure(Weight::Regular, text, 14.0 * s);
            let cx = x0 + this.open_w() / 2.0;
            this.fonts.draw(&mut this.pm, Weight::Regular, text, cx * s - tw / 2.0, (mid + 5.0) * s, 14.0 * s, GRAY, a);
        };
        match self.lyr_status {
            LyrStatus::Loading | LyrStatus::None => message(self, "Looking for lyrics…"),
            LyrStatus::Missing => message(self, "No lyrics found for this song"),
            LyrStatus::Ready => {
                let (px, lh) = if self.lyr_synced { (19.0, 23.75) } else { (15.0, 18.75) };
                let scroll = if self.lyr_synced { self.lyr_scroll.x } else { 0.0 };
                for (i, b) in self.lyr_blocks.iter().enumerate() {
                    let by = top + b.top - scroll;
                    if by + b.h < top - 4.0 || by > bottom + 4.0 {
                        continue;
                    }
                    // the lit line fades in over 0.3 s, the one before it fades out
                    let lit = smooth((self.t - self.lyr_since) / 0.3);
                    let la = if !self.lyr_synced {
                        0.85
                    } else if i as i32 == self.lyr_idx {
                        lerp(0.34, 1.0, lit)
                    } else if i as i32 == self.lyr_prev {
                        lerp(1.0, 0.34, lit)
                    } else {
                        0.34
                    };
                    // the other lines are 0.93 the size (scaled about their left-centre) and dimmer
                    let sk = if !self.lyr_synced { 1.0 } else if i as i32 == self.lyr_idx { lerp(0.93, 1.0, lit) } else if i as i32 == self.lyr_prev { lerp(1.0, 0.93, lit) } else { 0.93 };
                    for (ri, row) in b.rows.iter().enumerate() {
                        let center = by + 5.0 + ri as f32 * lh + lh / 2.0;
                        self.fonts.draw_faded(&mut self.pm, Weight::Bold, row, (x0 + 26.0) * s, (center + 0.31 * px * sk) * s, px * sk * s, WHITE, a * la, win);
                    }
                }
            }
        }
    }
}

/// Word-wrap `text` to `max` pixels (measured at `px`, both already scaled).
fn wrap(fonts: &mut Fonts, text: &str, px: f32, max: f32) -> Vec<String> {
    let mut rows = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let cand = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
        if cur.is_empty() || fonts.measure(Weight::Semibold, &cand, px) <= max {
            cur = cand;
        } else {
            rows.push(std::mem::take(&mut cur));
            cur = word.to_string();
        }
    }
    if !cur.is_empty() {
        rows.push(cur);
    }
    if rows.is_empty() {
        rows.push(String::new());
    }
    rows
}

struct OpenLayout {
    ow: f32,
    /// x, y (centre line) and width of the progress track
    track: Option<(f32, f32, f32)>,
    prev: (f32, f32),
    play: (f32, f32),
    next: (f32, f32),
    /// the lyrics button, at the right end of the controls row
    lyr_btn: (f32, f32),
}

pub fn fmt_time(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}
