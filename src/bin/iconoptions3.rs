//! Flat icon candidates (iconoptions3.png). Run: cargo run --release --bin iconoptions3
use island::gfx::{fill_rrect, rrect, Fonts, Weight};
use tiny_skia::{Color, FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform};

fn flat(pm: &mut Pixmap, s: f32, col: [u8; 3]) {
    fill_rrect(pm, s * 0.03, s * 0.03, s * 0.94, s * 0.94, [s * 0.225; 4], col, 1.0);
}

fn solid(col: [u8; 3]) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(col[0], col[1], col[2], 255);
    p.anti_alias = true;
    p
}

fn dot(pm: &mut Pixmap, s: f32, cx: f32, cy: f32, r: f32, col: [u8; 3]) {
    fill_rrect(pm, (cx - r) * s, (cy - r) * s, 2.0 * r * s, 2.0 * r * s, [r * s; 4], col, 1.0);
}

fn pill(pm: &mut Pixmap, s: f32, cx: f32, cy: f32, w: f32, h: f32, col: [u8; 3]) {
    fill_rrect(pm, (cx - w / 2.0) * s, (cy - h / 2.0) * s, w * s, h * s, [h * s / 2.0; 4], col, 1.0);
}

/// M: a poster. A flat orange circle and a black pill overlapping on cream.
fn m(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    flat(&mut pm, s, [244, 238, 225]);
    dot(&mut pm, s, 0.42, 0.43, 0.2, [255, 92, 40]);
    pill(&mut pm, s, 0.58, 0.63, 0.56, 0.2, [18, 18, 20]);
    pm
}

/// N: it grows. A dot, a small pill, a wide pill: the island opening up.
fn nn(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    flat(&mut pm, s, [16, 16, 18]);
    dot(&mut pm, s, 0.5, 0.27, 0.07, [255, 255, 255]);
    pill(&mut pm, s, 0.5, 0.5, 0.36, 0.15, [255, 255, 255]);
    pill(&mut pm, s, 0.5, 0.73, 0.64, 0.19, [255, 90, 60]);
    pm
}

/// O: one pill and one drop that has just left it.
fn o(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    flat(&mut pm, s, [16, 16, 18]);
    pill(&mut pm, s, 0.4, 0.5, 0.52, 0.22, [255, 255, 255]);
    dot(&mut pm, s, 0.79, 0.5, 0.065, [255, 90, 60]);
    pm
}

/// P: a b built from two pills stacked, black on yellow.
fn pp(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    flat(&mut pm, s, [255, 208, 0]);
    let mut pb = PathBuilder::new();
    pb.move_to(0.34 * s, 0.22 * s);
    pb.line_to(0.34 * s, 0.78 * s);
    pb.move_to(0.34 * s, 0.22 * s);
    pb.line_to(0.5 * s, 0.22 * s);
    pb.cubic_to(0.7 * s, 0.22 * s, 0.7 * s, 0.5 * s, 0.5 * s, 0.5 * s);
    pb.line_to(0.34 * s, 0.5 * s);
    pb.move_to(0.5 * s, 0.5 * s);
    pb.line_to(0.54 * s, 0.5 * s);
    pb.cubic_to(0.76 * s, 0.5 * s, 0.76 * s, 0.78 * s, 0.54 * s, 0.78 * s);
    pb.line_to(0.34 * s, 0.78 * s);
    pm.stroke_path(&pb.finish().unwrap(), &solid([18, 18, 20]), &Stroke { width: 0.11 * s, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Default::default() }, Transform::identity(), None);
    pm
}

/// Q: five bars, one picked out. Flat blue.
fn q(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    flat(&mut pm, s, [44, 78, 255]);
    for (i, h) in [0.26f32, 0.46, 0.66, 0.4, 0.3].iter().enumerate() {
        let x = 0.5 + (i as f32 - 2.0) * 0.14;
        let col = if i == 2 { [255, 226, 60] } else { [255, 255, 255] };
        pill(&mut pm, s, x, 0.5, 0.09, *h, col);
    }
    pm
}

/// R: sticker style. Black outlines and a hard shadow, no blur anywhere.
fn r(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    flat(&mut pm, s, [255, 168, 200]);
    let line = (s * 0.03).max(1.0);
    let ink = solid([18, 18, 20]);
    let st = Stroke { width: line, ..Default::default() };
    let (pw, ph, px, py) = (s * 0.7, s * 0.28, s * 0.13, s * 0.36);
    fill_rrect(&mut pm, px + s * 0.035, py + s * 0.035, pw, ph, [ph / 2.0; 4], [18, 18, 20], 1.0);
    fill_rrect(&mut pm, px, py, pw, ph, [ph / 2.0; 4], [255, 255, 255], 1.0);
    pm.stroke_path(&rrect(px, py, pw, ph, [ph / 2.0; 4]).unwrap(), &ink, &st, Transform::identity(), None);
    let d = ph * 0.58;
    let (dx, dy) = (px + ph * 0.2 + d / 2.0, py + ph / 2.0);
    let mut pb = PathBuilder::new();
    pb.push_circle(dx, dy, d / 2.0);
    let c = pb.finish().unwrap();
    pm.fill_path(&c, &solid([255, 214, 0]), FillRule::Winding, Transform::identity(), None);
    pm.stroke_path(&c, &ink, &st, Transform::identity(), None);
    for (i, k) in [0.4f32, 0.7, 0.5].iter().enumerate() {
        let bh = ph * k * 0.8;
        fill_rrect(&mut pm, px + pw - ph * 0.9 + i as f32 * ph * 0.2, py + ph / 2.0 - bh / 2.0, ph * 0.09, bh, [ph * 0.045; 4], [18, 18, 20], 1.0);
    }
    pm
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let makers: [(&str, fn(u32) -> Pixmap); 6] = [("M", m), ("N", nn), ("O", o), ("P", pp), ("Q", q), ("R", r)];
    let mut fonts = Fonts::new();
    let _ = Weight::Bold;
    let mut sheet = Pixmap::new(960, 700).unwrap();
    sheet.fill(Color::from_rgba8(38, 38, 42, 255));
    for (i, (name, mk)) in makers.iter().enumerate() {
        let (cx, cy) = ((i % 3) as i32 * 320 + 20, (i / 3) as i32 * 350 + 20);
        sheet.draw_pixmap(cx, cy, mk(256).as_ref(), &tiny_skia::PixmapPaint::default(), Transform::identity(), None);
        sheet.draw_pixmap(cx + 150, cy + 270, mk(48).as_ref(), &tiny_skia::PixmapPaint::default(), Transform::identity(), None);
        sheet.draw_pixmap(cx + 210, cy + 282, mk(32).as_ref(), &tiny_skia::PixmapPaint::default(), Transform::identity(), None);
        sheet.draw_pixmap(cx + 255, cy + 290, mk(16).as_ref(), &tiny_skia::PixmapPaint::default(), Transform::identity(), None);
        fonts.draw(&mut sheet, Weight::Bold, name, cx as f32 + 6.0, cy as f32 + 312.0, 34.0, [255, 255, 255], 1.0);
    }
    sheet.save_png(format!("{out}/iconoptions3.png")).unwrap();
}
