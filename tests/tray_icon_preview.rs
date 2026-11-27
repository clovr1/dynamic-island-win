use island::gfx::fill_rrect;
use tiny_skia::Pixmap;

fn icon(n: u32) -> Pixmap {
    let f = n as f32 / 32.0;
    let mut pm = Pixmap::new(n, n).unwrap();
    let (x, y, w, h) = (1.0 * f, 5.0 * f, 30.0 * f, 22.0 * f);
    fill_rrect(&mut pm, x, y, w, h, [h / 2.0; 4], [255, 255, 255], 1.0);
    let b = 2.6 * f;
    fill_rrect(&mut pm, x + b, y + b, w - 2.0 * b, h - 2.0 * b, [(h - 2.0 * b) / 2.0; 4], [0, 0, 0], 1.0);
    let d = 8.0 * f;
    fill_rrect(&mut pm, 6.6 * f, 16.0 * f - d / 2.0, d, d, [d / 2.0; 4], [255, 255, 255], 1.0);
    for (i, bh) in [7.0f32, 11.0, 6.0].iter().enumerate() {
        let (bw, bh) = (2.6 * f, bh * f);
        fill_rrect(&mut pm, (15.8 + i as f32 * 4.0) * f, 16.0 * f - bh / 2.0, bw, bh, [bw / 2.0; 4], [255, 255, 255], 1.0);
    }
    pm
}

#[test]
fn tray_icon_preview() {
    if let Ok(dir) = std::env::var("NOTE_OUT") {
        let mut sheet = Pixmap::new(200, 80).unwrap();
        sheet.fill(tiny_skia::Color::from_rgba8(32, 32, 32, 255));
        for (i, n) in [16u32, 24, 32].iter().enumerate() {
            let ic = icon(*n);
            sheet.draw_pixmap(10 + i as i32 * 50, 10, ic.as_ref(), &tiny_skia::PixmapPaint::default(), tiny_skia::Transform::identity(), None);
        }
        sheet.save_png(format!("{dir}/tray.png")).unwrap();
    }
}
