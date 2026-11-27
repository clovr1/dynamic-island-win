//! Renders every state of the island exactly like cmp/electron_states.js does for the Electron version, so the two can be laid side by side.
//! `cargo run --release --bin compare -- <dir with art.png and icon.png>`
use island::lyrics::{parse_lrc, Found};
use island::ui::{ArtData, MediaInfo, NotifData, Ui, TOP, W};
use tiny_skia::{Color, Pixmap, PixmapPaint, Transform};

const S: f32 = 2.0;

fn load(path: &str) -> ArtData {
    let pm = Pixmap::load_png(path).expect("png");
    let mut rgba = Vec::new();
    for p in pm.data().chunks_exact(4) {
        let a = p[3] as u32;
        let un = |c: u8| if a == 0 { 0 } else { ((c as u32 * 255 + a / 2) / a).min(255) as u8 };
        rgba.extend_from_slice(&[un(p[0]), un(p[1]), un(p[2]), p[3]]);
    }
    ArtData { w: pm.width(), h: pm.height(), rgba }
}

fn run(ui: &mut Ui, ms: f32) {
    let mut t = 0.0;
    while t < ms / 1000.0 {
        ui.tick(1.0 / 60.0);
        t += 1.0 / 60.0;
    }
}

fn save(ui: &mut Ui, dir: &str, name: &str) {
    let (w, h) = ui.size_px();
    let mut bg = Pixmap::new(w, h).unwrap();
    bg.fill(Color::from_rgba8(0x3c, 0x40, 0x50, 255));
    bg.draw_pixmap(0, 0, ui.render().as_ref(), &PixmapPaint::default(), Transform::identity(), None);
    bg.save_png(format!("{dir}/n_{name}.png")).unwrap();
}

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "cmp".into());
    let art = || load(&format!("{dir}/art.png"));
    let icon = || load(&format!("{dir}/icon.png"));
    let media = |pos: f64| MediaInfo { active: true, title: "Let Down".into(), artist: "Radiohead".into(), playing: false, pos, dur: 299.0 };
    let playing = |pos: f64| MediaInfo { playing: true, ..media(pos) };
    let hover = Some((W / 2.0 * S + 1.0, (TOP + 20.0) * S));
    let lines = [(4, "We were never meant to leave"), (9, "There's a stranger in my bed"), (14, "I can hear it breathe"), (19, "Hold on, hold on to me"), (24, "Every wing a little song that carries us along"), (29, "And send them through the sky to find you by the sea"), (34, "But I keep on moving on")];
    let lrc = lines.iter().map(|(t, l)| format!("[00:{:02}.00] {}", t, l)).collect::<Vec<_>>().join("\n");

    let mut ui = Ui::new(S);
    run(&mut ui, 1500.0);
    save(&mut ui, &dir, "idle");

    let mut ui = Ui::new(S);
    ui.set_art(Some(art()));
    ui.set_media(playing(190.0));
    run(&mut ui, 1600.0);
    save(&mut ui, &dir, "play");

    ui.set_media(media(190.0)); // (the Electron picture is taken while paused)
    ui.set_cursor(hover);
    run(&mut ui, 1600.0);
    save(&mut ui, &dir, "open");

    let mut ui = Ui::new(S);
    ui.push_notification(NotifData { app: "Mail".into(), aumid: String::new(), title: "Dina".into(), body: "Can you send me the slides for tomorrow?".into(), icon: Some(icon()) });
    run(&mut ui, 1600.0);
    save(&mut ui, &dir, "card_logo");

    let mut ui = Ui::new(S);
    let insta = || NotifData { app: "Google Chrome".into(), aumid: String::new(), title: "Instagram".into(), body: "You have unseen notifications.".into(), icon: island::brands::render(island::brands::brand_of("Google Chrome", "Instagram", "").unwrap(), 96) };
    ui.push_notification(insta());
    run(&mut ui, 1600.0);
    save(&mut ui, &dir, "card_instagram");
    // the same notification again and again must not keep the card alive
    let mut shown_late = false;
    for _ in 0..8 { run(&mut ui, 2500.0); ui.push_notification(insta()); if ui.mode() == island::ui::Mode::Card && ui.rect_px().3 > 0.0 { shown_late = true; } }
    run(&mut ui, 1200.0);
    println!("after 22 s of the same notification being re-sent: card on screen = {} (it must be false)", ui.mode() == island::ui::Mode::Card);
    let _ = shown_late;

    let mut ui = Ui::new(S);
    ui.push_notification(NotifData { app: "Messages".into(), aumid: String::new(), title: "Budi".into(), body: "Are you coming tonight?".into(), icon: None });
    run(&mut ui, 1600.0);
    save(&mut ui, &dir, "card_letter");

    let mut ui = Ui::new(S);
    ui.set_art(Some(art()));
    ui.set_media(playing(190.0));
    run(&mut ui, 800.0);
    ui.push_notification(NotifData { app: "Mail".into(), aumid: String::new(), title: "Dina".into(), body: "hi".into(), icon: Some(icon()) });
    run(&mut ui, 1600.0);
    save(&mut ui, &dir, "bubble");

    let mut ui = Ui::new(S);
    ui.set_art(Some(art()));
    ui.set_media(playing(15.0));
    run(&mut ui, 700.0);
    ui.set_cursor(hover);
    run(&mut ui, 1000.0);
    ui.toggle_lyrics();
    let req = ui.take_lyrics_request().unwrap();
    ui.set_lyrics(&req.key, Some(Found { synced: true, lines: parse_lrc(&lrc) }));
    run(&mut ui, 1800.0);
    save(&mut ui, &dir, "lyrics");
    ui.set_cursor(None);
    run(&mut ui, 2200.0);
    save(&mut ui, &dir, "lyricline");
}
