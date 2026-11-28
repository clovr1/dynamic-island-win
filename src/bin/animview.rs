//! Frames in the middle of the window's animations (a page change, a toggle, a slider drag, the logo burst, a theme fade).
use island::panel::{Id, Panel};
use island::settings::Settings;
use tiny_skia::{Pixmap, PixmapPaint, Transform};

fn run(p: &mut Panel, secs: f32) {
    let mut t = 0.0;
    while t < secs {
        p.tick(1.0 / 60.0);
        t += 1.0 / 60.0;
    }
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let size = (920, 740);
    let mut shots: Vec<Pixmap> = Vec::new();
    // a page change (about 0.12 s in): the rows are still sliding up, the highlight is on its way
    let mut p = Panel::new(1.0, Settings::default(), true, size);
    p.render();
    run(&mut p, 1.0);
    let _ = p.press(Id::Nav(2));
    run(&mut p, 0.12);
    shots.push(p.render().clone());
    // a toggle halfway
    run(&mut p, 1.0);
    let _ = p.press(Id::Notif);
    run(&mut p, 0.07);
    shots.push(p.render().clone());
    // the logo burst
    let mut p = Panel::new(1.0, Settings::default(), true, size);
    p.render();
    let _ = p.press(Id::Logo);
    run(&mut p, 0.3);
    shots.push(p.render().clone());
    // a slider being dragged
    let mut p = Panel::new(1.0, Settings::default(), true, size);
    p.set_page(4);
    run(&mut p, 1.0);
    p.render();
    let _ = p.mouse_down(700.0, 520.0);
    let _ = p.mouse_move(720.0, 520.0);
    run(&mut p, 0.1);
    shots.push(p.render().clone());
    // the theme fading (about 0.15 s in)
    let mut s = Settings::default();
    s.theme = 0;
    let mut p = Panel::new(1.0, s, true, size);
    p.set_page(4);
    run(&mut p, 1.0);
    let _ = p.press(Id::Theme(3));
    run(&mut p, 0.15);
    shots.push(p.render().clone());
    let (cw, ch) = (460u32, 370u32);
    let mut sheet = Pixmap::new(cw * 3, ch * 2).unwrap();
    for (i, s) in shots.iter().enumerate() {
        let mut small = Pixmap::new(cw, ch).unwrap();
        small.draw_pixmap(0, 0, s.as_ref(), &PixmapPaint { quality: tiny_skia::FilterQuality::Bilinear, ..Default::default() }, Transform::from_scale(cw as f32 / s.width() as f32, ch as f32 / s.height() as f32), None);
        sheet.draw_pixmap((i as u32 % 3 * cw) as i32, (i as u32 / 3 * ch) as i32, small.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
    }
    sheet.save_png(format!("{out}/anim_sheet.png")).unwrap();
}
