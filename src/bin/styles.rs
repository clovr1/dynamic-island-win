//! Five ways to draw the on/off settings of a page, side by side (so one can be picked).
use island::gfx::{fill_rrect, Fonts, Weight};
use tiny_skia::{Color, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform};

const BG: [u8; 3] = [24, 24, 28];
const CARD: [u8; 3] = [46, 46, 54];
const EDGE: [u8; 3] = [70, 70, 80];
const TEXT: [u8; 3] = [245, 245, 250];
const DIM: [u8; 3] = [150, 150, 165];
const PINK: [u8; 3] = [255, 92, 141];
const DARK: [u8; 3] = [30, 12, 20];

const ITEMS: [(&str, &str, bool); 5] = [
    ("Small pill while music plays", "Show the cover and equalizer at the top", true),
    ("Hide it while the music app is in front", "No need for the pill then", true),
    ("Lyrics button", "A lyrics button in the player", false),
    ("Volume slider in the player", "Change the music's volume", true),
    ("Equalizer in the cover's colour", "Otherwise the equalizer is white", false),
];

fn check(pm: &mut Pixmap, cx: f32, cy: f32, sz: f32, rgb: [u8; 3]) {
    let mut pb = PathBuilder::new();
    pb.move_to(cx - 0.32 * sz, cy + 0.02 * sz);
    pb.line_to(cx - 0.08 * sz, cy + 0.26 * sz);
    pb.line_to(cx + 0.34 * sz, cy - 0.24 * sz);
    if let Some(p) = pb.finish() {
        let mut paint = Paint::default();
        paint.set_color_rgba8(rgb[0], rgb[1], rgb[2], 255);
        paint.anti_alias = true;
        pm.stroke_path(&p, &paint, &Stroke { width: sz * 0.16, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Default::default() }, Transform::identity(), None);
    }
}

fn text(f: &mut Fonts, pm: &mut Pixmap, w: Weight, t: &str, x: f32, cy: f32, px: f32, rgb: [u8; 3]) {
    f.draw(pm, w, t, x, cy + px * 0.36, px, rgb, 1.0);
}

fn switch(pm: &mut Pixmap, x: f32, y: f32, on: bool) {
    fill_rrect(pm, x, y, 44.0, 24.0, [12.0; 4], if on { PINK } else { [90, 90, 104] }, 1.0);
    fill_rrect(pm, x + if on { 22.0 } else { 2.0 }, y + 2.0, 20.0, 20.0, [10.0; 4], [255, 255, 255], 1.0);
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let mut f = Fonts::new();
    let (cw, ch) = (480.0f32, 250.0f32);
    let mut pm = Pixmap::new(980, 800).unwrap();
    pm.fill(Color::from_rgba8(BG[0], BG[1], BG[2], 255));
    let titles = ["A  Tiles", "B  List with switches", "C  Check boxes", "D  One connected bar", "E  Cards with a switch"];
    for (n, title) in titles.iter().enumerate() {
        let (ox, oy) = (10.0 + (n % 2) as f32 * 490.0, 10.0 + (n / 2) as f32 * 262.0);
        fill_rrect(&mut pm, ox, oy, cw, ch, [14.0; 4], [32, 32, 38], 1.0);
        text(&mut f, &mut pm, Weight::Bold, title, ox + 16.0, oy + 22.0, 15.0, TEXT);
        let (x0, y0) = (ox + 16.0, oy + 46.0);
        match n {
            0 => {
                // tiles: 3 per row
                let names = ["Small pill", "Hide when player open", "Lyrics button", "Volume slider", "Cover-colour equalizer"];
                for (i, (_, _, on)) in ITEMS.iter().enumerate() {
                    let (tx, ty) = (x0 + (i % 3) as f32 * 148.0, y0 + (i / 3) as f32 * 92.0);
                    fill_rrect(&mut pm, tx, ty, 140.0, 84.0, [16.0; 4], if *on { PINK } else { CARD }, 1.0);
                    if !*on {
                        fill_rrect(&mut pm, tx + 1.0, ty + 1.0, 138.0, 82.0, [15.0; 4], CARD, 1.0);
                    }
                    fill_rrect(&mut pm, tx + 14.0, ty + 14.0, 22.0, 22.0, [11.0; 4], if *on { DARK } else { EDGE }, 1.0);
                    if *on {
                        check(&mut pm, tx + 25.0, ty + 25.0, 22.0, PINK);
                    }
                    text(&mut f, &mut pm, Weight::Semibold, names[i], tx + 14.0, ty + 62.0, 13.0, if *on { DARK } else { TEXT });
                }
            }
            1 => {
                fill_rrect(&mut pm, x0, y0, 448.0, 5.0 * 38.0 + 8.0, [14.0; 4], CARD, 1.0);
                for (i, (l, _, on)) in ITEMS.iter().enumerate() {
                    let ry = y0 + 4.0 + i as f32 * 38.0;
                    if i > 0 {
                        fill_rrect(&mut pm, x0 + 14.0, ry, 420.0, 1.0, [0.0; 4], EDGE, 0.7);
                    }
                    text(&mut f, &mut pm, Weight::Regular, l, x0 + 16.0, ry + 19.0, 14.0, TEXT);
                    switch(&mut pm, x0 + 448.0 - 60.0, ry + 7.0, *on);
                }
            }
            2 => {
                for (i, (l, _, on)) in ITEMS.iter().enumerate() {
                    let (cx, cy) = (x0 + (i % 2) as f32 * 226.0, y0 + (i / 2) as f32 * 52.0);
                    fill_rrect(&mut pm, cx, cy + 6.0, 24.0, 24.0, [7.0; 4], if *on { PINK } else { EDGE }, 1.0);
                    if !*on {
                        fill_rrect(&mut pm, cx + 2.0, cy + 8.0, 20.0, 20.0, [5.5; 4], BG, 1.0);
                    } else {
                        check(&mut pm, cx + 12.0, cy + 18.0, 24.0, DARK);
                    }
                    let short = f.fit(Weight::Regular, l, 13.5, 180.0);
                    text(&mut f, &mut pm, Weight::Regular, &short, cx + 34.0, cy + 18.0, 13.5, TEXT);
                }
            }
            3 => {
                let names = ["Pill", "Hide if player open", "Lyrics", "Volume", "Cover EQ"];
                let widths = [70.0f32, 150.0, 76.0, 84.0, 92.0];
                let total: f32 = widths.iter().sum();
                fill_rrect(&mut pm, x0, y0 + 20.0, total, 44.0, [22.0; 4], CARD, 1.0);
                let mut sx = x0;
                for (i, (_, _, on)) in ITEMS.iter().enumerate() {
                    if *on {
                        fill_rrect(&mut pm, sx + 3.0, y0 + 23.0, widths[i] - 6.0, 38.0, [19.0; 4], PINK, 1.0);
                    }
                    let tw = f.measure(Weight::Semibold, names[i], 13.0);
                    text(&mut f, &mut pm, Weight::Semibold, names[i], sx + widths[i] / 2.0 - tw / 2.0, y0 + 42.0, 13.0, if *on { DARK } else { DIM });
                    sx += widths[i];
                }
                text(&mut f, &mut pm, Weight::Regular, "Pink = on. Tap a segment to switch it.", x0, y0 + 100.0, 12.5, DIM);
            }
            _ => {
                for (i, (l, d, on)) in ITEMS.iter().enumerate() {
                    let (cx, cy) = (x0 + (i % 2) as f32 * 226.0, y0 + (i / 2) as f32 * 64.0);
                    fill_rrect(&mut pm, cx, cy, 218.0, 58.0, [12.0; 4], CARD, 1.0);
                    let short = f.fit(Weight::Semibold, l, 13.0, 150.0);
                    text(&mut f, &mut pm, Weight::Semibold, &short, cx + 12.0, cy + 18.0, 13.0, TEXT);
                    let dd = f.fit(Weight::Regular, d, 11.0, 150.0);
                    text(&mut f, &mut pm, Weight::Regular, &dd, cx + 12.0, cy + 40.0, 11.0, DIM);
                    switch(&mut pm, cx + 218.0 - 54.0, cy + 17.0, *on);
                }
            }
        }
    }
    pm.save_png(format!("{out}/toggle_styles.png")).unwrap();
}
