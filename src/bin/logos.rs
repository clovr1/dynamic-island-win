use island::brands;
use tiny_skia::{Color, Pixmap, PixmapPaint, Transform};
fn main() {
    let keys = ["whatsapp", "instagram", "telegram", "discord", "messenger", "facebook", "gmail", "youtube", "tiktok", "x"];
    let mut sheet = Pixmap::new(5 * 110 + 10, 2 * 110 + 10).unwrap();
    sheet.fill(Color::from_rgba8(30, 30, 34, 255));
    for (i, k) in keys.iter().enumerate() {
        let a = brands::render(k, 96).unwrap();
        let mut pm = Pixmap::new(96, 96).unwrap();
        for (d, s) in pm.data_mut().chunks_exact_mut(4).zip(a.rgba.chunks_exact(4)) {
            let al = s[3] as u32;
            d[0] = (s[0] as u32 * al / 255) as u8; d[1] = (s[1] as u32 * al / 255) as u8; d[2] = (s[2] as u32 * al / 255) as u8; d[3] = s[3];
        }
        sheet.draw_pixmap(10 + (i as i32 % 5) * 110, 10 + (i as i32 / 5) * 110, pm.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
    }
    sheet.save_png("preview/logos.png").unwrap();
}
