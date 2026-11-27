//! How the island looks hanging from the top edge (under a maximized window). `cargo run --release --bin notchview`
use island::lyrics::{parse_lrc, Found};
use island::ui::{ArtData, Env, MediaInfo, NotifData, Ui, TOP, W};
use tiny_skia::{Color, Pixmap, PixmapPaint, Transform};
const S: f32 = 2.0;
fn art() -> ArtData {
    let n = 128u32;
    let mut rgba = Vec::new();
    for y in 0..n { for x in 0..n { let t = (x + y) as f32 / (2 * n) as f32; rgba.extend_from_slice(&[(220.0 * (1.0 - t) + 150.0 * t) as u8, (190.0 * (1.0 - t) + 120.0 * t) as u8, (110.0 * (1.0 - t) + 70.0 * t) as u8, 255]); } }
    ArtData { w: n, h: n, rgba }
}
fn run(ui: &mut Ui, ms: f32) { let mut t = 0.0; while t < ms / 1000.0 { ui.tick(1.0 / 60.0); t += 1.0 / 60.0; } }
fn main() {
    let media = |p: bool| MediaInfo { active: true, title: "As the World Caves In".into(), artist: "Matt Maltese".into(), playing: p, pos: 102.0, dur: 258.0 };
    let mut shots: Vec<Pixmap> = Vec::new();
    let mut snap = |ui: &mut Ui, shots: &mut Vec<Pixmap>| {
        let mut bg = Pixmap::new(ui.size_px().0, ui.size_px().1).unwrap();
        bg.fill(Color::from_rgba8(36, 40, 52, 255));
        bg.draw_pixmap(0, 0, ui.render().as_ref(), &PixmapPaint::default(), Transform::identity(), None);
        shots.push(bg);
    };
    let lrc = "[00:04.00] As the world caves in\n[01:30.00] Sitting here in the dark\n[01:45.00] Let the world cave in on us tonight\n[02:20.00] And I'm holding on";
    for (hover, lyrics, card, playing) in [(false, false, false, true), (true, false, false, true), (true, true, false, true), (false, false, true, true)] {
        let mut ui = Ui::new(S);
        ui.set_art(Some(art()));
        ui.set_media(media(playing));
        ui.set_env(Env { maximized: true, ..Env::default() });
        run(&mut ui, 600.0);
        if card { ui.push_notification(NotifData { app: "Instagram".into(), aumid: String::new(), title: "Instagram".into(), body: "You have unseen notifications.".into(), icon: island::brands::render("instagram", 96) }); }
        if hover { ui.set_cursor(Some((W / 2.0 * S, 12.0 * S))); }
        run(&mut ui, 900.0);
        if lyrics { ui.toggle_lyrics(); let q = ui.take_lyrics_request().unwrap(); ui.set_lyrics(&q.key, Some(Found { synced: true, lines: parse_lrc(lrc) })); run(&mut ui, 1500.0); }
        snap(&mut ui, &mut shots);
    }
    let (w, h) = (shots[0].width(), 440u32);
    let mut sheet = Pixmap::new(w * 2, h * 2).unwrap();
    for (i, t) in shots.iter().enumerate() { sheet.draw_pixmap(((i as u32 % 2) * w) as i32, ((i as u32 / 2) * h) as i32, t.as_ref(), &PixmapPaint::default(), Transform::identity(), None); }
    sheet.save_png("preview/notchview.png").unwrap();
    let _ = TOP;
}
