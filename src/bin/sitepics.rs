//! The pictures for the website (site/img/*.png), drawn by the real island and settings window. Run: cargo run --release --bin sitepics
use island::panel::Panel;
use island::settings::Settings;
use island::ui::{ArtData, MediaInfo, NotifData, StatusData, Ui, TOP, W};
use tiny_skia::{Pixmap, PixmapPaint, Transform};

const SCALE: f32 = 2.0;

fn art(a: [u8; 3], b: [u8; 3], n: u32) -> ArtData {
    let mut rgba = Vec::new();
    for y in 0..n {
        for x in 0..n {
            let t = (x + y) as f32 / (2 * n) as f32;
            for k in 0..3 {
                rgba.push((a[k] as f32 * (1.0 - t) + b[k] as f32 * t) as u8);
            }
            rgba.push(255);
        }
    }
    ArtData { w: n, h: n, rgba }
}

/// a soft dark wallpaper
fn wallpaper(w: u32, h: u32, hue: u8) -> Pixmap {
    let mut pm = Pixmap::new(w, h).unwrap();
    let (c1, c2): ([f32; 3], [f32; 3]) = match hue {
        0 => ([70.0, 40.0, 120.0], [20.0, 90.0, 140.0]),
        1 => ([140.0, 60.0, 90.0], [40.0, 50.0, 120.0]),
        _ => ([30.0, 100.0, 110.0], [90.0, 50.0, 130.0]),
    };
    let data = pm.data_mut();
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f32 / w as f32, y as f32 / h as f32);
            let g1 = (1.0 - ((fx - 0.2).powi(2) + (fy - 0.1).powi(2)).sqrt() * 1.6).max(0.0);
            let g2 = (1.0 - ((fx - 0.85).powi(2) + (fy - 0.9).powi(2)).sqrt() * 1.5).max(0.0);
            let i = ((y * w + x) * 4) as usize;
            for k in 0..3 {
                data[i + k] = (14.0 + c1[k] * g1 * 0.7 + c2[k] * g2 * 0.7 + fy * 8.0).min(255.0) as u8;
            }
            data[i + 3] = 255;
        }
    }
    pm
}

fn run(ui: &mut Ui, secs: f32) {
    for _ in 0..(secs * 60.0) as u32 {
        ui.tick(1.0 / 60.0);
    }
}

fn new_ui() -> Ui {
    let mut ui = Ui::new(SCALE);
    ui.set_settings(&Settings::default());
    ui.set_clock("Friday, 27 November  14:05".into());
    ui
}

fn shot(ui: &mut Ui, hue: u8, crop_h: f32) -> Pixmap {
    let (pw, _) = ui.size_px();
    let ch = (crop_h * SCALE) as u32;
    let mut bg = wallpaper(pw, ch, hue);
    bg.draw_pixmap(0, 0, ui.render().as_ref(), &PixmapPaint::default(), Transform::identity(), None);
    bg
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "site/img".into());
    std::fs::create_dir_all(&out).unwrap();
    let music = |playing| MediaInfo { active: true, title: "Low Tide".into(), artist: "Paper Boats".into(), playing, pos: 75.0, dur: 200.0 };
    let cover = || art([255, 157, 58], [106, 75, 216], 128);

    // music pill
    let mut ui = new_ui();
    ui.set_art(Some(cover()));
    ui.set_media(music(true));
    run(&mut ui, 2.0);
    shot(&mut ui, 0, 80.0).save_png(format!("{out}/pill.png")).unwrap();
    // the island alone, on a transparent ground (for the pictures of the tutorial)
    ui.render().save_png(format!("{out}/raw-pill.png")).unwrap();

    // the open player
    ui.set_cursor(Some((W / 2.0 * SCALE, (TOP + 18.0) * SCALE)));
    run(&mut ui, 2.0);
    shot(&mut ui, 0, 230.0).save_png(format!("{out}/player.png")).unwrap();

    // lyrics (karaoke)
    let mut ui = new_ui();
    let mut s = Settings::default();
    s.lyrics_anim = 1;
    ui.set_settings(&s);
    ui.set_art(Some(cover()));
    ui.set_media(MediaInfo { pos: 23.0, ..music(true) });
    ui.set_cursor(Some((W / 2.0 * SCALE, (TOP + 18.0) * SCALE)));
    run(&mut ui, 2.0);
    ui.toggle_lyrics();
    if let Some(req) = ui.take_lyrics_request() {
        let lrc = "[00:08.00]Streetlights hum a quiet tune\n[00:14.00]Every window painted blue\n[00:20.00]Take the long way home with me\n[00:26.00]Let the whole city fade away\n[00:32.00]Tide pulls out, night rolls in";
        ui.set_lyrics(&req.key, Some(island::lyrics::Found { synced: true, lines: island::lyrics::parse_lrc(lrc) }));
    }
    run(&mut ui, 3.0);
    shot(&mut ui, 1, 230.0).save_png(format!("{out}/lyrics.png")).unwrap();

    // a notification card
    let mut ui = new_ui();
    ui.push_notification(NotifData { app: "Messages".into(), aumid: String::new(), title: "Budi".into(), body: "Are you coming tonight? We are at the usual place".into(), icon: Some(art([40, 200, 100], [20, 140, 90], 96)) });
    run(&mut ui, 1.4);
    shot(&mut ui, 2, 110.0).save_png(format!("{out}/card.png")).unwrap();

    // status pills: Wi-Fi, a speaker, a controller, charging
    let mut tiles: Vec<Pixmap> = Vec::new();
    for st in [
        StatusData::Sys { key: "wifi".into(), on: true },
        StatusData::Device { name: "JBL Flip 6".into(), kind: island::deviceicons::Kind::Speaker, connected: true },
        StatusData::Device { name: "Xbox Wireless Controller".into(), kind: island::deviceicons::Kind::Controller, connected: true },
        StatusData::Battery { label: "Charging".into(), level: 64, charging: true, low: false },
    ] {
        let mut ui = new_ui();
        ui.push_status(st);
        run(&mut ui, 1.0);
        tiles.push(shot(&mut ui, 1, 90.0));
    }
    let (tw, th) = (tiles[0].width(), tiles[0].height());
    let mut sheet = Pixmap::new(tw, th * tiles.len() as u32).unwrap();
    for (i, t) in tiles.iter().enumerate() {
        sheet.draw_pixmap(0, (i as u32 * th) as i32, t.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
    }
    sheet.save_png(format!("{out}/status.png")).unwrap();

    // the settings window
    for (page, name) in [(0u8, "settings-general"), (1, "settings-music")] {
        let mut p = Panel::new(1.0, Settings::default(), true, (980, 810));
        p.set_page(page);
        for _ in 0..180 {
            p.tick(1.0 / 60.0);
        }
        p.render().save_png(format!("{out}/{name}.png")).unwrap();
    }
    println!("pictures written to {out}");
}
