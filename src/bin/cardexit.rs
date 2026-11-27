//! Frames of a card going away (also an unfolded one), to look at the way back to the small pill.
use island::settings::Settings;
use island::ui::{MediaInfo, NotifData, Ui};

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let mut ui = Ui::new(1.0);
    ui.set_settings(&Settings::default());
    ui.set_media(MediaInfo { active: true, title: "Midnight Drive".into(), artist: "The Islanders".into(), playing: true, pos: 5.0, dur: 200.0 });
    let run = |ui: &mut Ui, s: f32| {
        let mut t = 0.0;
        while t < s {
            ui.tick(1.0 / 60.0);
            t += 1.0 / 60.0;
        }
    };
    run(&mut ui, 1.0);
    ui.push_notification(NotifData { app: "Messages".into(), aumid: String::new(), title: "Mum".into(), body: "Dinner at 7 tonight, don't be late. I made your favourite and everyone is already waiting for you.".into(), icon: None });
    run(&mut ui, 0.6);
    // hover the bubble, it becomes the card; click: unfolds
    let (x, y, w, h) = ui.rect_px();
    ui.set_cursor(Some((x + w - 20.0, y + h / 2.0)));
    run(&mut ui, 1.2);
    let _ = ui.card_click();
    run(&mut ui, 1.0);
    ui.set_cursor(None);
    // wait until it ends, then record the way back
    let mut shots = Vec::new();
    let mut t = 0.0;
    let mut recording = false;
    while t < 8.0 && shots.len() < 18 {
        ui.tick(1.0 / 60.0);
        t += 1.0 / 60.0;
        let (_, _, _, h) = ui.rect_px();
        if !recording && ui.mode() != island::ui::Mode::Card {
            recording = true;
        }
        if recording && ((t * 60.0) as u32) % 3 == 0 {
            shots.push(ui.render().clone());
        }
        let _ = h;
    }
    let cols = 3usize;
    let rows = (shots.len() + cols - 1) / cols;
    let mut sheet = tiny_skia::Pixmap::new(470 * cols as u32, 120 * rows as u32).unwrap();
    sheet.fill(tiny_skia::Color::from_rgba8(60, 62, 80, 255));
    for (i, p) in shots.iter().enumerate() {
        sheet.draw_pixmap((i % cols) as i32 * 470, (i / cols) as i32 * 120, p.as_ref(), &tiny_skia::PixmapPaint::default(), tiny_skia::Transform::identity(), None);
    }
    sheet.save_png(format!("{out}/card_exit.png")).unwrap();
}
