use island::gfx::{Fonts, Weight};
use tiny_skia::Pixmap;

#[test]
fn notes_are_drawn_not_boxes() {
    let mut f = Fonts::new();
    let mut pm = Pixmap::new(260, 70).unwrap();
    pm.fill(tiny_skia::Color::BLACK);
    f.draw(&mut pm, Weight::Semibold, "♪ ♫ Hello ♪", 10.0, 30.0, 28.0, [255, 255, 255], 1.0);
    f.draw(&mut pm, Weight::Semibold, "♪ 日本", 10.0, 62.0, 14.0, [255, 255, 255], 1.0);
    if let Ok(dir) = std::env::var("NOTE_OUT") {
        pm.save_png(format!("{dir}/notes.png")).unwrap();
    }
    assert!(f.measure(Weight::Semibold, "♪", 14.0) > 4.0);
    // a missing character takes space but draws nothing
    assert!(f.measure(Weight::Semibold, "日", 14.0) > 0.0);
}
