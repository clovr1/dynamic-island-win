//! Draws a few app icon candidates on one sheet (iconoptions.png). Run: cargo run --release --bin iconoptions
use island::gfx::{fill_rrect, rrect, Fonts, Weight};
use tiny_skia::{Color, FillRule, GradientStop, LinearGradient, Paint, Pixmap, Point, RadialGradient, SpreadMode, Stroke, Transform};

fn c(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color::from_rgba8(r, g, b, a)
}

fn tile(pm: &mut Pixmap, s: f32, stops: &[(f32, Color)], from: (f32, f32), to: (f32, f32)) -> tiny_skia::Path {
    let path = rrect(s * 0.03, s * 0.03, s * 0.94, s * 0.94, [s * 0.225; 4]).unwrap();
    let mut p = Paint::default();
    p.anti_alias = true;
    p.shader = LinearGradient::new(Point::from_xy(from.0 * s, from.1 * s), Point::from_xy(to.0 * s, to.1 * s), stops.iter().map(|(o, c)| GradientStop::new(*o, *c)).collect(), SpreadMode::Pad, Transform::identity()).unwrap();
    pm.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None);
    path
}

fn glow(pm: &mut Pixmap, path: &tiny_skia::Path, cx: f32, cy: f32, r: f32, col: Color) {
    let mut c0 = col;
    let mut c1 = col;
    c0.set_alpha(col.alpha());
    c1.set_alpha(0.0);
    let mut g = Paint::default();
    g.anti_alias = true;
    g.shader = RadialGradient::new(Point::from_xy(cx, cy), Point::from_xy(cx, cy), r, vec![GradientStop::new(0.0, c0), GradientStop::new(1.0, c1)], SpreadMode::Pad, Transform::identity()).unwrap();
    pm.fill_path(path, &g, FillRule::Winding, Transform::identity(), None);
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

fn bars(pm: &mut Pixmap, x: f32, y: f32, h: f32, w: f32, col: [u8; 3]) {
    for (i, k) in [0.45f32, 0.9, 0.62, 1.0].iter().enumerate() {
        let bh = h * k;
        fill_rrect(pm, x + i as f32 * w * 1.9, y + (h - bh) / 2.0, w, bh, [w / 2.0; 4], col, 0.96);
    }
}

/// A: the first idea, tidied. A black island on violet-to-cyan, its sound rippling out.
fn a(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    let t = tile(&mut pm, s, &[(0.0, c(140, 70, 255, 255)), (0.55, c(60, 120, 255, 255)), (1.0, c(0, 214, 200, 255))], (0.0, 0.0), (1.0, 1.0));
    glow(&mut pm, &t, s * 0.5, s * 0.05, s * 0.8, c(255, 255, 255, 70));
    // a black island with a cover and bars, and the sound spreading out under it
    let mut mask = tiny_skia::Mask::new(n, n).unwrap();
    mask.fill_path(&t, FillRule::Winding, true, Transform::identity());
    for (k, al) in [(0.0f32, 150u8), (1.0, 90), (2.0, 45)] {
        let (w, h) = (s * (0.7 + 0.16 * k), s * (0.28 + 0.16 * k));
        let path = rrect(s * 0.5 - w / 2.0, s * 0.5 - h / 2.0, w, h, [h / 2.0; 4]).unwrap();
        let mut sp = Paint::default();
        sp.set_color_rgba8(255, 255, 255, al);
        sp.anti_alias = true;
        pm.stroke_path(&path, &sp, &Stroke { width: (s * 0.014).max(1.0), ..Default::default() }, Transform::identity(), Some(&mask));
    }
    let (pw, ph) = (s * 0.64, s * 0.24);
    let (px, py) = (s * 0.18, s * 0.38);
    fill_rrect(&mut pm, px, py + s * 0.015, pw, ph, [ph / 2.0; 4], [0, 0, 40], 0.3);
    fill_rrect(&mut pm, px, py, pw, ph, [ph / 2.0; 4], [8, 8, 14], 1.0);
    let dd = ph * 0.58;
    fill_rrect(&mut pm, px + ph * 0.2, py + (ph - dd) / 2.0, dd, dd, [dd * 0.3; 4], [255, 138, 76], 1.0);
    fill_rrect(&mut pm, px + ph * 0.2 + dd * 0.22, py + (ph - dd) / 2.0 + dd * 0.18, dd * 0.6, dd * 0.6, [dd * 0.3; 4], [255, 80, 150], 0.9);
    bars(&mut pm, px + pw - ph * 0.85, py + ph * 0.25, ph * 0.5, ph * 0.09, [255, 255, 255]);
    pm
}

/// B: the island itself, black on black with a glowing rim.
fn b(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    let t = tile(&mut pm, s, &[(0.0, c(34, 30, 48, 255)), (1.0, c(10, 10, 16, 255))], (0.0, 0.0), (0.4, 1.0));
    glow(&mut pm, &t, s * 0.5, s * 0.5, s * 0.55, c(255, 90, 150, 60));
    let (pw, ph) = (s * 0.7, s * 0.3);
    let (px, py) = (s * 0.15, s * 0.35);
    let path = rrect(px, py, pw, ph, [ph / 2.0; 4]).unwrap();
    let mut rim = Paint::default();
    rim.anti_alias = true;
    rim.shader = LinearGradient::new(Point::from_xy(px, py), Point::from_xy(px + pw, py + ph), vec![GradientStop::new(0.0, c(255, 150, 70, 255)), GradientStop::new(0.5, c(255, 70, 150, 255)), GradientStop::new(1.0, c(150, 90, 255, 255))], SpreadMode::Pad, Transform::identity()).unwrap();
    fill_rrect(&mut pm, px, py, pw, ph, [ph / 2.0; 4], [0, 0, 0], 1.0);
    pm.stroke_path(&path, &rim, &Stroke { width: (s * 0.03).max(1.0), ..Default::default() }, Transform::identity(), None);
    let d = ph * 0.56;
    fill_rrect(&mut pm, px + ph * 0.22, py + (ph - d) / 2.0, d, d, [d * 0.3; 4], [255, 138, 76], 1.0);
    fill_rrect(&mut pm, px + ph * 0.22 + d * 0.22, py + (ph - d) / 2.0 + d * 0.18, d * 0.6, d * 0.6, [d * 0.3; 4], [255, 80, 150], 0.9);
    bars(&mut pm, px + pw - ph * 0.9, py + ph * 0.25, ph * 0.5, ph * 0.09, [255, 255, 255]);
    pm
}

/// C: bold and flat. A white island with a coloured dot, on sunset colours.
fn cc(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    let t = tile(&mut pm, s, &[(0.0, c(255, 150, 80, 255)), (0.5, c(255, 70, 130, 255)), (1.0, c(140, 60, 220, 255))], (0.1, 0.0), (0.9, 1.0));
    glow(&mut pm, &t, s * 0.3, s * 0.1, s * 0.7, c(255, 255, 255, 60));
    let (pw, ph) = (s * 0.66, s * 0.3);
    let (px, py) = (s * 0.17, s * 0.35);
    fill_rrect(&mut pm, px, py + s * 0.02, pw, ph, [ph / 2.0; 4], [90, 20, 70], 0.3);
    fill_rrect(&mut pm, px, py, pw, ph, [ph / 2.0; 4], [255, 255, 255], 1.0);
    let d = ph * 0.6;
    fill_rrect(&mut pm, px + ph * 0.2, py + (ph - d) / 2.0, d, d, [d / 2.0; 4], [255, 80, 130], 1.0);
    bars(&mut pm, px + pw - ph * 0.9, py + ph * 0.25, ph * 0.5, ph * 0.09, [40, 20, 60]);
    pm
}

/// D: ripples. The island sits on deep blue and its sound spreads out like water.
fn d(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    let t = tile(&mut pm, s, &[(0.0, c(20, 60, 140, 255)), (1.0, c(6, 14, 50, 255))], (0.0, 0.0), (0.3, 1.0));
    glow(&mut pm, &t, s * 0.5, s * 0.5, s * 0.6, c(60, 200, 255, 90));
    let mut mask = tiny_skia::Mask::new(n, n).unwrap();
    mask.fill_path(&t, FillRule::Winding, true, Transform::identity());
    for (k, al) in [230u8, 150, 90, 50].into_iter().enumerate() {
        let rad = s * (0.14 + 0.1 * k as f32);
        let mut pb = tiny_skia::PathBuilder::new();
        pb.push_oval(tiny_skia::Rect::from_xywh(s * 0.5 - rad * 1.5, s * 0.5 - rad * 0.8, rad * 3.0, rad * 1.6).unwrap());
        let mut sp = Paint::default();
        sp.set_color_rgba8(130, 225, 255, al);
        sp.anti_alias = true;
        pm.stroke_path(&pb.finish().unwrap(), &sp, &Stroke { width: (s * 0.016).max(1.0), ..Default::default() }, Transform::identity(), Some(&mask));
    }
    let (pw, ph) = (s * 0.5, s * 0.2);
    fill_rrect(&mut pm, s * 0.25, s * 0.4 + s * 0.012, pw, ph, [ph / 2.0; 4], [0, 0, 0], 0.4);
    fill_rrect(&mut pm, s * 0.25, s * 0.4, pw, ph, [ph / 2.0; 4], [5, 6, 12], 1.0);
    let dd = ph * 0.58;
    fill_rrect(&mut pm, s * 0.25 + ph * 0.2, s * 0.4 + (ph - dd) / 2.0, dd, dd, [dd * 0.3; 4], [90, 220, 255], 1.0);
    bars(&mut pm, s * 0.25 + pw - ph * 0.85, s * 0.4 + ph * 0.25, ph * 0.5, ph * 0.09, [255, 255, 255]);
    pm
}

/// E: light and friendly. A mint tile with a dark island and a soft shadow.
fn e(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    let t = tile(&mut pm, s, &[(0.0, c(236, 255, 246, 255)), (1.0, c(150, 235, 205, 255))], (0.0, 0.0), (0.6, 1.0));
    glow(&mut pm, &t, s * 0.5, s * 0.1, s * 0.7, c(255, 255, 255, 140));
    let (pw, ph) = (s * 0.68, s * 0.28);
    let (px, py) = (s * 0.16, s * 0.36);
    fill_rrect(&mut pm, px, py + s * 0.05, pw, ph, [ph / 2.0; 4], [20, 90, 70], 0.28);
    fill_rrect(&mut pm, px, py, pw, ph, [ph / 2.0; 4], [16, 24, 22], 1.0);
    let dd = ph * 0.58;
    fill_rrect(&mut pm, px + ph * 0.2, py + (ph - dd) / 2.0, dd, dd, [dd / 2.0; 4], [60, 220, 160], 1.0);
    bars(&mut pm, px + pw - ph * 0.9, py + ph * 0.25, ph * 0.5, ph * 0.09, [140, 255, 205]);
    pm
}

/// F: bubbles. A white island with three bubbles rising from it, on pink and orange.
fn f(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    let t = tile(&mut pm, s, &[(0.0, c(255, 120, 190, 255)), (1.0, c(255, 170, 90, 255))], (0.0, 0.0), (1.0, 1.0));
    glow(&mut pm, &t, s * 0.3, s * 0.1, s * 0.8, c(255, 255, 255, 90));
    for (x, y, r, al) in [(0.3f32, 0.3f32, 0.075f32, 0.95f32), (0.52, 0.2, 0.05, 0.75), (0.68, 0.3, 0.03, 0.55)] {
        fill_rrect(&mut pm, (x - r) * s, (y - r) * s, 2.0 * r * s, 2.0 * r * s, [r * s; 4], [255, 255, 255], al);
    }
    let (pw, ph) = (s * 0.7, s * 0.3);
    let (px, py) = (s * 0.15, s * 0.46);
    fill_rrect(&mut pm, px, py + s * 0.02, pw, ph, [ph / 2.0; 4], [150, 30, 80], 0.28);
    fill_rrect(&mut pm, px, py, pw, ph, [ph / 2.0; 4], [255, 255, 255], 1.0);
    let dd = ph * 0.6;
    fill_rrect(&mut pm, px + ph * 0.2, py + (ph - dd) / 2.0, dd, dd, [dd / 2.0; 4], [255, 100, 150], 1.0);
    bars(&mut pm, px + pw - ph * 0.9, py + ph * 0.25, ph * 0.5, ph * 0.09, [255, 100, 150]);
    pm
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let makers: [(&str, fn(u32) -> Pixmap); 6] = [("A", a), ("B", b), ("C", cc), ("D", d), ("E", e), ("F", f)];
    let mut fonts = Fonts::new();
    let mut sheet = Pixmap::new(960, 700).unwrap();
    sheet.fill(Color::from_rgba8(38, 38, 42, 255));
    for (i, (name, mk)) in makers.iter().enumerate() {
        let (cx, cy) = ((i % 3) as i32 * 320 + 20, (i / 3) as i32 * 350 + 20);
        sheet.draw_pixmap(cx, cy, mk(256).as_ref(), &tiny_skia::PixmapPaint::default(), Transform::identity(), None);
        sheet.draw_pixmap(cx + 150, cy + 270, mk(48).as_ref(), &tiny_skia::PixmapPaint::default(), Transform::identity(), None);
        sheet.draw_pixmap(cx + 210, cy + 282, mk(32).as_ref(), &tiny_skia::PixmapPaint::default(), Transform::identity(), None);
        sheet.draw_pixmap(cx + 255, cy + 290, mk(16).as_ref(), &tiny_skia::PixmapPaint::default(), Transform::identity(), None);
        fonts.draw(&mut sheet, Weight::Bold, name, cx as f32 + 6.0, cy as f32 + 312.0, 34.0, [255, 255, 255], 1.0);
        let _ = (&makers, Point::from_xy(0.0, 0.0));
    }
    sheet.save_png(format!("{out}/iconoptions.png")).unwrap();
}
