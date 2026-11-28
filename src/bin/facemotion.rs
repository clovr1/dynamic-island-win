//! The little face turning its eyes after the mouse, frame by frame. Run: cargo run --release --bin facemotion
use island::settings::Settings;
use island::ui::{Ui, TOP, W};
use tiny_skia::{Color, Pixmap, PixmapPaint, Transform};

const S: f32 = 3.0;

fn crop(ui: &mut Ui) -> Pixmap {
    let (w, h) = (110.0 * S, 44.0 * S);
    let mut out = Pixmap::new(w as u32, h as u32).unwrap();
    out.fill(Color::from_rgba8(60, 64, 80, 255));
    let img = ui.render().clone();
    out.draw_pixmap(-(((W / 2.0 - 55.0) * S) as i32), -(((TOP - 4.0) * S) as i32), img.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
    out
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let mut ui = Ui::new(S);
    ui.set_settings(&Settings::default());
    for _ in 0..60 {
        ui.tick(1.0 / 60.0);
    }
    let mut frames: Vec<Pixmap> = Vec::new();
    // the mouse goes to the right, then up-left, then far down; one picture every 4th frame
    let targets: [(f32, f32); 4] = [(0.0, 0.0), (400.0, 0.0), (-300.0, -120.0), (60.0, 300.0)];
    for (ti, (tx, ty)) in targets.iter().enumerate() {
        ui.set_look(Some((*tx, *ty)));
        for f in 0..24 {
            ui.tick(1.0 / 60.0);
            if f % 4 == 3 && ti > 0 {
                frames.push(crop(&mut ui));
            }
        }
    }
    let (fw, fh) = (frames[0].width(), frames[0].height());
    let cols = 6u32;
    let rows = (frames.len() as u32 + cols - 1) / cols;
    let mut sheet = Pixmap::new(fw * cols, fh * rows).unwrap();
    sheet.fill(Color::from_rgba8(30, 30, 34, 255));
    for (i, p) in frames.iter().enumerate() {
        sheet.draw_pixmap(((i as u32 % cols) * fw) as i32, ((i as u32 / cols) * fh) as i32, p.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
    }
    sheet.save_png(format!("{out}/facemotion.png")).unwrap();
}
