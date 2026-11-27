//! Renders the lyrics features to a PNG: `cargo run --release --bin preview_lyrics`
use island::lyrics::{parse_lrc, Found};
use island::ui::{ArtData, Env, MediaInfo, Ui, TOP, W};
use tiny_skia::{Color, Pixmap, PixmapPaint, Transform};

const SCALE: f32 = 2.0;

fn art() -> ArtData {
    let n = 128u32;
    let mut rgba = Vec::new();
    for y in 0..n {
        for x in 0..n {
            let t = (x + y) as f32 / (2 * n) as f32;
            rgba.extend_from_slice(&[(200.0 * (1.0 - t) + 60.0 * t) as u8, (190.0 * (1.0 - t) + 70.0 * t) as u8, (190.0 * (1.0 - t) + 80.0 * t) as u8, 255]);
        }
    }
    ArtData { w: n, h: n, rgba }
}

fn lrc() -> String {
    let lines = [
        "We were never meant to leave",
        "There's a stranger in my bed",
        "I can hear it breathe",
        "Hold on, hold on to me",
        "Every wing a little song that carries us along",
        "And send them through the sky to find you by the sea",
        "But I keep on moving on",
        "Paper birds tonight",
    ];
    lines.iter().enumerate().map(|(i, l)| format!("[00:{:02}.00] {}", 4 + i * 5, l)).collect::<Vec<_>>().join("\n")
}

fn main() {
    let mut ui = Ui::new(SCALE);
    let (pw, _) = ui.size_px();
    let mut shots: Vec<(String, Pixmap)> = Vec::new();
    let mut t = 0.0f32;
    let media = |pos: f64| MediaInfo { active: true, title: "Summer Flows".into(), artist: "Lumen Coast".into(), playing: false, pos, dur: 200.0 };
    let mut snap = |name: &str, ui: &mut Ui, shots: &mut Vec<(String, Pixmap)>| {
        let mut bg = Pixmap::new(ui.size_px().0, ui.size_px().1).unwrap();
        bg.fill(Color::from_rgba8(60, 64, 80, 255));
        bg.draw_pixmap(0, 0, ui.render().as_ref(), &PixmapPaint::default(), Transform::identity(), None);
        shots.push((name.to_string(), bg));
    };
    let mut run = |ui: &mut Ui, ms: f32, t: &mut f32| {
        let end = *t + ms / 1000.0;
        while *t < end { ui.tick(1.0 / 60.0); *t += 1.0 / 60.0; }
    };
    ui.set_art(Some(art()));
    ui.set_media(media(2.0));
    run(&mut ui, 800.0, &mut t);
    snap("1 small pill (lyrics off)", &mut ui, &mut shots);
    ui.set_cursor(Some((W / 2.0 * SCALE, (TOP + 18.0) * SCALE)));
    run(&mut ui, 900.0, &mut t);
    snap("2 hover: player with the lyrics button", &mut ui, &mut shots);
    ui.toggle_lyrics();
    let req = ui.take_lyrics_request().expect("a lookup is requested");
    println!("lookup requested: {} / {} / {}s", req.title, req.artist, req.dur);
    run(&mut ui, 500.0, &mut t);
    snap("3 lyrics view: looking...", &mut ui, &mut shots);
    ui.set_lyrics(&req.key, Some(Found { synced: true, lines: parse_lrc(&lrc()) }));
    ui.set_media(media(14.0));
    run(&mut ui, 900.0, &mut t);
    snap("4 lyrics, line 3 lit", &mut ui, &mut shots);
    ui.set_media(media(24.0));
    run(&mut ui, 300.0, &mut t);
    snap("5 next line, scrolling", &mut ui, &mut shots);
    run(&mut ui, 800.0, &mut t);
    snap("6 settled", &mut ui, &mut shots);
    ui.set_cursor(None);
    ui.set_media(media(31.0));
    run(&mut ui, 1400.0, &mut t);
    snap("7 cursor leaves: the pill shows the line", &mut ui, &mut shots);
    ui.set_media(media(34.0));
    run(&mut ui, 200.0, &mut t);
    snap("8 next line arriving (long line widens the pill)", &mut ui, &mut shots);
    run(&mut ui, 900.0, &mut t);
    snap("9 settled", &mut ui, &mut shots);
    // the music app is in front: the small pill stays empty
    ui.set_env(Env { player_front: true, ..Env::default() });
    run(&mut ui, 900.0, &mut t);
    snap("10 Spotify in front: empty island", &mut ui, &mut shots);
    ui.set_env(Env::default());
    run(&mut ui, 900.0, &mut t);
    snap("11 left Spotify: music pill back", &mut ui, &mut shots);

    // the notch (maximized window) shows the line, too
    ui.set_env(Env { maximized: true, ..Env::default() });
    ui.set_media(MediaInfo { playing: true, ..media(14.0) });
    run(&mut ui, 1200.0, &mut t);
    snap("12 notch: the line is shown", &mut ui, &mut shots);
    ui.set_media(MediaInfo { playing: true, ..media(24.0) });
    run(&mut ui, 150.0, &mut t);
    snap("13 notch: next (longer) line arriving", &mut ui, &mut shots);
    run(&mut ui, 900.0, &mut t);
    snap("14 notch: settled", &mut ui, &mut shots);
    // the lyrics view while hanging from the top edge (maximized window)
    ui.set_media(MediaInfo { playing: false, ..media(24.0) });
    ui.set_cursor(Some((W / 2.0 * SCALE, 12.0 * SCALE)));
    run(&mut ui, 1400.0, &mut t);
    snap("15 maximized window: lyrics in the enlarged notch", &mut ui, &mut shots);
    let crop_h = (225.0 * SCALE) as u32;
    let cols = 3u32;
    let rows = (shots.len() as u32 + cols - 1) / cols;
    let mut sheet = Pixmap::new(pw * cols, crop_h * rows).unwrap();
    sheet.fill(Color::from_rgba8(20, 20, 24, 255));
    for (i, (name, pm)) in shots.iter().enumerate() {
        sheet.draw_pixmap(((i as u32 % cols) * pw) as i32, ((i as u32 / cols) * crop_h) as i32, pm.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
        println!("{:>2}: {}", i, name);
    }
    sheet.save_png("preview/lyrics.png").unwrap();
}
