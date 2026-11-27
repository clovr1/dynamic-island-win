//! Renders the island to PNG files (no window needed): `cargo run --release --bin preview`

use island::ui::{ArtData, Env, MediaInfo, NotifData, Ui, TOP, W};
use tiny_skia::{Color, Paint, Pixmap, PixmapPaint, Rect, Transform};

const SCALE: f32 = 2.0;

fn gradient(a: [u8; 3], b: [u8; 3], n: u32) -> ArtData {
    let mut rgba = Vec::new();
    for y in 0..n {
        for x in 0..n {
            let t = (x + y) as f32 / (2 * n) as f32;
            rgba.extend_from_slice(&[(a[0] as f32 * (1.0 - t) + b[0] as f32 * t) as u8, (a[1] as f32 * (1.0 - t) + b[1] as f32 * t) as u8, (a[2] as f32 * (1.0 - t) + b[2] as f32 * t) as u8, 255]);
        }
    }
    ArtData { w: n, h: n, rgba }
}

/// a white speech-bubble glyph on a transparent background (like most Windows app logos)
fn glyph(n: u32) -> ArtData {
    let mut rgba = vec![0u8; (n * n * 4) as usize];
    for y in 0..n {
        for x in 0..n {
            let (dx, dy) = (x as f32 - n as f32 / 2.0, y as f32 - n as f32 / 2.0);
            if dx * dx + dy * dy < (n as f32 * 0.32).powi(2) {
                let i = ((y * n + x) * 4) as usize;
                rgba[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
    }
    ArtData { w: n, h: n, rgba }
}

struct Rec {
    ui: Ui,
    bg: Pixmap,
    shots: Vec<(String, Pixmap)>,
    t: f32,
}

impl Rec {
    fn new() -> Rec {
        let ui = Ui::new(SCALE);
        let (pw, ph) = ui.size_px();
        let mut bg = Pixmap::new(pw, ph).unwrap();
        bg.fill(Color::from_rgba8(60, 64, 80, 255));
        let mut p = Paint::default();
        p.set_color_rgba8(30, 33, 46, 255);
        bg.fill_rect(Rect::from_xywh(0.0, 0.0, pw as f32, 4.0 * SCALE).unwrap(), &p, Transform::identity(), None);
        Rec { ui, bg, shots: Vec::new(), t: 0.0 }
    }
    /// run the animation forward and take a picture at each of these times (ms since `reset`)
    fn at(&mut self, label: &str, times: &[u32]) {
        for &ms in times {
            while self.t * 1000.0 < ms as f32 {
                self.ui.tick(1.0 / 60.0);
                self.t += 1.0 / 60.0;
            }
            let mut out = self.bg.clone();
            out.draw_pixmap(0, 0, self.ui.render().as_ref(), &PixmapPaint::default(), Transform::identity(), None);
            self.shots.push((format!("{label} +{ms}ms"), out));
        }
    }
    fn reset(&mut self) {
        self.t = 0.0;
    }
    fn hover(&mut self, on: bool) {
        self.ui.set_cursor(if on { Some((W / 2.0 * SCALE, (TOP + 18.0) * SCALE)) } else { None });
    }
}

fn main() {
    let mut r = Rec::new();
    let music = |playing| MediaInfo { active: true, title: "Gravity".into(), artist: "Beach Bunny".into(), playing, pos: 75.0, dur: 200.0 };
    let notif = |app: &str, title: &str, body: &str, icon: ArtData| NotifData { app: app.into(), aumid: String::new(), title: title.into(), body: body.into(), icon: Some(icon) };

    r.at("idle", &[0]);
    // a notification with nothing playing: a card
    r.ui.push_notification(notif("Messages", "Budi", "Are you coming tonight? We are at the usual place", gradient([40, 200, 100], [20, 140, 90], 96)));
    r.reset();
    r.at("card", &[60, 200, 500, 1200]);
    // a Windows-style glyph logo (white on transparent)
    for _ in 0..400 { r.ui.tick(1.0 / 60.0); }
    r.ui.push_notification(notif("Windows Security", "Microsoft Defender summary", "Microsoft Defender Antivirus did not find any threats", glyph(96)));
    r.reset();
    r.at("glyph card", &[700]);
    for _ in 0..600 { r.ui.tick(1.0 / 60.0); }
    // music + a bubble beside the pill
    r.ui.set_art(Some(gradient([255, 157, 58], [106, 75, 216], 128)));
    r.ui.set_media(music(true));
    for _ in 0..90 { r.ui.tick(1.0 / 60.0); }
    r.ui.push_notification(notif("Instagram", "sara", "sent you a reel", gradient([253, 89, 73], [214, 36, 159], 96)));
    r.reset();
    r.at("bubble", &[80, 200, 400, 900]);
    // hover the bubble: it grows into the card
    r.ui.push_notification(notif("Instagram", "sara", "and another one", gradient([253, 89, 73], [214, 36, 159], 96)));
    for _ in 0..60 { r.ui.tick(1.0 / 60.0); }
    // a maximized window: notch
    r.ui.set_env(Env { maximized: true, ..Env::default() });
    r.reset();
    r.at("maximized -> notch", &[60, 140, 260, 600]);
    r.ui.push_notification(notif("Mail", "Rina", "Meeting moved to 3 PM", gradient([60, 140, 255], [30, 90, 200], 96)));
    r.reset();
    r.at("notch + notification", &[120, 300, 700]);
    for _ in 0..720 { r.ui.tick(1.0 / 60.0); }
    // hover the notch: it grows into an enlarged notch that hangs from the top edge
    r.ui.set_cursor(Some((W / 2.0 * SCALE, 8.0 * SCALE)));
    r.reset();
    r.at("hover the notch", &[60, 140, 220, 320, 500, 900]);
    r.ui.set_cursor(None);
    r.reset();
    r.at("cursor leaves: retracts", &[320, 420, 520, 640, 1000]);
    // a notification while maximized: the card hangs from the edge, too
    r.ui.push_notification(notif("Mail", "Dina", "Can you send me the slides?", gradient([60, 140, 255], [30, 90, 200], 96)));
    r.reset();
    r.at("card under a maximized window", &[150, 400, 900]);
    for _ in 0..600 { r.ui.tick(1.0 / 60.0); }
    // full screen: the island tucks away
    r.ui.set_env(Env { fullscreen: true, ..Env::default() });
    r.reset();
    r.at("full screen -> tucked", &[60, 160, 300, 700]);
    r.ui.set_env(Env { fullscreen: true, peek: true, ..Env::default() });
    r.reset();
    r.at("peek (cursor at the top edge)", &[100, 250, 700]);
    r.ui.set_env(Env::default());
    r.reset();
    r.at("back to a normal window", &[100, 300, 800]);

    let (pw, ph) = r.ui.size_px();
    let crop_h = (190.0 * SCALE) as u32;
    let cols = 3u32;
    let rows = (r.shots.len() as u32 + cols - 1) / cols;
    let mut sheet = Pixmap::new(pw * cols, crop_h * rows).unwrap();
    sheet.fill(Color::from_rgba8(20, 20, 24, 255));
    for (i, (name, pm)) in r.shots.iter().enumerate() {
        sheet.draw_pixmap(((i as u32 % cols) * pw) as i32, ((i as u32 / cols) * crop_h) as i32, pm.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
        println!("{:>2}: {}", i, name);
    }
    std::fs::create_dir_all("preview").unwrap();
    sheet.save_png("preview/sheet2.png").unwrap();
    let _ = ph;
    println!("wrote preview/sheet2.png ({}x{})", pw * cols, crop_h * rows);
}
