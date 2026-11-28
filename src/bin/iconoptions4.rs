//! "Bloop" icon candidates (iconoptions4.png). Run: cargo run --release --bin iconoptions4
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

/// Soft blobs that melt together (metaballs): shapes are painted into a coverage grid, blurred, then cut at a threshold.
fn blob(pm: &mut Pixmap, s: f32, shapes: &[(f32, f32, f32, f32, f32)], top: [u8; 3], bottom: [u8; 3]) {
    let n = pm.width() as usize;
    let mut a = vec![0.0f32; n * n];
    for &(x, y, w, h, r) in shapes {
        let (x0, y0, x1, y1) = ((x * s) as i32, (y * s) as i32, ((x + w) * s) as i32, ((y + h) * s) as i32);
        for py in y0.max(0)..y1.min(n as i32) {
            for px in x0.max(0)..x1.min(n as i32) {
                // inside a rounded rectangle with corner radius r?
                let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
                let rr = r * s;
                let dx = (fx - (x * s + rr)).min(0.0).min((x * s + w * s - rr) - fx).abs().max(0.0);
                let _ = dx;
                let cx = fx.max(x * s + rr).min((x * s + w * s - rr).max(x * s + rr));
                let cy = fy.max(y * s + rr).min((y * s + h * s - rr).max(y * s + rr));
                if (fx - cx).hypot(fy - cy) <= rr {
                    a[py as usize * n + px as usize] = 1.0;
                }
            }
        }
    }
    let rad = ((s * 0.028) as usize).max(1);
    for _ in 0..3 {
        let mut t = vec![0.0f32; n * n];
        for y in 0..n {
            for x in 0..n {
                let (lo, hi) = (x.saturating_sub(rad), (x + rad).min(n - 1));
                t[y * n + x] = (lo..=hi).map(|i| a[y * n + i]).sum::<f32>() / (hi - lo + 1) as f32;
            }
        }
        for y in 0..n {
            for x in 0..n {
                let (lo, hi) = (y.saturating_sub(rad), (y + rad).min(n - 1));
                a[y * n + x] = (lo..=hi).map(|i| t[i * n + x]).sum::<f32>() / (hi - lo + 1) as f32;
            }
        }
    }
    let data = pm.data_mut();
    for y in 0..n {
        let k = y as f32 / n as f32;
        let k = ((k - 0.15) / 0.6).clamp(0.0, 1.0);
        for x in 0..n {
            let v = ((a[y * n + x] - 0.42) / 0.12).clamp(0.0, 1.0);
            let v = v * v * (3.0 - 2.0 * v);
            if v <= 0.0 {
                continue;
            }
            let i = (y * n + x) * 4;
            for q in 0..3 {
                let col = top[q] as f32 * (1.0 - k) + bottom[q] as f32 * k;
                data[i + q] = (col * v + data[i + q] as f32 * (1.0 - v)).round() as u8;
            }
            data[i + 3] = 255;
        }
    }
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

/// S: the pill pops a drop, with little pop marks around it.
fn s_(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    flat(&mut pm, s, [16, 16, 18]);
    pill(&mut pm, s, 0.38, 0.55, 0.5, 0.22, [255, 255, 255]);
    dot(&mut pm, s, 0.74, 0.5, 0.065, [255, 90, 60]);
    for (dx, dy) in [(0.0f32, -1.0f32), (0.87, -0.5), (-0.87, -0.5)] {
        let (x0, y0) = (0.74 + dx * 0.11, 0.5 + dy * 0.11);
        let (x1, y1) = (0.74 + dx * 0.17, 0.5 + dy * 0.17);
        let mut pb = PathBuilder::new();
        pb.move_to(x0 * s, y0 * s);
        pb.line_to(x1 * s, y1 * s);
        pm.stroke_path(&pb.finish().unwrap(), &solid([255, 90, 60]), &Stroke { width: 0.025 * s, line_cap: LineCap::Round, ..Default::default() }, Transform::identity(), None);
    }
    pm
}

/// T: bloop, bloop, bloop. Bubbles of growing size rise from the pill.
fn t(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    flat(&mut pm, s, [16, 16, 18]);
    pill(&mut pm, s, 0.5, 0.72, 0.62, 0.2, [255, 255, 255]);
    dot(&mut pm, s, 0.34, 0.5, 0.035, [255, 90, 60]);
    dot(&mut pm, s, 0.45, 0.36, 0.055, [255, 90, 60]);
    dot(&mut pm, s, 0.62, 0.24, 0.085, [255, 90, 60]);
    pm
}

/// U: the pill stretches and lets go of a drop, like water.
fn u(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    flat(&mut pm, s, [16, 16, 18]);
    blob(&mut pm, s, &[(0.13, 0.4, 0.5, 0.2, 0.1), (0.58, 0.44, 0.14, 0.12, 0.05), (0.7, 0.4, 0.18, 0.2, 0.1)], [255, 255, 255], [255, 255, 255]);
    pm
}

/// V: a drop in water. A dot with rings, white on coral.
fn v(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    flat(&mut pm, s, [255, 90, 60]);
    dot(&mut pm, s, 0.5, 0.5, 0.1, [255, 255, 255]);
    for (r, w) in [(0.2f32, 0.045f32), (0.31, 0.035), (0.41, 0.025)] {
        let mut pb = PathBuilder::new();
        pb.push_circle(0.5 * s, 0.5 * s, r * s);
        let mut mask = tiny_skia::Mask::new(n, n).unwrap();
        mask.fill_path(&rrect(s * 0.03, s * 0.03, s * 0.94, s * 0.94, [s * 0.225; 4]).unwrap(), FillRule::Winding, true, Transform::identity());
        pm.stroke_path(&pb.finish().unwrap(), &solid([255, 255, 255]), &Stroke { width: w * s, ..Default::default() }, Transform::identity(), Some(&mask));
    }
    pm
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let makers: [(&str, fn(u32) -> Pixmap); 6] = [("O", o), ("S", s_), ("T", t), ("U", u), ("V", v), ("O", o)];
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
    sheet.save_png(format!("{out}/iconoptions4.png")).unwrap();
}
