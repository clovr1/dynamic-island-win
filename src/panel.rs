//! The settings window's contents, drawn by ourselves: a side list of pages on the left, the settings of the chosen page on the right,
//! like an ordinary Windows app. This file only knows about pixels and mouse positions; `settings_win.rs` puts it into a window.

use crate::gfx::{fill_rrect, Fonts, Weight};
use crate::settings::{MaxMode, Settings};
use crate::lyrics::{parse_lrc, Found};
use crate::ui::{ArtData, Env, Hit, MediaInfo, Need, NotifData, Ui};
use tiny_skia::{Color, GradientStop, LinearGradient, Mask, Paint, PathBuilder, Pixmap, Point, RadialGradient, SpreadMode, Transform};

/// width of the list of pages (logical pixels)
const SIDE: f32 = 224.0;
/// The colours of one theme.
#[derive(Clone, Copy)]
pub struct Palette {
    pub name: &'static str,
    pub win_bg: [u8; 3],
    pub page_bg: [u8; 3],
    pub card: [u8; 3],
    pub card_hover: [u8; 3],
    pub edge: [u8; 3],
    pub text: [u8; 3],
    pub dim: [u8; 3],
    pub accent: [u8; 3],
    /// text on top of the accent colour
    pub on_accent: [u8; 3],
    pub nav_on: [u8; 3],
    pub nav_hover: [u8; 3],
    pub light: bool,
}

pub const THEMES: [Palette; 6] = [
    Palette { name: "Midnight", win_bg: [32, 32, 32], page_bg: [39, 39, 39], card: [49, 49, 49], card_hover: [55, 55, 55], edge: [64, 64, 64], text: [255, 255, 255], dim: [166, 166, 166], accent: [96, 205, 255], on_accent: [0, 0, 0], nav_on: [55, 55, 55], nav_hover: [45, 45, 45], light: false },
    Palette { name: "Aurora", win_bg: [14, 27, 30], page_bg: [18, 34, 37], card: [25, 47, 50], card_hover: [31, 56, 60], edge: [42, 73, 77], text: [235, 255, 250], dim: [138, 176, 172], accent: [52, 226, 168], on_accent: [0, 28, 20], nav_on: [28, 52, 56], nav_hover: [22, 42, 46], light: false },
    Palette { name: "Sunset", win_bg: [33, 24, 26], page_bg: [41, 30, 32], card: [55, 40, 42], card_hover: [64, 47, 49], edge: [82, 60, 62], text: [255, 244, 238], dim: [188, 156, 148], accent: [255, 140, 82], on_accent: [40, 14, 0], nav_on: [62, 45, 47], nav_hover: [50, 36, 38], light: false },
    Palette { name: "Grape", win_bg: [26, 22, 38], page_bg: [32, 27, 47], card: [43, 36, 63], card_hover: [51, 43, 74], edge: [68, 59, 98], text: [246, 242, 255], dim: [164, 154, 196], accent: [182, 140, 255], on_accent: [24, 8, 52], nav_on: [50, 42, 74], nav_hover: [39, 33, 57], light: false },
    Palette { name: "Rose", win_bg: [34, 22, 30], page_bg: [43, 28, 38], card: [58, 38, 51], card_hover: [68, 45, 60], edge: [88, 60, 78], text: [255, 240, 247], dim: [190, 152, 172], accent: [255, 112, 166], on_accent: [48, 4, 26], nav_on: [66, 44, 58], nav_hover: [52, 34, 46], light: false },
    Palette { name: "Daylight", win_bg: [236, 238, 242], page_bg: [246, 247, 250], card: [255, 255, 255], card_hover: [240, 243, 250], edge: [218, 222, 232], text: [24, 28, 38], dim: [100, 108, 124], accent: [0, 103, 224], on_accent: [255, 255, 255], nav_on: [255, 255, 255], nav_hover: [226, 230, 238], light: true },
];

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
    InstallFile,
    Save,
    Discard,
    /// the little buttons under the preview
    DemoNotif,
    DemoMax,
    DemoFs,
    DemoStatus,
    StatusPills,
    DndQuiet,
    Sounds,
    Volume,
    PlayerVolume,
    Theme(u8),
    Logo,
}

/// What the panel asks its window to do.
pub enum Act {
    /// the draft was edited (nothing is applied yet)
    Changed(Settings),
    /// "Save": apply to the running island: settings, start with Windows
    Save(Settings, bool),
    CheckNow,
    Install,
    Test,
    Quit,
    InstallFile,
}

enum Row {
    /// id, title, description, on
    Toggle(Id, &'static str, &'static str, bool),
    /// id, title, description, value, min, max, text
    Slider(Id, &'static str, &'static str, f32, f32, f32, String),
    /// title, description, chosen, options
    Radio(&'static str, &'static str, usize, [&'static str; 3]),
    /// the theme picker: chosen
    Themes(u8),
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
            Row::Themes(_) => 126.0,
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
    /// the draft: what the window shows (and the preview); the running island has `saved`
    pub s: Settings,
    autostart: bool,
    saved: Settings,
    saved_autostart: bool,
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
    /// the live island at the top: the real island, with the settings applied to it
    preview: Ui,
    demo_max: bool,
    demo_fs: bool,
    preview_pct: u32,
    /// where the preview stage is (logical) and where the island's own picture starts inside the window (physical)
    stage: (f32, f32, f32, f32),
    canvas_at: (f32, f32),
    cursor_in: bool,
    preview_hand: bool,
    preview_down: bool,
    status_n: usize,
    notif_n: usize,
    /// everything must be drawn again (otherwise only the live island moves)
    needs_full: bool,
    stage_pm: Option<Pixmap>,
    stage_bg: Option<Pixmap>,
    stage_mask: Option<Mask>,
    /// seconds since the window opened (drives the little animations)
    t_anim: f32,
    /// seconds since the page changed (the rows slide in one after another)
    page_anim: f32,
    /// where the highlight in the page list is (it glides)
    nav_y: f32,
    /// the theme being faded to, the colours it came from, and how far the fade is (0..1)
    pal_theme: u8,
    pal_prev: Option<Palette>,
    pal_k: f32,
    ripples: Vec<Ripple>,
    /// seconds since the logo was clicked (-1: not now)
    burst: f32,
    /// the live island needs to be drawn again
    stage_force: bool,
    /// how "chosen" each dock button is (they swell and bounce)
    dock_k: [f32; 6],
    bg_cache: Option<((u32, u32, u8, u8), Pixmap)>,
}

/// A ripple that spreads from where a row or button was pressed.
struct Ripple {
    x: f32,
    y: f32,
    t0: f32,
    rect: (f32, f32, f32, f32),
}

/// height of the bar at the top, and where the page's controls start
const TOPBAR: f32 = 56.0;
const ITEMS_TOP: f32 = 462.0;

/// A thing on a page, as it is drawn: switches are chips, sliders are dials, themes are orbs.
enum Piece {
    Heading(String),
    Text(String),
    Chip { id: Id, label: String, on: bool },
    Action { id: Id, label: String, style: u8 },
    Dial { id: Id, label: String, v: f32, lo: f32, hi: f32, val: String, unit: String },
    Orb(u8, &'static str, bool),
}

const SUBTITLES: [&str; 6] = [
    "The basics.",
    "Tune the little pill that follows your songs.",
    "How loudly (or quietly) the world gets to knock.",
    "Manners for when windows get in the way.",
    "Make it yours: colours, size, sounds, speed.",
    "Keep Bloop fresh.",
];

/// A made-up song for the preview (a gradient cover, so the equalizer takes its colour from it).
fn sample_art() -> ArtData {
    let n = 96u32;
    let mut rgba = Vec::with_capacity((n * n * 4) as usize);
    for y in 0..n {
        for x in 0..n {
            let t = (x + y) as f32 / (2.0 * n as f32);
            let (cx, cy) = (x as f32 - n as f32 * 0.68, y as f32 - n as f32 * 0.32);
            let sun = (1.0 - ((cx * cx + cy * cy).sqrt() / (n as f32 * 0.22))).clamp(0.0, 1.0);
            let r = 40.0 + 190.0 * t + 60.0 * sun;
            let g = 30.0 + 60.0 * (1.0 - t) + 150.0 * sun;
            let b = 150.0 - 90.0 * t;
            rgba.extend_from_slice(&[r.min(255.0) as u8, g.min(255.0) as u8, b.min(255.0) as u8, 255]);
        }
    }
    ArtData { w: n, h: n, rgba }
}

const SAMPLE_LRC: &str = "[00:30.00]Streetlights are counting down\n[00:36.00]Windows rolling, nobody's around\n[00:42.00]Every mile a little brighter\n[00:48.00]Every turn a little lighter\n[00:54.00]Take the long way home tonight\n[01:00.00]Keep the radio on low\n[01:06.00]Let the city fade to light\n[01:12.00]Nowhere else I want to go\n[01:20.00]Midnight drive\n[01:30.00]";

fn rr(pm: &mut Pixmap, s: f32, x: f32, y: f32, w: f32, h: f32, r: f32, rgb: [u8; 3], a: f32) {
    fill_rrect(pm, x * s, y * s, w * s, h * s, [r * s; 4], rgb, a);
}

/// Put `src` (premultiplied) over `dst` at (dx, dy): transparent pixels are skipped, solid ones copied, the rest blended.
/// (A plain loop: much faster than the general drawing routine for big pictures.)
fn blit_over(dst: &mut Pixmap, src: &Pixmap, dx: i32, dy: i32) {
    let (dw, dh) = (dst.width() as i32, dst.height() as i32);
    let (sw, sh) = (src.width() as i32, src.height() as i32);
    let x0 = dx.max(0);
    let x1 = (dx + sw).min(dw);
    if x1 <= x0 {
        return;
    }
    let sdata = src.data();
    let ddata = dst.data_mut();
    for y in dy.max(0)..(dy + sh).min(dh) {
        let srow = (((y - dy) * sw + (x0 - dx)) * 4) as usize;
        let drow = ((y * dw + x0) * 4) as usize;
        let n = ((x1 - x0) * 4) as usize;
        for (d, s) in ddata[drow..drow + n].chunks_exact_mut(4).zip(sdata[srow..srow + n].chunks_exact(4)) {
            let a = s[3] as u32;
            if a == 0 {
                continue;
            }
            if a == 255 {
                d.copy_from_slice(s);
            } else {
                let inv = 255 - a;
                for k in 0..4 {
                    d[k] = (s[k] as u32 + (d[k] as u32 * inv + 127) / 255).min(255) as u8;
                }
            }
        }
    }
}

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let m = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t).round().clamp(0.0, 255.0) as u8;
    [m(0), m(1), m(2)]
}

impl Panel {
    /// The default (and smallest) size of the window's inside, logical pixels.
    pub const DEFAULT_SIZE: (f32, f32) = (980.0, 790.0);
    pub const MIN_SIZE: (f32, f32) = (780.0, 640.0);

    pub fn new(scale: f32, s: Settings, autostart: bool, size_px: (u32, u32)) -> Panel {
        let mut p = Panel {
            fonts: Fonts::new_ui(),
            scale,
            w: 1.0,
            h: 1.0,
            pm: Pixmap::new(1, 1).unwrap(),
            saved: s.clone(),
            saved_autostart: autostart,
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
            preview: Ui::new(scale),
            demo_max: false,
            demo_fs: false,
            preview_pct: 100,
            stage: (0.0, 0.0, 0.0, 0.0),
            canvas_at: (0.0, 0.0),
            cursor_in: false,
            preview_hand: false,
            preview_down: false,
            status_n: 0,
            notif_n: 0,
            needs_full: true,
            stage_pm: None,
            stage_bg: None,
            stage_mask: None,
            t_anim: 0.0,
            page_anim: 1.0,
            nav_y: 84.0,
            pal_theme: 0,
            pal_prev: None,
            pal_k: 1.0,
            ripples: Vec::new(),
            burst: -1.0,
            stage_force: true,
            dock_k: [0.0; 6],
            bg_cache: None,
        };
        p.pal_theme = p.s.theme;
        p.status = format!("Updates come from github.com/{}", p.s.update_repo);
        p.resize(size_px.0, size_px.1);
        p.start_preview();
        p
    }

    fn start_preview(&mut self) {
        self.preview_pct = self.s.scale_pct;
        self.preview.set_scale(self.scale * self.s.scale_pct as f32 / 100.0);
        self.preview.set_settings(&self.s);
        self.preview.set_art(Some(sample_art()));
        self.preview.set_media(MediaInfo { active: true, title: "Midnight Drive".into(), artist: "The Islanders".into(), playing: true, pos: 52.0, dur: 214.0 });
    }

    fn sync_preview(&mut self) {
        self.stage_force = true;
        if self.preview_pct != self.s.scale_pct {
            self.preview_pct = self.s.scale_pct;
            self.preview.set_scale(self.scale * self.s.scale_pct as f32 / 100.0);
        }
        self.preview.set_settings(&self.s);
        self.preview.set_env(Env { maximized: self.demo_max, fullscreen: self.demo_fs, ..Env::default() });
    }

    fn feed_lyrics(&mut self) {
        if let Some(r) = self.preview.take_lyrics_request() {
            self.preview.set_lyrics(&r.key, Some(Found { synced: true, lines: parse_lrc(SAMPLE_LRC) }));
        }
    }

    fn demo_status(&mut self) {
        use crate::ui::StatusData;
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as usize).unwrap_or(0);
        let keys = ["wifi", "bluetooth", "airplane", "dnd", "nightlight", "energy", "hotspot", "cellular"];
        self.status_n += 1;
        if self.status_n % 9 == 0 {
            self.preview.push_status(StatusData::Battery { label: "Charging".into(), level: 64, charging: true, low: false });
        } else {
            let key = keys[(t / 7 + self.status_n) % keys.len()];
            self.preview.push_status(StatusData::Sys { key: key.into(), on: self.status_n % 2 == 1 });
        }
    }

    fn demo_notification(&mut self) {
        let who = [
            ("Alex", "Are we still on for tonight?"),
            ("Mum", "Dinner at 7 tonight, don't be late. I made your favourite and everyone is already waiting for you."),
            ("Nova", "Your build finished: 0 errors, 2 warnings. Tap to see the full log and the list of changed files."),
        ];
        self.notif_n += 1;
        let (name, text) = who[self.notif_n % who.len()];
        self.preview.forget_recent();
        self.preview.push_notification(NotifData { app: "Messages".into(), aumid: String::new(), title: name.into(), body: text.into(), icon: None });
    }

    /// The window changed size.
    pub fn resize(&mut self, w_px: u32, h_px: u32) {
        let (w_px, h_px) = (w_px.max(2), h_px.max(2));
        self.needs_full = true;
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
        self.needs_full = true;
        self.saved = s.clone();
        self.saved_autostart = autostart;
        self.s = s;
        self.autostart = autostart;
        self.sync_preview();
    }

    /// Unsaved changes?
    pub fn dirty(&self) -> bool {
        self.s != self.saved || self.autostart != self.saved_autostart
    }

    pub fn autostart_draft(&self) -> bool {
        self.autostart
    }

    /// The draft was applied: it is now what the running island has.
    /// Sounds the live island wants to play (the window plays them with the draft's volume).
    pub fn take_sounds(&mut self) -> Vec<crate::ui::Sound> {
        self.preview.take_sounds()
    }

    pub fn mark_saved(&mut self) {
        self.needs_full = true;
        self.saved = self.s.clone();
        self.saved_autostart = self.autostart;
    }

    fn discard(&mut self) {
        self.s = self.saved.clone();
        self.autostart = self.saved_autostart;
        self.sync_preview();
    }

    pub fn set_page(&mut self, page: u8) {
        self.needs_full = true;
        if self.page != page {
            self.page_anim = 0.0;
        }
        self.page = page.min(PAGES.len() as u8 - 1);
        self.scroll = 0.0;
    }

    pub fn set_status(&mut self, text: &str, ready: bool) {
        self.needs_full = true;
        self.status = text.to_string();
        self.ready = ready;
        self.busy = false;
    }

    pub fn set_busy(&mut self, text: &str) {
        self.needs_full = true;
        self.status = text.to_string();
        self.busy = true;
    }

    pub fn pal(&self) -> Palette {
        let to = THEMES[(self.pal_theme as usize).min(THEMES.len() - 1)];
        match &self.pal_prev {
            Some(from) if self.pal_k < 1.0 => {
                let k = self.pal_k * self.pal_k * (3.0 - 2.0 * self.pal_k);
                let m = |a: [u8; 3], b: [u8; 3]| mix(a, b, k);
                Palette { name: to.name, win_bg: m(from.win_bg, to.win_bg), page_bg: m(from.page_bg, to.page_bg), card: m(from.card, to.card), card_hover: m(from.card_hover, to.card_hover), edge: m(from.edge, to.edge), text: m(from.text, to.text), dim: m(from.dim, to.dim), accent: m(from.accent, to.accent), on_accent: m(from.on_accent, to.on_accent), nav_on: m(from.nav_on, to.nav_on), nav_hover: m(from.nav_hover, to.nav_hover), light: to.light }
            }
            _ => to,
        }
    }

    /// For the window's title bar: (dark?, colour as a Windows COLORREF 0x00BBGGRR).
    pub fn title_colors(&self) -> (bool, u32) {
        let c = self.pal().win_bg;
        (!self.pal().light, (c[2] as u32) << 16 | (c[1] as u32) << 8 | c[0] as u32)
    }

    fn rows(&self) -> Vec<Row> {
        let s = &self.s;
        match self.page {
            0 => vec![
                Row::Toggle(Id::Autostart, "Start with Windows", "Open the island automatically when you sign in (applied when you press Save)", self.autostart),
                Row::Button(Id::Reset, "Reset all settings", "Put every setting back to how it was at the start (press Save to apply)".into(), "Reset".into(), 0),
                Row::Button(Id::Quit, "Quit Bloop", "Close the island completely".into(), "Quit".into(), 2),
            ],
            1 => vec![
                Row::Toggle(Id::Pill, "Small pill while music plays", "Show the cover and equalizer at the top while a song plays", s.show_music_pill),
                Row::Toggle(Id::PlayerFront, "Hide it while the music app is in front", "No need for the pill when the player's own window is open", s.hide_when_player_front),
                Row::Toggle(Id::Lyrics, "Lyrics button", "Show a lyrics button in the player (lyrics come from lrclib.net)", s.lyrics_button),
                Row::Toggle(Id::PlayerVolume, "Volume slider in the player", "Change the volume of the music right from the open player", s.player_volume),
                Row::Toggle(Id::Eq, "Equalizer in the cover's colour", "Otherwise the equalizer is white", s.accent_eq),
            ],
            2 => vec![
                Row::Toggle(Id::Notif, "Show notifications in the island", "Messages from your apps appear at the top of the screen", s.notifications),
                Row::Toggle(Id::Bubble, "Small bubble next to the music pill", "A small bubble while music plays, otherwise a card", s.use_bubble),
                Row::Toggle(Id::StatusPills, "Status pills", "A slim pill when Wi-Fi, Bluetooth, airplane mode, battery ... change", s.status_pills),
                Row::Toggle(Id::DndQuiet, "Respect Do Not Disturb", "While it is on, notifications are only listed in the island's home view", s.dnd_quiet),
                Row::Slider(Id::CardSecs, "How long a card stays", "Seconds before it goes away", s.card_secs as f32, 2.0, 20.0, format!("{} s", s.card_secs)),
                Row::Button(Id::Test, "Test", "Show a test notification now".into(), "Show".into(), 0),
            ],
            3 => vec![
                Row::Radio("Under a maximized window", "What the island does when a window covers the top of the screen", match s.maximized { MaxMode::Notch => 0, MaxMode::Hide => 1, MaxMode::Normal => 2 }, ["Become a slim notch", "Hide (move the mouse to the top edge to bring it back)", "Do nothing"]),
                Row::Toggle(Id::FsHide, "Hide for full-screen apps", "Videos and games get the whole screen", s.fullscreen_hide),
            ],
            4 => vec![
                Row::Themes(s.theme),
                Row::Toggle(Id::Sounds, "Sound effects", "Soft sounds for notifications, status pills and the play, pause and skip buttons (hovering is silent)", s.sounds),
                Row::Slider(Id::Volume, "Volume", "How loud the sound effects are", s.volume as f32, 0.0, 100.0, format!("{} %", s.volume)),
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
                    Row::Button(Id::InstallFile, "Install from a file", "Pick a newer Bloop.exe on this PC (to try a build before it is published)".into(), "Choose file...".into(), 0),
                    Row::Button(Id::CheckNow, "Bloop", format!("Version {} - {}", crate::version::VERSION, self.status), label, style),
                ]
            }
        }
    }

    /// Moves the switches (and the live island) forward. True while something still moves.
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
        // the little animations of the window itself
        self.t_anim += dt;
        self.page_anim += dt;
        for i in 0..6 {
            let target = if self.page as usize == i { 1.0 } else { 0.0 };
            let d = target - self.dock_k[i];
            if d.abs() > 0.004 {
                self.dock_k[i] += d * (1.0 - (-dt * 14.0).exp());
                moving = true;
            } else {
                self.dock_k[i] = target;
            }
        }
        if self.page_anim < 0.8 {
            moving = true;
        }
        if self.s.theme != self.pal_theme {
            self.pal_prev = Some(self.pal());
            self.pal_theme = self.s.theme;
            self.pal_k = 0.0;
        }
        if self.pal_k < 1.0 {
            self.pal_k = (self.pal_k + dt / 0.4).min(1.0);
            moving = true;
        }
        let now = self.t_anim;
        self.ripples.retain(|r| now - r.t0 < 0.55);
        if !self.ripples.is_empty() {
            moving = true;
        }
        if self.burst >= 0.0 {
            self.burst += dt;
            if self.burst > 0.9 {
                self.burst = -1.0;
            }
            moving = true;
        }
        if moving {
            self.needs_full = true;
        }
        let need = self.preview.tick(dt);
        if need != Need::Rest {
            self.stage_force = true;
        }
        self.feed_lyrics();
        moving || need != Need::Rest
    }

    /// How soon the next picture is wanted (None: nothing moves).
    pub fn frame_ms(&self) -> Option<u32> {
        if self.anim.iter().any(|a| (a.2 - a.1).abs() > 0.001) {
            return Some(16);
        }
        if self.page_anim < 0.8 || self.pal_k < 1.0 || !self.ripples.is_empty() || self.burst >= 0.0 || (0..6).any(|i| (self.dock_k[i] - if self.page as usize == i { 1.0 } else { 0.0 }).abs() > 0.004) {
            return Some(16);
        }
        match self.preview.need() {
            Need::Fast => Some(16),
            _ => Some(33), // (the logo's bubbles keep drifting)
        }
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

    #[allow(non_snake_case, unused_variables)]
    /// The next picture: the whole window when something changed, otherwise only the logo (always drifting) and, when it moves,
    /// the live island's box. Gives the boxes that changed.
    pub fn frame(&mut self) -> (&Pixmap, Vec<(i32, i32, i32, i32)>) {
        if self.needs_full || self.stage.2 == 0.0 {
            let (w, h) = self.size_px();
            return (self.render(), vec![(0, 0, w as i32, h as i32)]);
        }
        let s = self.scale;
        let mut pm = std::mem::replace(&mut self.pm, Pixmap::new(1, 1).unwrap());
        let (pw, ph) = (pm.width() as i32, pm.height() as i32);
        let mut rects = Vec::new();
        // the logo
        self.draw_logo(&mut pm, false);
        rects.push((0, 0, ((SIDE * s).ceil() as i32).min(pw), ((84.0 * s).ceil() as i32).min(ph)));
        if self.stage_force {
            self.stage_force = false;
            let (x, y, w, h) = self.stage;
            let bg = self.pal().page_bg;
            rr(&mut pm, s, x - 2.0, y - 2.0, w + 4.0, h + 4.0, 0.0, bg, 1.0);
            self.draw_stage(&mut pm, x, y, w, h);
            let rx = ((x - 2.0) * s).floor().max(0.0) as i32;
            let ry = ((y - 2.0) * s).floor().max(0.0) as i32;
            rects.push((rx, ry, (((w + 4.0) * s).ceil() as i32).min(pw - rx).max(1), (((h + 4.0) * s).ceil() as i32).min(ph - ry).max(1)));
        }
        self.pm = pm;
        (&self.pm, rects)
    }

    /// The colour of each page (its dock button, its switches, its dials).
    fn cat_color(&self, page: u8) -> [u8; 3] {
        const CAT: [[u8; 3]; 6] = [[100, 210, 255], [255, 92, 141], [255, 159, 10], [64, 156, 255], [176, 132, 255], [48, 209, 88]];
        CAT[(page as usize).min(5)]
    }

    /// The background: the theme's colour with two soft glows in the colour of the page. (Made once, then copied.)
    fn paint_bg(&self, pm: &mut Pixmap) {
        let pal = self.pal();
        let c = self.cat_color(self.page);
        pm.fill(Color::from_rgba8(pal.win_bg[0], pal.win_bg[1], pal.win_bg[2], 255));
        let (w, h) = (pm.width() as f32, pm.height() as f32);
        let glow = |pm: &mut Pixmap, cx: f32, cy: f32, r: f32, a: u8| {
            let mut paint = Paint::default();
            paint.shader = RadialGradient::new(
                Point::from_xy(cx, cy),
                Point::from_xy(cx, cy),
                r,
                vec![GradientStop::new(0.0, tiny_skia::Color::from_rgba8(c[0], c[1], c[2], a)), GradientStop::new(1.0, tiny_skia::Color::from_rgba8(c[0], c[1], c[2], 0))],
                SpreadMode::Pad,
                Transform::identity(),
            )
            .unwrap_or(tiny_skia::Shader::SolidColor(tiny_skia::Color::TRANSPARENT));
            if let Some(r) = tiny_skia::Rect::from_xywh(0.0, 0.0, w, h) {
                pm.fill_rect(r, &paint, Transform::identity(), None);
            }
        };
        let k = if pal.light { 0.55 } else { 1.0 };
        glow(pm, w * 0.12, h * 0.05, w * 0.55, (46.0 * k) as u8);
        glow(pm, w * 0.95, h * 0.95, w * 0.5, (30.0 * k) as u8);
    }

    /// Copy rows `y0..y1` (physical pixels) of the background over what is there (the cache, or painted fresh while the theme fades).
    fn restore_bg(&mut self, pm: &mut Pixmap, y0: u32, y1: u32) {
        let (pw, ph) = (pm.width(), pm.height());
        let key = (pw, ph, self.pal_theme, self.page);
        let fading = self.pal_k < 1.0;
        if !fading && self.bg_cache.as_ref().map(|(k, _)| *k != key).unwrap_or(true) {
            let mut bg = Pixmap::new(pw, ph).unwrap();
            self.paint_bg(&mut bg);
            self.bg_cache = Some((key, bg));
        }
        let y1 = y1.min(ph);
        let (a, b) = ((y0 * pw * 4) as usize, (y1 * pw * 4) as usize);
        match (&self.bg_cache, fading) {
            (Some((_, bg)), false) => pm.data_mut()[a..b].copy_from_slice(&bg.data()[a..b]),
            _ => {
                let mut bg = Pixmap::new(pw, ph).unwrap();
                self.paint_bg(&mut bg);
                pm.data_mut()[a..b].copy_from_slice(&bg.data()[a..b]);
            }
        }
    }

    /// The top bar: the logo (a little pill that lets a bubble float up now and then; click it for a burst), the name and version, and
    /// - when there are unsaved changes - Discard and Save.
    #[allow(non_snake_case, unused_variables)]
    fn draw_logo(&mut self, pm: &mut Pixmap, push_hot: bool) {
        let s = self.scale;
        let pal = self.pal();
        let (TEXT, DIM) = (pal.text, pal.dim);
        let ACCENT = self.cat_color(self.page);
        let h = (TOPBAR * s).ceil() as u32;
        let full = pm.width();
        // (the bar's own background first)
        self.restore_bg(pm, 0, h);
        let t = self.t_anim;
        // the pill
        let (px, py, pw, ph) = (22.0, 14.0, 56.0, 26.0);
        let squash = if self.burst >= 0.0 { 1.0 + 0.14 * (self.burst * 14.0).sin() * (-self.burst * 4.0).exp() } else { 1.0 };
        let (qw, qh) = (pw * squash, ph / squash);
        let (qx, qy) = (px + (pw - qw) / 2.0, py + (ph - qh) / 2.0);
        rr(pm, s, qx, qy, qw, qh, qh / 2.0, [0, 0, 0], 1.0);
        rr(pm, s, qx + 5.0, qy + qh / 2.0 - 7.5, 15.0, 15.0, 5.0, [255, 138, 76], 1.0);
        rr(pm, s, qx + 8.5, qy + qh / 2.0 - 4.0, 8.0, 8.0, 3.0, [255, 80, 150], 0.85);
        for i in 0..3 {
            let bh = 4.0 + 7.0 * (0.5 + 0.5 * (t * (5.0 + i as f32 * 1.7) + i as f32 * 1.3).sin());
            rr(pm, s, qx + qw - 21.0 + i as f32 * 5.5, qy + qh / 2.0 - bh / 2.0, 2.6, bh, 1.3, [255, 255, 255], 0.9);
        }
        // a bubble leaves the pill every few seconds, floats up and pops
        let p = (t % 3.8) / 3.8;
        if p < 0.55 {
            let q = p / 0.55;
            let (bx, by) = (qx + qw + 3.0 + q * 10.0, qy + qh / 2.0 - q * 12.0);
            let r = 2.0 + 3.6 * (std::f32::consts::PI * (q * 0.9).min(1.0)).sin().max(0.2) * (1.0 - q * 0.3);
            let a = if q > 0.82 { 1.0 - (q - 0.82) / 0.18 } else { 1.0 };
            rr(pm, s, bx - r, by - r, r * 2.0, r * 2.0, r, ACCENT, 0.9 * a);
            if q > 0.8 {
                let ring = (q - 0.8) / 0.2;
                let rd = r + ring * 6.0;
                rr(pm, s, bx - rd, by - rd, rd * 2.0, rd * 2.0, rd, ACCENT, 0.25 * (1.0 - ring));
            }
        }
        if self.burst >= 0.0 {
            let b = (self.burst / 0.9).min(1.0);
            for i in 0..9 {
                let ang = i as f32 / 9.0 * std::f32::consts::TAU + 0.4;
                let d = 10.0 + 38.0 * (1.0 - (1.0 - b).powi(3));
                let r = 4.5 * (1.0 - b * 0.7);
                let (bx, by) = (px + pw / 2.0 + ang.cos() * d * 1.5, py + ph / 2.0 + ang.sin() * d);
                rr(pm, s, bx - r, by - r, r * 2.0, r * 2.0, r, if i % 2 == 0 { ACCENT } else { [255, 138, 76] }, 0.85 * (1.0 - b));
            }
        }
        self.fonts.draw(pm, Weight::Bold, "Bloop", 94.0 * s, 32.0 * s, 21.0 * s, TEXT, 1.0);
        self.fonts.draw(pm, Weight::Regular, &format!("v{}", crate::version::VERSION), 160.0 * s, 32.0 * s, 12.0 * s, DIM, 1.0);
        if push_hot {
            self.hot.push(Hot { id: Id::Logo, x: px - 8.0, y: 4.0, w: 190.0, h: 46.0 });
        }
        let _ = full;
    }

    /// A flat little icon for the dock (white shapes on the coloured circle).
    fn draw_glyph(&self, pm: &mut Pixmap, kind: usize, cx: f32, cy: f32, sz: f32, fg: [u8; 3], bg: [u8; 3]) {
        let s = self.scale;
        let c = |pm: &mut Pixmap, x: f32, y: f32, r: f32, col: [u8; 3]| rr(pm, s, cx + x * sz - r * sz, cy + y * sz - r * sz, 2.0 * r * sz, 2.0 * r * sz, r * sz, col, 1.0);
        let b = |pm: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, col: [u8; 3]| rr(pm, s, cx + x * sz, cy + y * sz, w * sz, h * sz, r * sz, col, 1.0);
        match kind {
            0 => {
                // two sliders
                b(pm, -0.34, -0.19, 0.68, 0.07, 0.035, fg);
                b(pm, -0.34, 0.12, 0.68, 0.07, 0.035, fg);
                c(pm, -0.1, -0.155, 0.11, fg);
                c(pm, -0.1, -0.155, 0.05, bg);
                c(pm, 0.14, 0.155, 0.11, fg);
                c(pm, 0.14, 0.155, 0.05, bg);
            }
            1 => {
                // a note
                c(pm, -0.1, 0.17, 0.14, fg);
                b(pm, 0.02, -0.3, 0.06, 0.5, 0.03, fg);
                b(pm, 0.02, -0.3, 0.24, 0.11, 0.05, fg);
            }
            2 => {
                // a bell
                c(pm, 0.0, -0.06, 0.2, fg);
                b(pm, -0.2, -0.06, 0.4, 0.2, 0.02, fg);
                b(pm, -0.27, 0.08, 0.54, 0.09, 0.045, fg);
                c(pm, 0.0, 0.25, 0.065, fg);
            }
            3 => {
                // a window
                b(pm, -0.3, -0.24, 0.6, 0.48, 0.07, fg);
                b(pm, -0.26, -0.12, 0.52, 0.32, 0.03, bg);
                c(pm, -0.2, -0.18, 0.025, bg);
            }
            4 => {
                // a drop
                c(pm, 0.0, 0.09, 0.21, fg);
                let mut pb = PathBuilder::new();
                pb.move_to(cx, cy - 0.32 * sz);
                pb.line_to(cx + 0.19 * sz, cy);
                pb.line_to(cx - 0.19 * sz, cy);
                pb.close();
                if let Some(path) = pb.finish() {
                    let mut paint = Paint::default();
                    paint.set_color_rgba8(fg[0], fg[1], fg[2], 255);
                    paint.anti_alias = true;
                    pm.fill_path(&path, &paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
                }
            }
            _ => {
                // an arrow up
                let mut pb = PathBuilder::new();
                pb.move_to(cx, cy - 0.3 * sz);
                pb.line_to(cx + 0.24 * sz, cy - 0.02 * sz);
                pb.line_to(cx - 0.24 * sz, cy - 0.02 * sz);
                pb.close();
                if let Some(path) = pb.finish() {
                    let mut paint = Paint::default();
                    paint.set_color_rgba8(fg[0], fg[1], fg[2], 255);
                    paint.anti_alias = true;
                    pm.fill_path(&path, &paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
                }
                b(pm, -0.075, -0.04, 0.15, 0.3, 0.03, fg);
            }
        }
    }

    /// What the thing under the mouse does (shown under the dock).
    fn hint_for(&self, rows: &[Row], id: Id) -> String {
        for r in rows {
            match r {
                Row::Toggle(i, t, d, _) if *i == id => return format!("{t}: {d}"),
                Row::Slider(i, t, d, ..) if *i == id => return format!("{t}: {d}"),
                Row::Button(i, t, d, _, _) if *i == id => return format!("{t}: {d}"),
                Row::Radio(t, _, _, opts) => {
                    if let Id::MaxOpt(n) = id {
                        return format!("{t}: {}", opts[(n as usize).min(2)]);
                    }
                }
                Row::Themes(_) => {
                    if let Id::Theme(n) = id {
                        return format!("Theme {}: the colours of this window (the island itself stays black)", THEMES[(n as usize).min(5)].name);
                    }
                }
                _ => {}
            }
        }
        match id {
            Id::Nav(i) => SUBTITLES[(i as usize).min(5)].to_string(),
            Id::Logo => "Bloop! Click for bubbles".to_string(),
            Id::DemoNotif => "Send a notification to the live island above".to_string(),
            Id::DemoStatus => "Show a status pill (Wi-Fi, Bluetooth, battery ...)".to_string(),
            Id::DemoMax => "Pretend a window is maximized under the island".to_string(),
            Id::DemoFs => "Pretend a full-screen app is open".to_string(),
            Id::Save => "Apply everything to the running island".to_string(),
            Id::Discard => "Throw the changes away".to_string(),
            _ => String::new(),
        }
    }

    pub fn render(&mut self) -> &Pixmap {
        self.needs_full = false;
        let pal = self.pal();
        let (CARD, CARD_HOVER, EDGE, TEXT, DIM) = (pal.card, pal.card_hover, pal.edge, pal.text, pal.dim);
        let s = self.scale;
        let acc = self.cat_color(self.page);
        let on_acc = if acc[0] as f32 * 0.299 + acc[1] as f32 * 0.587 + acc[2] as f32 * 0.114 > 150.0 { [20, 20, 26] } else { [255, 255, 255] };
        let mut pm = std::mem::replace(&mut self.pm, Pixmap::new(1, 1).unwrap());
        let (pw, ph) = (pm.width(), pm.height());
        self.restore_bg(&mut pm, 0, ph);
        self.hot.clear();
        let (w, h) = (self.w, self.h);
        let rows = self.rows();

        // ---- the settings of the page: switches as chips, sliders as dials, themes as orbs ----
        let cw = (w - 64.0).min(760.0);
        let ml = (w - cw) / 2.0;
        let items_top = ITEMS_TOP;
        let pieces = self.pieces(&rows);
        let mut x = ml;
        let mut y = items_top - self.scroll;
        let mut line_h = 0.0f32;
        let gap = 12.0;
        for (i, pc) in pieces.iter().enumerate() {
            let (iw, ih, newline) = match pc {
                Piece::Heading(_) | Piece::Text(_) => (cw, 26.0, true),
                Piece::Chip { label, .. } => ((self.fonts.measure(Weight::Semibold, label, 14.5 * s) / s + 56.0).min(cw), 42.0, false),
                Piece::Action { label, .. } => (self.fonts.measure(Weight::Semibold, label, 14.5 * s) / s + 44.0, 42.0, false),
                Piece::Dial { .. } => (124.0, 142.0, false),
                Piece::Orb(..) => (78.0, 86.0, false),
            };
            // a new line when it does not fit, for headings, and when the kind of thing changes (switches / dials / orbs)
            let group = |pc: &Piece| match pc {
                Piece::Chip { .. } | Piece::Action { .. } => 0,
                Piece::Dial { .. } => 1,
                Piece::Orb(..) => 2,
                _ => 3,
            };
            let changed = i > 0 && group(&pieces[i - 1]) != group(pc);
            if (newline && x > ml) || (x > ml && (x + iw > ml + cw || changed)) {
                y += line_h + gap;
                x = ml;
                line_h = 0.0;
            }
            let k = ((self.page_anim - 0.035 * i as f32) / 0.32).clamp(0.0, 1.0);
            let off = 22.0 * (1.0 - k).powi(3);
            let oy = y + off;
            match pc {
                Piece::Heading(t) => {
                    self.fonts.draw(&mut pm, Weight::Semibold, &t.to_uppercase(), x * s, (oy + 17.0) * s, 11.5 * s, DIM, 1.0);
                }
                Piece::Text(t) => {
                    let t = self.fonts.fit(Weight::Regular, t, 13.5 * s, iw * s);
                    self.fonts.draw(&mut pm, Weight::Regular, &t, x * s, (oy + 17.0) * s, 13.5 * s, DIM, 1.0);
                }
                Piece::Chip { id, label, on } => {
                    let t = self.want(*id, if *on { 1.0 } else { 0.0 });
                    let e = t * t * (3.0 - 2.0 * t);
                    let squash = 1.0 + 0.07 * (std::f32::consts::PI * t).sin();
                    let hov = self.hover == Some(*id);
                    let (bw, bh) = (iw * squash, ih / squash);
                    let (bx, by) = (x + (iw - bw) / 2.0, oy + (ih - bh) / 2.0);
                    let base = if hov { CARD_HOVER } else { CARD };
                    rr(&mut pm, s, bx - 1.0, by - 1.0, bw + 2.0, bh + 2.0, bh / 2.0 + 1.0, mix(EDGE, acc, e), 1.0);
                    rr(&mut pm, s, bx, by, bw, bh, bh / 2.0, mix(base, acc, e), 1.0);
                    // the dot: hollow when off, a solid white one when on
                    rr(&mut pm, s, bx + 16.0, by + bh / 2.0 - 6.0, 12.0, 12.0, 6.0, mix(EDGE, on_acc, e), 1.0);
                    rr(&mut pm, s, bx + 18.0, by + bh / 2.0 - 4.0, 8.0, 8.0, 4.0, mix(base, acc, e), 1.0);
                    rr(&mut pm, s, bx + 19.5, by + bh / 2.0 - 2.5, 5.0, 5.0, 2.5, on_acc, e);
                    let l = self.fonts.fit(Weight::Semibold, label, 14.5 * s, (iw - 52.0) * s);
                    self.text_mid(&mut pm, Weight::Semibold, &l, bx + 38.0, by + bh / 2.0, 14.5, mix(TEXT, on_acc, e));
                    self.hot.push(Hot { id: *id, x, y: oy, w: iw, h: ih });
                }
                Piece::Action { id, label, style } => {
                    let hov = self.hover == Some(*id);
                    let pressed = hov && self.down == Some(*id);
                    let base = if *style == 1 { acc } else { CARD };
                    let col = if pressed { mix(base, [0, 0, 0], 0.25) } else if hov { mix(base, [255, 255, 255], 0.14) } else { base };
                    let grow = if hov { 1.04 } else { 1.0 };
                    let (bw, bh) = (iw * grow, ih * grow);
                    let (bx, by) = (x + (iw - bw) / 2.0, oy + (ih - bh) / 2.0);
                    rr(&mut pm, s, bx - 1.0, by - 1.0, bw + 2.0, bh + 2.0, bh / 2.0 + 1.0, if *style == 2 { [255, 105, 97] } else { EDGE }, if *style == 1 { 0.0 } else { 1.0 });
                    rr(&mut pm, s, bx, by, bw, bh, bh / 2.0, col, 1.0);
                    let tc = if *style == 1 { on_acc } else if *style == 2 { [255, 120, 110] } else { TEXT };
                    self.text_mid(&mut pm, Weight::Semibold, label, bx + 22.0, by + bh / 2.0, 14.5, tc);
                    self.hot.push(Hot { id: *id, x, y: oy, w: iw, h: ih });
                }
                Piece::Dial { id, label, v, lo, hi, val, unit } => {
                    let frac = ((v - lo) / (hi - lo)).clamp(0.0, 1.0);
                    let (dcx, dcy, r) = (x + iw / 2.0, oy + 52.0, 38.0);
                    let held = self.drag == Some(*id);
                    let hov = self.hover == Some(*id);
                    // the dial: a ring, the part that is "full" in the page's colour, and a knob at the end
                    rr(&mut pm, s, dcx - r - 6.0, dcy - r - 6.0, 2.0 * (r + 6.0), 2.0 * (r + 6.0), r + 6.0, if hov || held { CARD_HOVER } else { CARD }, 1.0);
                    self.arc(&mut pm, dcx, dcy, r, 135.0, 405.0, 7.0, EDGE, 1.0);
                    if frac > 0.002 {
                        self.arc(&mut pm, dcx, dcy, r, 135.0, 135.0 + 270.0 * frac, 7.0, acc, 1.0);
                    }
                    let ang = (135.0 + 270.0 * frac).to_radians();
                    let kr = if held { 8.5 } else { 6.5 };
                    rr(&mut pm, s, dcx + ang.cos() * r - kr, dcy + ang.sin() * r - kr, kr * 2.0, kr * 2.0, kr, [255, 255, 255], 1.0);
                    let vw = self.fonts.measure(Weight::Bold, val, 19.0 * s) / s;
                    self.text_mid(&mut pm, Weight::Bold, val, dcx - vw / 2.0, dcy - 4.0, 19.0, TEXT);
                    let uw = self.fonts.measure(Weight::Regular, unit, 11.5 * s) / s;
                    self.text_mid(&mut pm, Weight::Regular, unit, dcx - uw / 2.0, dcy + 14.0, 11.5, DIM);
                    let lw = self.fonts.measure(Weight::Regular, label, 13.0 * s) / s;
                    self.text_mid(&mut pm, Weight::Regular, label, x + iw / 2.0 - lw.min(iw) / 2.0, oy + 118.0, 13.0, TEXT);
                    self.hot.push(Hot { id: *id, x: dcx - r - 8.0, y: dcy - r - 8.0, w: 2.0 * (r + 8.0), h: 2.0 * (r + 8.0) });
                }
                Piece::Orb(n, name, chosen) => {
                    let t = &THEMES[(*n as usize).min(5)];
                    let id = Id::Theme(*n);
                    let hov = self.hover == Some(id);
                    let (ocx, ocy, r) = (x + iw / 2.0, oy + 28.0, if hov { 26.0 } else { 24.0 });
                    if *chosen {
                        rr(&mut pm, s, ocx - r - 4.0, ocy - r - 4.0, 2.0 * (r + 4.0), 2.0 * (r + 4.0), r + 4.0, acc, 1.0);
                    }
                    rr(&mut pm, s, ocx - r, ocy - r, 2.0 * r, 2.0 * r, r, t.win_bg, 1.0);
                    rr(&mut pm, s, ocx - r + 5.0, ocy - r + 5.0, 2.0 * r - 10.0, 2.0 * r - 10.0, r - 5.0, t.card, 1.0);
                    rr(&mut pm, s, ocx - 8.0, ocy - 8.0, 16.0, 16.0, 8.0, t.accent, 1.0);
                    let nw = self.fonts.measure(Weight::Regular, name, 12.0 * s) / s;
                    self.text_mid(&mut pm, Weight::Regular, name, ocx - nw / 2.0, oy + 68.0, 12.0, if *chosen { TEXT } else { DIM });
                    self.hot.push(Hot { id, x: ocx - 30.0, y: oy - 2.0, w: 60.0, h: 80.0 });
                }
            }
            x += iw + gap;
            line_h = line_h.max(ih);
        }
        let total = y + line_h + self.scroll + 40.0;
        self.max_scroll = (total - h).max(0.0);
        self.scroll = self.scroll.min(self.max_scroll);

        // ripples from presses
        for r in &self.ripples {
            let age = (self.t_anim - r.t0) / 0.55;
            let (rx, ry, rw, rh) = r.rect;
            let rad = age.sqrt() * rw.max(rh) * 0.5;
            if let (Some(mut mask), Some(clip), Some(circle)) = (Mask::new(pw, ph), crate::gfx::rrect(rx * s, ry * s, rw * s, rh * s, [rh * 0.5 * s; 4]), crate::gfx::rrect((r.x - rad) * s, (r.y - rad) * s, rad * 2.0 * s, rad * 2.0 * s, [rad * s; 4])) {
                mask.fill_path(&clip, tiny_skia::FillRule::Winding, true, Transform::identity());
                let mut paint = Paint::default();
                paint.set_color_rgba8(255, 255, 255, ((1.0 - age) * 60.0) as u8);
                paint.anti_alias = true;
                pm.fill_path(&circle, &paint, tiny_skia::FillRule::Winding, Transform::identity(), Some(&mask));
            }
        }

        // everything above the items stays where it is: cover what scrolled under it, then the bar, the live island, the dock
        self.restore_bg(&mut pm, 0, ((items_top - 4.0) * s) as u32);
        self.draw_logo(&mut pm, true);
        // Save / Discard (only with unsaved changes)
        if self.dirty() {
            let pulse = 0.5 + 0.5 * (self.t_anim * 4.0).sin();
            let mut bx = w - 24.0;
            for (id, label, primary) in [(Id::Save, "Save", true), (Id::Discard, "Discard", false)] {
                let tw = self.fonts.measure(Weight::Semibold, label, 14.0 * s) / s;
                let bw = tw + 36.0;
                bx -= bw;
                let hov = self.hover == Some(id);
                let down = hov && self.down == Some(id);
                let base = if primary { acc } else { CARD };
                let col = if down { mix(base, [0, 0, 0], 0.25) } else if hov { mix(base, [255, 255, 255], 0.14) } else { base };
                rr(&mut pm, s, bx, 12.0, bw, 32.0, 16.0, col, 1.0);
                self.text_mid(&mut pm, Weight::Semibold, label, bx + 18.0, 28.0, 14.0, if primary { on_acc } else { TEXT });
                self.hot.insert(0, Hot { id, x: bx, y: 12.0, w: bw, h: 32.0 });
                bx -= 10.0;
            }
            let msg = "Unsaved changes";
            let mw = self.fonts.measure(Weight::Regular, msg, 13.0 * s) / s;
            self.text_mid(&mut pm, Weight::Regular, msg, bx - mw - 4.0, 28.0, 13.0, TEXT);
            rr(&mut pm, s, bx - mw - 20.0, 24.0, 8.0, 8.0, 4.0, acc, 0.5 + 0.5 * pulse);
        }

        // the live island
        let sw = (w - 64.0).min(760.0);
        self.draw_stage(&mut pm, (w - sw) / 2.0, 62.0, sw, 232.0);
        // the little buttons that make it show things
        let mut bx = (w - sw) / 2.0;
        for (id, label, on) in [(Id::DemoNotif, "Notification", false), (Id::DemoStatus, "Status pill", false), (Id::DemoMax, "Maximized", self.demo_max), (Id::DemoFs, "Full-screen", self.demo_fs)] {
            let tw = self.fonts.measure(Weight::Semibold, label, 12.5 * s) / s;
            let bw = tw + 26.0;
            let hov = self.hover == Some(id);
            let down = hov && self.down == Some(id);
            let base = if on { acc } else { CARD };
            let col = if down { mix(base, [0, 0, 0], 0.25) } else if hov { mix(base, [255, 255, 255], 0.14) } else { base };
            rr(&mut pm, s, bx, 300.0, bw, 28.0, 14.0, col, 1.0);
            self.text_mid(&mut pm, Weight::Semibold, label, bx + 13.0, 314.0, 12.5, if on { on_acc } else { TEXT });
            self.hot.push(Hot { id, x: bx, y: 300.0, w: bw, h: 28.0 });
            bx += bw + 8.0;
        }

        // the dock: one round button per page; the chosen one is big and in the page's colour
        let n = PAGES.len() as f32;
        let (dgap, dd) = (30.0, 58.0);
        let total_w = n * dd + (n - 1.0) * dgap;
        let dx0 = (w - total_w) / 2.0;
        for (i, name) in PAGES.iter().enumerate() {
            let id = Id::Nav(i as u8);
            let col = self.cat_color(i as u8);
            let k = self.dock_k[i];
            let hov = self.hover == Some(id);
            let d = dd * (1.0 + 0.2 * k + if hov { 0.06 } else { 0.0 });
            let cx = dx0 + i as f32 * (dd + dgap) + dd / 2.0;
            let cy = 368.0 - 6.0 * k;
            let (fill, fg) = (mix(CARD, col, k), mix(DIM, if col[0] as f32 * 0.299 + col[1] as f32 * 0.587 + col[2] as f32 * 0.114 > 150.0 { [20, 20, 26] } else { [255, 255, 255] }, k));
            if k > 0.05 {
                rr(&mut pm, s, cx - d / 2.0 - 5.0, cy - d / 2.0 - 5.0, d + 10.0, d + 10.0, d / 2.0 + 5.0, col, 0.18 * k);
            }
            rr(&mut pm, s, cx - d / 2.0, cy - d / 2.0, d, d, d / 2.0, fill, 1.0);
            self.draw_glyph(&mut pm, i, cx, cy, d * 0.62, fg, fill);
            let lw = self.fonts.measure(Weight::Semibold, name, 12.5 * s) / s;
            self.text_mid(&mut pm, if k > 0.5 { Weight::Semibold } else { Weight::Regular }, name, cx - lw / 2.0, 414.0, 12.5, if k > 0.5 { TEXT } else { DIM });
            self.hot.push(Hot { id, x: cx - dd / 2.0 - 6.0, y: 368.0 - dd / 2.0 - 10.0, w: dd + 12.0, h: dd + 40.0 });
        }
        // what the thing under the mouse does
        let hint = match self.hover {
            Some(id) => self.hint_for(&rows, id),
            None => String::new(),
        };
        let hint = if hint.is_empty() { SUBTITLES[self.page as usize].to_string() } else { hint };
        let hint = self.fonts.fit(Weight::Regular, &hint, 13.5 * s, (cw - 20.0) * s);
        let hw = self.fonts.measure(Weight::Regular, &hint, 13.5 * s) / s;
        self.text_mid(&mut pm, Weight::Regular, &hint, w / 2.0 - hw / 2.0, 440.0, 13.5, DIM);
        let _ = (h, EDGE);

        self.pm = pm;
        &self.pm
    }

    /// An arc of a circle, from `a0` to `a1` degrees (0 = right, going clockwise on the screen), with round ends.
    #[allow(clippy::too_many_arguments)]
    fn arc(&self, pm: &mut Pixmap, cx: f32, cy: f32, r: f32, a0: f32, a1: f32, width: f32, rgb: [u8; 3], alpha: f32) {
        let s = self.scale;
        let mut pb = PathBuilder::new();
        let steps = (((a1 - a0).abs() / 4.0).ceil() as i32).max(2);
        for i in 0..=steps {
            let a = (a0 + (a1 - a0) * i as f32 / steps as f32).to_radians();
            let (px, py) = ((cx + a.cos() * r) * s, (cy + a.sin() * r) * s);
            if i == 0 {
                pb.move_to(px, py);
            } else {
                pb.line_to(px, py);
            }
        }
        if let Some(path) = pb.finish() {
            let mut paint = Paint::default();
            paint.set_color_rgba8(rgb[0], rgb[1], rgb[2], (alpha * 255.0) as u8);
            paint.anti_alias = true;
            let st = tiny_skia::Stroke { width: width * s, line_cap: tiny_skia::LineCap::Round, line_join: tiny_skia::LineJoin::Round, ..Default::default() };
            pm.stroke_path(&path, &paint, &st, Transform::identity(), None);
        }
    }

    /// The rows of a page as the pieces the page is made of: chips, dials, orbs and buttons.
    fn pieces(&self, rows: &[Row]) -> Vec<Piece> {
        let mut out = Vec::new();
        for r in rows {
            match r {
                Row::Toggle(id, title, _, on) => {
                    if *id == Id::FsHide {
                        out.push(Piece::Heading("Full-screen apps".into()));
                    }
                    out.push(Piece::Chip { id: *id, label: title.to_string(), on: *on });
                }
                Row::Slider(id, title, _, v, lo, hi, text) => {
                    let (val, unit) = match text.split_once(' ') {
                        Some((a, b)) => (a.to_string(), b.to_string()),
                        None => (text.clone(), String::new()),
                    };
                    out.push(Piece::Dial { id: *id, label: title.to_string(), v: *v, lo: *lo, hi: *hi, val, unit });
                }
                Row::Radio(title, _, chosen, _) => {
                    out.push(Piece::Heading(title.to_string()));
                    for (i, label) in ["Slim notch", "Hide", "Do nothing"].iter().enumerate() {
                        out.push(Piece::Chip { id: Id::MaxOpt(i as u8), label: label.to_string(), on: *chosen == i });
                    }
                }
                Row::Themes(chosen) => {
                    out.push(Piece::Heading("Theme".into()));
                    for (i, t) in THEMES.iter().enumerate() {
                        out.push(Piece::Orb(i as u8, t.name, *chosen as usize == i));
                    }
                }
                Row::Button(id, title, desc, label, style) => {
                    if *id == Id::CheckNow {
                        out.push(Piece::Text(desc.clone()));
                    }
                    let text = match id {
                        Id::Test => "Show a test notification".to_string(),
                        Id::Reset => "Reset all settings".to_string(),
                        Id::Quit => "Quit Bloop".to_string(),
                        Id::InstallFile => "Install from a file".to_string(),
                        _ => {
                            let _ = title;
                            label.clone()
                        }
                    };
                    out.push(Piece::Action { id: *id, label: text, style: *style });
                }
            }
        }
        out
    }

    /// The live island: the real island, drawn with the settings as they are right now. Hover it, click its buttons, open the player.
    /// It is drawn into its own small picture (with the wallpaper and the rounded corners made once), then placed in the window.
    fn draw_stage(&mut self, pm: &mut Pixmap, x: f32, y: f32, w: f32, h: f32) {
        let s = self.scale;
        self.stage = (x, y, w, h);
        let (pw, ph) = ((w * s).round().max(1.0) as u32, (h * s).round().max(1.0) as u32);
        if self.stage_pm.as_ref().map(|p| (p.width(), p.height())) != Some((pw, ph)) {
            // (re)build: the wallpaper, the corner mask, the picture itself
            let mut bg = Pixmap::new(pw, ph).unwrap();
            let rect = crate::gfx::rrect(0.0, 0.0, pw as f32, ph as f32, [10.0 * s; 4]);
            if let Some(path) = &rect {
                let mut paint = Paint::default();
                paint.shader = LinearGradient::new(
                    Point::from_xy(0.0, 0.0),
                    Point::from_xy(0.0, ph as f32),
                    vec![GradientStop::new(0.0, tiny_skia::Color::from_rgba8(30, 33, 48, 255)), GradientStop::new(1.0, tiny_skia::Color::from_rgba8(9, 9, 13, 255))],
                    SpreadMode::Pad,
                    Transform::identity(),
                )
                .unwrap_or(tiny_skia::Shader::SolidColor(tiny_skia::Color::BLACK));
                paint.anti_alias = true;
                bg.fill_path(path, &paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
            }
            let mut mask = Mask::new(pw, ph);
            if let (Some(m), Some(path)) = (mask.as_mut(), &rect) {
                m.fill_path(path, tiny_skia::FillRule::Winding, true, Transform::identity());
            }
            self.stage_bg = Some(bg);
            self.stage_mask = mask;
            self.stage_pm = Pixmap::new(pw, ph);
        }
        let (Some(mut st), Some(bg)) = (self.stage_pm.take(), self.stage_bg.as_ref()) else { return };
        st.data_mut().copy_from_slice(bg.data());
        // a maximized window under the island
        if self.demo_max {
            rr(&mut st, s, 0.0, 0.0, w, 34.0, 0.0, [52, 55, 70], 1.0);
            rr(&mut st, s, 14.0, 14.0, 70.0, 7.0, 3.5, [90, 94, 112], 1.0);
            for i in 0..3 {
                rr(&mut st, s, w - 24.0 - i as f32 * 20.0, 12.0, 10.0, 10.0, 5.0, [90, 94, 112], 1.0);
            }
        }
        // (the stage's top edge is the top edge of the screen: the island's own picture starts there)
        let island = self.preview.render();
        let ox = ((pw as f32 - island.width() as f32) / 2.0).round();
        blit_over(&mut st, island, ox as i32, 0);
        self.canvas_at = (x * s + ox, y * s);
        if self.preview.is_hidden() {
            let t = "Hidden while a full-screen app is open (move the mouse to the top edge to peek)";
            let tw = self.fonts.measure(Weight::Regular, t, 12.5 * s);
            self.fonts.draw(&mut st, Weight::Regular, t, (pw as f32) / 2.0 - tw / 2.0, ph as f32 / 2.0, 12.5 * s, [150, 150, 160], 1.0);
        }
        let hint = "Live preview - hover the island, press its buttons, click a long notification to unfold it";
        self.fonts.draw(&mut st, Weight::Regular, hint, 14.0 * s, (h - 12.0) * s, 11.5 * s, [120, 122, 134], 1.0);
        // the rounded corners: only the four corner squares need the mask
        if let Some(m) = &self.stage_mask {
            let cr = ((10.0 * s).ceil() as u32 + 2).min(pw / 2).min(ph / 2);
            let md = m.data();
            let sd = st.data_mut();
            for (cx0, cy0) in [(0, 0), (pw - cr, 0), (0, ph - cr), (pw - cr, ph - cr)] {
                for yy in cy0..cy0 + cr {
                    for xx in cx0..cx0 + cr {
                        let i = (yy * pw + xx) as usize;
                        let a = md[i] as u32;
                        if a < 255 {
                            for k in 0..4 {
                                sd[i * 4 + k] = ((sd[i * 4 + k] as u32 * a + 127) / 255) as u8;
                            }
                        }
                    }
                }
            }
        }
        blit_over(pm, &st, (x * s).round() as i32, (y * s).round() as i32);
        self.stage_pm = Some(st);
    }

    // ---------- mouse ----------

    fn hit_at(&self, x: f32, y: f32) -> Option<Id> {
        let (x, y) = (x / self.scale, y / self.scale);
        self.hot.iter().find(|r| x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h).map(|r| r.id)
    }

    pub fn hand(&self) -> bool {
        self.hover.is_some() || self.preview_hand
    }

    pub fn leave(&mut self) {
        if self.hover.is_some() {
            self.needs_full = true;
        }
        self.hover = None;
        self.preview.set_cursor(None);
        self.cursor_in = false;
    }

    /// Is the point over the live island's stage? Then the island's own coordinates.
    fn in_stage(&self, x: f32, y: f32) -> Option<(f32, f32)> {
        let (sx, sy, sw, sh) = self.stage;
        let k = self.scale;
        if x >= sx * k && x <= (sx + sw) * k && y >= sy * k && y <= (sy + sh) * k {
            Some((x - self.canvas_at.0, y - self.canvas_at.1))
        } else {
            None
        }
    }

    pub fn cancel(&mut self) {
        self.down = None;
        self.drag = None;
    }

    pub fn mouse_move(&mut self, x: f32, y: f32) -> Option<Act> {
        let before = self.hover;
        self.hover = self.hit_at(x, y);
        if before != self.hover || self.drag.is_some() {
            self.needs_full = true;
        }
        match self.in_stage(x, y) {
            Some((px, py)) => {
                self.stage_force = true;
                self.preview.set_cursor(Some((px, py)));
                self.preview.mouse_move(px);
                self.preview_hand = self.preview.wants_hand(px, py);
                self.cursor_in = true;
            }
            None => {
                if self.cursor_in {
                    self.preview.set_cursor(None);
                    self.cursor_in = false;
                    self.preview_hand = false;
                }
            }
        }
        if let Some(id) = self.drag {
            return self.set_slider(id, x, y);
        }
        None
    }

    pub fn mouse_down(&mut self, x: f32, y: f32) -> Option<Act> {
        self.needs_full = true;
        let id = self.hit_at(x, y);
        self.hover = id;
        self.down = id;
        if let Some(i) = id {
            if !matches!(i, Id::Nav(_) | Id::Quit | Id::Logo | Id::Save | Id::Discard) {
                if let Some(r) = self.hot.iter().find(|r| r.id == i) {
                    let rect = (r.x, r.y, r.w, r.h);
                    self.ripples.push(Ripple { x: x / self.scale, y: y / self.scale, t0: self.t_anim, rect });
                }
            }
        }
        if id.is_none() {
            if let Some((px, py)) = self.in_stage(x, y) {
                self.preview.mouse_down(px, py);
                self.preview_down = true;
            }
        }
        match id {
            Some(Id::Nav(n)) => {
                self.set_page(n);
                self.down = None;
                None
            }
            Some(id @ (Id::CardSecs | Id::Scale | Id::Speed | Id::Volume)) => {
                self.drag = Some(id);
                self.set_slider(id, x, y)
            }
            _ => None,
        }
    }

    pub fn mouse_up(&mut self, x: f32, y: f32) -> Option<Act> {
        self.needs_full = true;
        if self.preview_down {
            self.preview_down = false;
            let (px, py) = (x - self.canvas_at.0, y - self.canvas_at.1);
            match self.preview.mouse_up(px, py) {
                Hit::PlayPause => {
                    self.preview.toggle_playing_locally();
                    let s = if self.preview.is_playing() { crate::ui::Sound::Play } else { crate::ui::Sound::Pause };
                    self.preview.play_sound(s);
                }
                Hit::Prev | Hit::Next => self.preview.play_sound(crate::ui::Sound::Skip),
                Hit::Card(_) => {
                    if !self.preview.card_click() {
                        self.preview.dismiss_card();
                    }
                }
                Hit::Bubble => self.preview.dismiss_bubble(),
                Hit::LyricsToggle => {
                    self.preview.toggle_lyrics();
                    self.feed_lyrics();
                }
                _ => {}
            }
        }
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
                return Some(Act::Changed(self.s.clone()));
            }
            Id::Pill => s.show_music_pill = !s.show_music_pill,
            Id::PlayerFront => s.hide_when_player_front = !s.hide_when_player_front,
            Id::Lyrics => s.lyrics_button = !s.lyrics_button,
            Id::Eq => s.accent_eq = !s.accent_eq,
            Id::Notif => s.notifications = !s.notifications,
            Id::Bubble => s.use_bubble = !s.use_bubble,
            Id::FsHide => s.fullscreen_hide = !s.fullscreen_hide,
            Id::CheckUpd => s.check_updates = !s.check_updates,
            Id::StatusPills => s.status_pills = !s.status_pills,
            Id::Sounds => s.sounds = !s.sounds,
            Id::PlayerVolume => s.player_volume = !s.player_volume,
            Id::DndQuiet => s.dnd_quiet = !s.dnd_quiet,
            Id::Theme(n) => {
                if s.theme == n {
                    return None;
                }
                s.theme = n.min(5);
            }
            Id::MaxOpt(n) => {
                s.maximized = [MaxMode::Notch, MaxMode::Hide, MaxMode::Normal][n as usize % 3];
                if s.maximized == self.s.maximized {
                    return None;
                }
            }
            Id::Logo => {
                // a burst of bubbles, and the island answers with a notification
                self.burst = 0.0;
                self.demo_notification();
                return None;
            }
            Id::DemoNotif => {
                self.demo_notification();
                return None;
            }
            Id::DemoStatus => {
                self.demo_status();
                return None;
            }
            Id::DemoMax => {
                self.demo_max = !self.demo_max;
                self.sync_preview();
                return None;
            }
            Id::DemoFs => {
                self.demo_fs = !self.demo_fs;
                self.sync_preview();
                return None;
            }
            Id::Test => return Some(Act::Test),
            Id::CheckNow => return Some(if self.ready { Act::Install } else { Act::CheckNow }),
            Id::Reset => {
                // back to the defaults in the draft (applied when "Save" is pressed)
                let keep = s.update_repo.clone();
                s = Settings::default();
                s.update_repo = keep;
            }
            Id::Save => return Some(Act::Save(self.s.clone(), self.autostart)),
            Id::Discard => {
                self.discard();
                return Some(Act::Changed(self.s.clone()));
            }
            Id::Quit => return Some(Act::Quit),
            Id::InstallFile => return Some(Act::InstallFile),
            _ => return None,
        }
        self.s = s.clone();
        self.sync_preview();
        Some(Act::Changed(s))
    }

    fn set_slider(&mut self, id: Id, x: f32, y: f32) -> Option<Act> {
        let r = self.hot.iter().find(|r| r.id == id)?;
        // the dial: the angle of the mouse around its centre (from 7 o'clock, clockwise, 270 degrees)
        let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
        let (dx, dy) = (x / self.scale - cx, y / self.scale - cy);
        let theta = dy.atan2(dx).to_degrees().rem_euclid(360.0);
        let mut a = (theta - 135.0).rem_euclid(360.0);
        if a > 270.0 {
            a = if a < 315.0 { 270.0 } else { 0.0 };
        }
        let frac = a / 270.0;
        let (lo, hi) = match id {
            Id::CardSecs => (2.0, 20.0),
            Id::Scale => (80.0, 140.0),
            Id::Volume => (0.0, 100.0),
            _ => (50.0, 200.0),
        };
        let v = (lo + frac * (hi - lo)).round() as u32;
        let slot = match id {
            Id::CardSecs => &mut self.s.card_secs,
            Id::Scale => &mut self.s.scale_pct,
            Id::Volume => &mut self.s.volume,
            _ => &mut self.s.speed_pct,
        };
        if *slot == v {
            return None;
        }
        *slot = v;
        self.sync_preview();
        Some(Act::Changed(self.s.clone()))
    }

    /// The middle of a control, in window pixels (for tests and tools).
    pub fn center_of(&mut self, id: Id) -> Option<(f32, f32)> {
        self.render();
        let r = self.hot.iter().find(|r| r.id == id)?;
        Some(((r.x + r.w / 2.0) * self.scale, (r.y + r.h / 2.0) * self.scale))
    }

    /// Press the control `id` as if it was clicked (used by the preview tool and tests).
    pub fn press(&mut self, id: Id) -> Option<Act> {
        self.render();
        let r = self.hot.iter().find(|r| r.id == id)?;
        let (x, y) = ((r.x + r.w / 2.0) * self.scale, (r.y + r.h / 2.0) * self.scale);
        let _ = self.mouse_move(x, y);
        let down = self.mouse_down(x, y);
        let up = self.mouse_up(x, y);
        up.or(down)
    }

    pub fn wheel(&mut self, notches: f32) {
        self.needs_full = true;
        if self.cursor_in {
            self.preview.wheel(notches);
            return;
        }
        self.scroll = (self.scroll - notches * 56.0).clamp(0.0, self.max_scroll);
    }
}
