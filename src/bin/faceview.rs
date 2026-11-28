//! The little face in the resting pill, in a few moments of its day, and the "Copied" pill. Run: cargo run --release --bin faceview
use island::settings::Settings;
use island::ui::{StatusData, Ui, TOP, W};
use tiny_skia::{Color, Pixmap, PixmapPaint, Transform};

const S: f32 = 4.0;

fn run_to(ui: &mut Ui, from: f32, to: f32) {
    let mut t = from;
    while t < to {
        ui.tick(1.0 / 60.0);
        t += 1.0 / 60.0;
    }
}

fn crop(ui: &mut Ui) -> Pixmap {
    let (w, h) = (150.0 * S, 48.0 * S);
    let mut out = Pixmap::new(w as u32, h as u32).unwrap();
    out.fill(Color::from_rgba8(60, 64, 80, 255));
    let img = ui.render().clone();
    let (ox, oy) = ((W / 2.0 - 75.0) * S, (TOP - 6.0) * S);
    out.draw_pixmap(-(ox as i32), -(oy as i32), img.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
    out
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let mk = || {
        let mut ui = Ui::new(S);
        ui.set_settings(&Settings::default());
        ui
    };
    let mut shots: Vec<Pixmap> = Vec::new();
    // awake, a blink, a glance, just after a pill, asleep
    for (label, t) in [("awake", 1.0f32), ("blink", 4.4), ("glance", 3.6), ("asleep", 110.0)] {
        let mut ui = mk();
        run_to(&mut ui, 0.0, t);
        shots.push(crop(&mut ui));
        println!("{label}: t = {t}");
    }
    // smiling: a pill has just gone
    let mut ui = mk();
    ui.push_status(StatusData::Copied { kind: "Text".into() });
    run_to(&mut ui, 0.0, 5.0);
    shots.push(crop(&mut ui));
    // the pill of something copied
    for kind in ["Text", "Image", "Files"] {
        let mut ui = mk();
        ui.push_status(StatusData::Copied { kind: kind.into() });
        run_to(&mut ui, 0.0, 1.0);
        let (w, h) = (330.0 * S, 62.0 * S);
        let mut o = Pixmap::new(w as u32, h as u32).unwrap();
        o.fill(Color::from_rgba8(60, 64, 80, 255));
        let img = ui.render().clone();
        o.draw_pixmap(-(((W / 2.0 - 165.0) * S) as i32), -(((TOP - 6.0) * S) as i32), img.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
        shots.push(o);
    }
    let sw = shots.iter().map(|p| p.width()).max().unwrap();
    let sh: u32 = shots.iter().map(|p| p.height() + 10).sum();
    let mut sheet = Pixmap::new(sw, sh).unwrap();
    sheet.fill(Color::from_rgba8(30, 30, 34, 255));
    let mut y = 0;
    for p in &shots {
        sheet.draw_pixmap(0, y as i32, p.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
        y += p.height() + 10;
    }
    sheet.save_png(format!("{out}/faceview.png")).unwrap();
}
