//! The settings window's contents, drawn by ourselves: a side list of pages on the left, the settings of the chosen page on the right,
//! like an ordinary Windows app. This file only knows about pixels and mouse positions; `settings_win.rs` puts it into a window.

use crate::gfx::{fill_rrect, Fonts, Weight};
use crate::settings::{MaxMode, Settings};
use crate::lyrics::{parse_lrc, Found};
use crate::ui::{ArtData, Env, Hit, MediaInfo, Need, NotifData, Ui};
use tiny_skia::{Color, GradientStop, LinearGradient, Mask, Paint, Pixmap, PixmapPaint, Point, SpreadMode, Transform};

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
}

/// A ripple that spreads from where a row or button was pressed.
struct Ripple {
    x: f32,
    y: f32,
    t0: f32,
    rect: (f32, f32, f32, f32),
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
    pub const DEFAULT_SIZE: (f32, f32) = (920.0, 740.0);
    pub const MIN_SIZE: (f32, f32) = (700.0, 460.0);

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
        let target = 84.0 + self.page as f32 * 42.0;
        let d = target - self.nav_y;
        if d.abs() > 0.2 {
            self.nav_y += d * (1.0 - (-dt * 16.0).exp());
            moving = true;
        } else {
            self.nav_y = target;
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
        if self.page_anim < 0.8 || self.pal_k < 1.0 || !self.ripples.is_empty() || self.burst >= 0.0 || (84.0 + self.page as f32 * 42.0 - self.nav_y).abs() > 0.2 {
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

    /// The top of the page list: the name, the version, and the little logo (a pill that now and then lets a bubble float up).
    /// Click it for a burst of bubbles.
    #[allow(non_snake_case, unused_variables)]
    fn draw_logo(&mut self, pm: &mut Pixmap, push_hot: bool) {
        let s = self.scale;
        let pal = self.pal();
        let (WIN_BG, TEXT, DIM, ACCENT) = (pal.win_bg, pal.text, pal.dim, pal.accent);
        rr(pm, s, 0.0, 0.0, SIDE, 84.0, 0.0, WIN_BG, 1.0);
        self.fonts.draw(pm, Weight::Semibold, "Bloop", 22.0 * s, 40.0 * s, 18.0 * s, TEXT, 1.0);
        self.fonts.draw(pm, Weight::Regular, &format!("Settings - version {}", crate::version::VERSION), 22.0 * s, 60.0 * s, 12.0 * s, DIM, 1.0);
        let t = self.t_anim;
        // the pill
        let (px, py, pw, ph) = (SIDE - 78.0, 24.0, 52.0, 24.0);
        let squash = if self.burst >= 0.0 { 1.0 + 0.12 * (self.burst * 14.0).sin() * (-self.burst * 4.0).exp() } else { 1.0 };
        let (qw, qh) = (pw * squash, ph / squash);
        let (qx, qy) = (px + (pw - qw) / 2.0, py + (ph - qh) / 2.0);
        rr(pm, s, qx, qy, qw, qh, qh / 2.0, [0, 0, 0], 1.0);
        rr(pm, s, qx + 5.0, qy + qh / 2.0 - 7.0, 14.0, 14.0, 4.5, [255, 138, 76], 1.0);
        rr(pm, s, qx + 8.0, qy + qh / 2.0 - 4.0, 8.0, 8.0, 3.0, [255, 80, 150], 0.85);
        for i in 0..3 {
            let h = 4.0 + 6.0 * (0.5 + 0.5 * (t * (5.0 + i as f32 * 1.7) + i as f32 * 1.3).sin());
            rr(pm, s, qx + qw - 20.0 + i as f32 * 5.0, qy + qh / 2.0 - h / 2.0, 2.4, h, 1.2, [255, 255, 255], 0.9);
        }
        // a bubble leaves the pill every few seconds, floats up and pops
        let p = (t % 3.8) / 3.8;
        if p < 0.55 {
            let q = p / 0.55;
            let (bx, by) = (qx + qw + 2.0 + q * 12.0, qy + qh / 2.0 - q * 24.0);
            let r = 2.0 + 3.6 * (std::f32::consts::PI * (q * 0.9).min(1.0)).sin().max(0.2) * (1.0 - q * 0.3);
            let a = if q > 0.82 { 1.0 - (q - 0.82) / 0.18 } else { 1.0 };
            rr(pm, s, bx - r, by - r, r * 2.0, r * 2.0, r, ACCENT, 0.9 * a);
            if q > 0.8 {
                let ring = (q - 0.8) / 0.2;
                let rr_ = r + ring * 6.0;
                rr(pm, s, bx - rr_, by - rr_, rr_ * 2.0, rr_ * 2.0, rr_, ACCENT, 0.25 * (1.0 - ring));
                rr(pm, s, bx - rr_ + 1.2, by - rr_ + 1.2, rr_ * 2.0 - 2.4, rr_ * 2.0 - 2.4, rr_ - 1.2, WIN_BG, 0.25 * (1.0 - ring));
            }
        }
        // the burst
        if self.burst >= 0.0 {
            let b = (self.burst / 0.9).min(1.0);
            for i in 0..9 {
                let ang = i as f32 / 9.0 * std::f32::consts::TAU + 0.4;
                let d = 10.0 + 34.0 * (1.0 - (1.0 - b).powi(3));
                let r = 4.5 * (1.0 - b * 0.7);
                let (bx, by) = (px + pw / 2.0 + ang.cos() * d * 1.5, py + ph / 2.0 + ang.sin() * d);
                rr(pm, s, bx - r, by - r, r * 2.0, r * 2.0, r, if i % 2 == 0 { ACCENT } else { [255, 138, 76] }, 0.85 * (1.0 - b));
            }
        }
        if push_hot {
            self.hot.push(Hot { id: Id::Logo, x: px - 8.0, y: 8.0, w: pw + 40.0, h: 66.0 });
        }
    }

    pub fn render(&mut self) -> &Pixmap {
        self.needs_full = false;
        let pal = self.pal();
        let (WIN_BG, PAGE_BG, CARD, CARD_HOVER, EDGE, TEXT, DIM, ACCENT, ON_ACCENT, NAV_ON, NAV_HOVER) = (pal.win_bg, pal.page_bg, pal.card, pal.card_hover, pal.edge, pal.text, pal.dim, pal.accent, pal.on_accent, pal.nav_on, pal.nav_hover);
        let (BTN, TRACK) = (mix(pal.card, pal.text, 0.14), mix(pal.card, pal.text, 0.34));
        let s = self.scale;
        let mut pm = std::mem::replace(&mut self.pm, Pixmap::new(1, 1).unwrap());
        pm.fill(Color::from_rgba8(PAGE_BG[0], PAGE_BG[1], PAGE_BG[2], 255));
        self.hot.clear();
        let (w, h) = (self.w, self.h);

        // ---- the list of pages ----
        rr(&mut pm, s, 0.0, 0.0, SIDE, h, 0.0, WIN_BG, 1.0);
        self.draw_logo(&mut pm, true);
        // the highlight glides to the chosen page
        rr(&mut pm, s, 10.0, self.nav_y, SIDE - 20.0, 38.0, 5.0, NAV_ON, 1.0);
        rr(&mut pm, s, 10.0, self.nav_y + 11.0, 3.0, 16.0, 1.5, ACCENT, 1.0);
        for (i, name) in PAGES.iter().enumerate() {
            let id = Id::Nav(i as u8);
            let (x, y, bw, bh) = (10.0, 84.0 + i as f32 * 42.0, SIDE - 20.0, 38.0);
            let on = self.page == i as u8;
            let hov = self.hover == Some(id);
            if !on && hov {
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
            self.text_mid(&mut pm, Weight::Regular, "Quit Bloop", x + 18.0, y + bh / 2.0, 14.0, if pal.light { [200, 40, 40] } else { [255, 138, 128] });
            self.hot.push(Hot { id: Id::Quit, x, y, w: bw, h: bh });
        }

        // ---- the settings of the page ----
        let cx = SIDE + 36.0;
        let cw = (w - SIDE - 72.0).clamp(300.0, 760.0);
        const ROWS_TOP: f32 = 348.0;
        let mut y = ROWS_TOP - self.scroll;
        let rows = self.rows();
        for (i, row) in rows.iter().enumerate() {
            let rh = row.height();
            // the rows slide up into place one after another when the page changes
            let k = ((self.page_anim - 0.045 * i as f32) / 0.3).clamp(0.0, 1.0);
            let off = 26.0 * (1.0 - k).powi(3);
            self.draw_row(&mut pm, row, cx, y + off, cw, rh);
            y += rh + 6.0;
        }
        // ripples from presses
        for r in &self.ripples {
            let age = (self.t_anim - r.t0) / 0.55;
            let (rx, ry, rw, rh) = r.rect;
            let rad = age.sqrt() * rw.max(rh) * 0.45;
            if let (Some(mut mask), Some(clip), Some(circle)) = (Mask::new(pm.width(), pm.height()), crate::gfx::rrect(rx * s, ry * s, rw * s, rh * s, [6.0 * s; 4]), crate::gfx::rrect((r.x - rad) * s, (r.y - rad) * s, rad * 2.0 * s, rad * 2.0 * s, [rad * s; 4])) {
                mask.fill_path(&clip, tiny_skia::FillRule::Winding, true, Transform::identity());
                let mut paint = Paint::default();
                paint.set_color_rgba8(ACCENT[0], ACCENT[1], ACCENT[2], ((1.0 - age) * 34.0) as u8);
                paint.anti_alias = true;
                pm.fill_path(&circle, &paint, tiny_skia::FillRule::Winding, Transform::identity(), Some(&mask));
            }
        }
        let total = y + self.scroll + 90.0;
        self.max_scroll = (total - h).max(0.0);
        self.scroll = self.scroll.min(self.max_scroll);

        // everything above the rows stays where it is: the live island, the title and the little buttons
        rr(&mut pm, s, SIDE, 0.0, w - SIDE, ROWS_TOP - 8.0, 0.0, PAGE_BG, 1.0);
        self.draw_stage(&mut pm, cx, 16.0, cw, 246.0);
        self.fonts.draw(&mut pm, Weight::Semibold, PAGES[self.page as usize], cx * s, 300.0 * s, 26.0 * s, TEXT, 1.0);
        self.fonts.draw(&mut pm, Weight::Regular, SUBTITLES[self.page as usize], cx * s, 324.0 * s, 13.0 * s, DIM, 1.0);
        // the buttons that make the preview show things
        let mut bx = cx + cw;
        for (id, label, on) in [(Id::DemoFs, "Full-screen", self.demo_fs), (Id::DemoMax, "Maximized", self.demo_max), (Id::DemoStatus, "Status pill", false), (Id::DemoNotif, "Notification", false)] {
            let tw = self.fonts.measure(Weight::Semibold, label, 12.5 * s) / s;
            let bw = tw + 26.0;
            bx -= bw;
            let hov = self.hover == Some(id);
            let down = hov && self.down == Some(id);
            let base = if on { ACCENT } else { BTN };
            let col = if down { mix(base, [0, 0, 0], 0.22) } else if hov { mix(base, [255, 255, 255], 0.12) } else { base };
            rr(&mut pm, s, bx, 276.0, bw, 28.0, 14.0, col, 1.0);
            self.text_mid(&mut pm, Weight::Semibold, label, bx + 13.0, 290.0, 12.5, if on { ON_ACCENT } else { TEXT });
            self.hot.push(Hot { id, x: bx, y: 276.0, w: bw, h: 28.0 });
            bx -= 8.0;
        }
        // unsaved changes: a bar at the bottom with Discard / Save
        if self.dirty() {
            let bh = 64.0;
            rr(&mut pm, s, SIDE, h - bh, w - SIDE, bh, 0.0, CARD, 1.0);
            rr(&mut pm, s, SIDE, h - bh, w - SIDE, 1.0, 0.0, EDGE, 1.0);
            rr(&mut pm, s, cx, h - bh / 2.0 - 4.0, 8.0, 8.0, 4.0, ACCENT, 1.0);
            let msg = self.fonts.fit(Weight::Regular, "Unsaved changes - not applied until you save", 13.0 * s, (cw - 290.0).max(80.0) * s);
            self.text_mid(&mut pm, Weight::Regular, &msg, cx + 18.0, h - bh / 2.0, 13.0, TEXT);
            let mut bx = cx + cw;
            for (id, label, primary) in [(Id::Save, "Save", true), (Id::Discard, "Discard", false)] {
                let bw = if primary { 96.0 } else { 90.0 };
                bx -= bw;
                let hov = self.hover == Some(id);
                let down = hov && self.down == Some(id);
                let base = if primary { ACCENT } else { BTN };
                let col = if down { mix(base, [0, 0, 0], 0.22) } else if hov { mix(base, [255, 255, 255], 0.12) } else { base };
                rr(&mut pm, s, bx, h - bh / 2.0 - 16.0, bw, 32.0, 5.0, col, 1.0);
                let tw = self.fonts.measure(Weight::Semibold, label, 13.5 * s) / s;
                self.text_mid(&mut pm, Weight::Semibold, label, bx + bw / 2.0 - tw / 2.0, h - bh / 2.0, 13.5, if primary { ON_ACCENT } else { TEXT });
                self.hot.insert(0, Hot { id, x: bx, y: h - bh / 2.0 - 16.0, w: bw, h: 32.0 });
                bx -= 10.0;
            }
        }
        // a thin scroll bar
        if self.max_scroll > 0.0 {
            let vh = h - ROWS_TOP;
            let th = (vh * (h - ROWS_TOP) / (total - ROWS_TOP)).max(30.0);
            let ty = ROWS_TOP + (vh - th) * (self.scroll / self.max_scroll);
            rr(&mut pm, s, w - 8.0, ty, 3.0, th, 1.5, TRACK, 0.9);
        }
        self.pm = pm;
        &self.pm
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

    #[allow(non_snake_case, unused_variables)]
    fn draw_row(&mut self, pm: &mut Pixmap, row: &Row, cx: f32, y: f32, cw: f32, rh: f32) {
        let pal = self.pal();
        let (WIN_BG, PAGE_BG, CARD, CARD_HOVER, EDGE, TEXT, DIM, ACCENT, ON_ACCENT, NAV_ON, NAV_HOVER) = (pal.win_bg, pal.page_bg, pal.card, pal.card_hover, pal.edge, pal.text, pal.dim, pal.accent, pal.on_accent, pal.nav_on, pal.nav_hover);
        let (BTN, TRACK) = (mix(pal.card, pal.text, 0.14), mix(pal.card, pal.text, 0.34));
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
                    rr(pm, s, tx, ty, tw, th, th / 2.0, TRACK, 1.0 - ease * 2.0);
                    rr(pm, s, tx + 1.2, ty + 1.2, tw - 2.4, th - 2.4, th / 2.0 - 1.2, mix(CARD, CARD_HOVER, 0.0), 1.0 - ease * 2.0);
                }
                rr(pm, s, tx, ty, tw, th, th / 2.0, ACCENT, ease);
                // the knob stretches while it travels, like a drop of water (and settles with a round dot)
                let d = 12.0 + ease * 2.0;
                let stretch = (std::f32::consts::PI * t).sin() * 9.0;
                let kx = tx + 4.0 + ease * (tw - 8.0 - 14.0) + (14.0 - d) / 2.0 - stretch * ease;
                rr(pm, s, kx, ty + (th - d) / 2.0, d + stretch, d, d / 2.0, mix(TRACK, ON_ACCENT, ease), 1.0);
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
                rr(pm, s, sx, ty - 2.0, sw, 4.0, 2.0, TRACK, 1.0);
                rr(pm, s, sx, ty - 2.0, sw * frac, 4.0, 2.0, ACCENT, 1.0);
                let held = self.drag == Some(*id);
                let hov = self.hover == Some(*id);
                let kx = sx + sw * frac;
                rr(pm, s, kx - 10.0, ty - 10.0, 20.0, 20.0, 10.0, mix(CARD, TEXT, 0.25), 1.0);
                                let d = if held { 8.0 } else if hov { 14.0 } else { 12.0 };
                rr(pm, s, kx - d / 2.0, ty - d / 2.0, d, d, d / 2.0, ACCENT, 1.0);
                let tw = self.fonts.measure(Weight::Regular, text, 14.0 * s) / s;
                self.text_mid(pm, Weight::Regular, text, right - tw, ty, 14.0, TEXT);
                // while it is dragged, the value pops up above the knob
                if held {
                    let bw = self.fonts.measure(Weight::Semibold, text, 12.5 * s) / s + 18.0;
                    rr(pm, s, kx - bw / 2.0, ty - 44.0, bw, 24.0, 12.0, ACCENT, 1.0);
                    rr(pm, s, kx - 4.0, ty - 22.0, 8.0, 8.0, 2.0, ACCENT, 1.0);
                    self.text_mid(pm, Weight::Semibold, text, kx - bw / 2.0 + 9.0, ty - 32.0, 12.5, ON_ACCENT);
                }
                self.hot.push(Hot { id: *id, x: sx - 12.0, y, w: sw + 24.0, h: rh });
            }
            Row::Themes(chosen) => {
                card(self, pm, None);
                self.text_mid(pm, Weight::Regular, "Theme", cx + 18.0, y + 20.0, 14.5, TEXT);
                self.fonts.draw(pm, Weight::Regular, "Colours of this window (the island itself stays black)", (cx + 18.0) * s, (y + 42.0) * s, 12.5 * s, DIM, 1.0);
                let n = THEMES.len() as f32;
                let gap = 10.0;
                let bw = ((cw - 36.0 - gap * (n - 1.0)) / n).min(110.0);
                for (i, t) in THEMES.iter().enumerate() {
                    let id = Id::Theme(i as u8);
                    let bx = cx + 18.0 + i as f32 * (bw + gap);
                    let by = y + 56.0;
                    let on = *chosen as usize == i;
                    let hov = self.hover == Some(id);
                    // a small sample of the theme: its window, a card and the accent
                    if on {
                        rr(pm, s, bx - 2.5, by - 2.5, bw + 5.0, 56.0 + 5.0, 9.5, ACCENT, 1.0);
                    }
                    rr(pm, s, bx, by, bw, 56.0, 7.0, if hov && !on { t.edge } else { t.page_bg }, 1.0);
                    rr(pm, s, bx, by, bw, 56.0, 7.0, t.win_bg, 1.0);
                    rr(pm, s, bx + 6.0, by + 7.0, bw - 12.0, 22.0, 4.0, t.card, 1.0);
                    rr(pm, s, bx + 10.0, by + 14.0, bw * 0.38, 4.0, 2.0, t.text, 0.9);
                    rr(pm, s, bx + 10.0, by + 21.0, bw * 0.24, 3.0, 1.5, t.dim, 0.9);
                    rr(pm, s, bx + bw - 28.0, by + 14.0, 16.0, 8.0, 4.0, t.accent, 1.0);
                    rr(pm, s, bx + 6.0, by + 36.0, bw - 12.0, 12.0, 4.0, t.accent, 1.0);
                    let tw = self.fonts.measure(Weight::Semibold, t.name, 12.0 * s) / s;
                    self.text_mid(pm, Weight::Semibold, t.name, bx + bw / 2.0 - tw / 2.0, by + 36.0 + 6.0, 11.0, t.on_accent);
                    self.hot.push(Hot { id, x: bx, y: by, w: bw, h: 56.0 });
                }
            }
            Row::Button(id, title, desc, label, style) => {
                card(self, pm, None);
                self.title_desc(pm, title, desc, cx + 18.0, y, rh, cw - 190.0);
                let (bw, bh) = (116.0, 32.0);
                let (bx, by) = (right - bw, y + rh / 2.0 - bh / 2.0);
                let hov = self.hover == Some(*id);
                let pressed = hov && self.down == Some(*id);
                let base = if *style == 1 { ACCENT } else { BTN };
                let col = if pressed { mix(base, [0, 0, 0], 0.22) } else if hov { mix(base, [255, 255, 255], 0.12) } else { base };
                rr(pm, s, bx, by, bw, bh, 5.0, col, 1.0);
                let tcol = match style {
                    1 => ON_ACCENT,
                    2 => if pal.light { [200, 40, 40] } else { [255, 138, 128] },
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
                        rr(pm, s, rx + 6.0, ry - 4.0, 8.0, 8.0, 4.0, ON_ACCENT, 1.0);
                    } else {
                        rr(pm, s, rx, ry - 10.0, 20.0, 20.0, 10.0, TRACK, 1.0);
                        rr(pm, s, rx + 1.4, ry - 8.6, 17.2, 17.2, 8.6, CARD, 1.0);
                    }
                    self.text_mid(pm, Weight::Regular, name, rx + 34.0, ry, 14.0, TEXT);
                    self.hot.push(Hot { id, x: cx, y: oy, w: cw, h: 42.0 });
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(non_snake_case)]
    fn title_desc(&mut self, pm: &mut Pixmap, title: &str, desc: &str, x: f32, y: f32, rh: f32, max_w: f32) {
        let s = self.scale;
        let (TEXT, DIM) = (self.pal().text, self.pal().dim);
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
            return self.set_slider(id, x);
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
                self.set_slider(id, x)
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

    fn set_slider(&mut self, id: Id, x: f32) -> Option<Act> {
        let r = self.hot.iter().find(|r| r.id == id)?;
        let (sx, sw) = (r.x + 12.0, r.w - 24.0);
        let frac = ((x / self.scale - sx) / sw).clamp(0.0, 1.0);
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
