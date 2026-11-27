//! Frames of "lyrics view -> back to the controls": nothing may overlap. `cargo run --release --bin crossfade`
use island::lyrics::{parse_lrc, Found};
use island::ui::{MediaInfo, Ui, TOP, W};
use tiny_skia::{Color, Pixmap, PixmapPaint, Transform};
const S: f32 = 2.0;
fn main() {
    let mut ui = Ui::new(S);
    ui.set_media(MediaInfo { active: true, title: "Hello".into(), artist: "Adele".into(), playing: true, pos: 16.0, dur: 295.0 });
    ui.set_cursor(Some((W / 2.0 * S, (TOP + 12.0) * S)));
    for _ in 0..90 { ui.tick(1.0 / 60.0); }
    ui.toggle_lyrics();
    let req = ui.take_lyrics_request().unwrap();
    ui.set_lyrics(&req.key, Some(Found { synced: true, lines: parse_lrc("[00:04.00] I've forgotten how it felt before the world fell at our feet\n[00:09.00] There's such a difference between us\n[00:14.00] And a million miles") }));
    for _ in 0..90 { ui.tick(1.0 / 60.0); }
    // press the way back (the bubble in the top right corner)
    let (bx, by) = ((W / 2.0 + 200.0 - 37.0) * S, (TOP + 39.0) * S);
    ui.mouse_down(bx, by);
    let hit = ui.mouse_up(bx, by);
    println!("click on the way back -> {:?}", hit);
    ui.toggle_lyrics();
    let mut tiles = Vec::new();
    for i in 0..12 {
        for _ in 0..4 { ui.tick(1.0 / 60.0); }
        let mut bg = Pixmap::new(ui.size_px().0, ui.size_px().1).unwrap();
        bg.fill(Color::from_rgba8(60, 64, 80, 255));
        bg.draw_pixmap(0, 0, ui.render().as_ref(), &PixmapPaint::default(), Transform::identity(), None);
        tiles.push(bg); let _ = i;
    }
    let (w, h) = (tiles[0].width(), 420u32);
    let mut sheet = Pixmap::new(w * 4, h * 3).unwrap();
    for (i, t) in tiles.iter().enumerate() { sheet.draw_pixmap(((i as u32 % 4) * w) as i32, ((i as u32 / 4) * h) as i32, t.as_ref(), &PixmapPaint::default(), Transform::identity(), None); }
    sheet.save_png("preview/crossfade.png").unwrap();
}
