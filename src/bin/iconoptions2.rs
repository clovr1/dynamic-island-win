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

/// G: a glass bubble, with a rainbow sheen and a shine, and little bubbles rising.
fn g(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    let t = tile(&mut pm, s, &[(0.0, c(34, 24, 90, 255)), (1.0, c(8, 10, 34, 255))], (0.0, 0.0), (0.5, 1.0));
    glow(&mut pm, &t, s * 0.5, s * 0.55, s * 0.55, c(120, 90, 255, 90));
    let circle = |cx: f32, cy: f32, r: f32| {
        let mut pb = tiny_skia::PathBuilder::new();
        pb.push_circle(cx * s, cy * s, r * s);
        pb.finish().unwrap()
    };
    let big = circle(0.47, 0.56, 0.29);
    let mut fill = Paint::default();
    fill.anti_alias = true;
    fill.shader = RadialGradient::new(Point::from_xy(s * 0.4, s * 0.45), Point::from_xy(s * 0.47, s * 0.56), s * 0.34, vec![GradientStop::new(0.0, c(255, 255, 255, 70)), GradientStop::new(0.6, c(110, 200, 255, 70)), GradientStop::new(0.88, c(255, 120, 220, 120)), GradientStop::new(1.0, c(255, 220, 255, 190))], SpreadMode::Pad, Transform::identity()).unwrap();
    pm.fill_path(&big, &fill, FillRule::Winding, Transform::identity(), None);
    let mut rim = Paint::default();
    rim.anti_alias = true;
    rim.shader = LinearGradient::new(Point::from_xy(s * 0.2, s * 0.3), Point::from_xy(s * 0.75, s * 0.85), vec![GradientStop::new(0.0, c(140, 240, 255, 255)), GradientStop::new(0.5, c(255, 140, 230, 255)), GradientStop::new(1.0, c(150, 120, 255, 255))], SpreadMode::Pad, Transform::identity()).unwrap();
    pm.stroke_path(&big, &rim, &Stroke { width: (s * 0.016).max(1.0), ..Default::default() }, Transform::identity(), None);
    // the shine
    let mut pb = tiny_skia::PathBuilder::new();
    pb.push_oval(tiny_skia::Rect::from_xywh(s * 0.27, s * 0.38, s * 0.15, s * 0.07).unwrap());
    let mut sh = Paint::default();
    sh.set_color_rgba8(255, 255, 255, 215);
    sh.anti_alias = true;
    pm.fill_path(&pb.finish().unwrap(), &sh, FillRule::Winding, Transform::identity().pre_rotate_at(-40.0, s * 0.345, s * 0.415), None);
    // small bubbles
    for (x, y, r) in [(0.78, 0.26, 0.06f32), (0.67, 0.14, 0.035), (0.86, 0.42, 0.03)] {
        let b = circle(x, y, r);
        pm.fill_path(&b, &fill, FillRule::Winding, Transform::identity(), None);
        pm.stroke_path(&b, &rim, &Stroke { width: (s * 0.01).max(1.0), ..Default::default() }, Transform::identity(), None);
    }
    pm
}

/// H: the letter b, with a bubble dot, on fresh green.
fn h(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    let t = tile(&mut pm, s, &[(0.0, c(190, 255, 120, 255)), (0.55, c(40, 215, 160, 255)), (1.0, c(0, 150, 190, 255))], (0.1, 0.0), (0.8, 1.0));
    glow(&mut pm, &t, s * 0.3, s * 0.05, s * 0.8, c(255, 255, 255, 90));
    let mut fonts = Fonts::new();
    let px = s * 0.86;
    let w = fonts.measure(Weight::Bold, "b", px);
    fonts.draw(&mut pm, Weight::Bold, "b", (s - w) / 2.0 - s * 0.04, s * 0.76, px, [0, 55, 70], 0.28);
    fonts.draw(&mut pm, Weight::Bold, "b", (s - w) / 2.0 - s * 0.04, s * 0.74, px, [255, 255, 255], 1.0);
    let r = s * 0.055;
    fill_rrect(&mut pm, s * 0.72, s * 0.2, r * 2.0, r * 2.0, [r; 4], [255, 255, 255], 0.95);
    let r = s * 0.03;
    fill_rrect(&mut pm, s * 0.8, s * 0.33, r * 2.0, r * 2.0, [r; 4], [255, 255, 255], 0.7);
    pm
}

/// I: a little blob mascot.
fn i(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    let t = tile(&mut pm, s, &[(0.0, c(255, 226, 100, 255)), (1.0, c(255, 140, 70, 255))], (0.0, 0.0), (0.7, 1.0));
    glow(&mut pm, &t, s * 0.3, s * 0.05, s * 0.8, c(255, 255, 255, 100));
    fill_rrect(&mut pm, s * 0.2, s * 0.8, s * 0.6, s * 0.05, [s * 0.025; 4], [150, 60, 20], 0.18);
    blob(&mut pm, s, &[(0.2, 0.32, 0.6, 0.5, 0.25), (0.4, 0.2, 0.2, 0.2, 0.1)], [255, 255, 255], [255, 238, 214]);
    for x in [0.37f32, 0.63] {
        let r = s * 0.052;
        fill_rrect(&mut pm, x * s - r, s * 0.5 - r, r * 2.0, r * 2.0, [r; 4], [40, 24, 30], 1.0);
        let h = r * 0.38;
        fill_rrect(&mut pm, x * s - r * 0.1, s * 0.5 - r * 0.7, h, h, [h / 2.0; 4], [255, 255, 255], 1.0);
    }
    for x in [0.28f32, 0.72] {
        let r = s * 0.04;
        fill_rrect(&mut pm, x * s - r, s * 0.58 - r * 0.7, r * 2.0, r * 1.4, [r * 0.7; 4], [255, 120, 140], 0.55);
    }
    let mut pb = tiny_skia::PathBuilder::new();
    pb.move_to(s * 0.44, s * 0.585);
    pb.quad_to(s * 0.5, s * 0.66, s * 0.56, s * 0.585);
    let mut sp = Paint::default();
    sp.set_color_rgba8(40, 24, 30, 255);
    sp.anti_alias = true;
    pm.stroke_path(&pb.finish().unwrap(), &sp, &Stroke { width: s * 0.02, line_cap: tiny_skia::LineCap::Round, ..Default::default() }, Transform::identity(), None);
    pm
}

/// J: a music note whose parts melt together, glowing on black.
fn j(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    let t = tile(&mut pm, s, &[(0.0, c(34, 30, 44, 255)), (1.0, c(10, 10, 14, 255))], (0.0, 0.0), (0.4, 1.0));
    glow(&mut pm, &t, s * 0.5, s * 0.5, s * 0.6, c(255, 80, 140, 70));
    blob(&mut pm, s, &[(0.24, 0.55, 0.27, 0.23, 0.115), (0.45, 0.2, 0.075, 0.45, 0.035), (0.45, 0.2, 0.3, 0.1, 0.05), (0.62, 0.26, 0.13, 0.22, 0.065)], [255, 170, 80], [255, 60, 150]);
    pm
}

/// K: the island hanging from the top edge of the screen, on a soft morning sky.
fn k(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    let t = tile(&mut pm, s, &[(0.0, c(110, 190, 255, 255)), (0.6, c(190, 170, 255, 255)), (1.0, c(255, 190, 210, 255))], (0.0, 0.0), (0.4, 1.0));
    glow(&mut pm, &t, s * 0.5, s * 0.5, s * 0.5, c(255, 255, 255, 80));
    let mut mask = tiny_skia::Mask::new(n, n).unwrap();
    mask.fill_path(&t, FillRule::Winding, true, Transform::identity());
    // a soft light below the island
    let mut pb = tiny_skia::PathBuilder::new();
    pb.push_oval(tiny_skia::Rect::from_xywh(s * 0.2, s * 0.4, s * 0.6, s * 0.2).unwrap());
    let mut lp = Paint::default();
    lp.anti_alias = true;
    lp.shader = RadialGradient::new(Point::from_xy(s * 0.5, s * 0.5), Point::from_xy(s * 0.5, s * 0.5), s * 0.3, vec![GradientStop::new(0.0, c(255, 255, 255, 150)), GradientStop::new(1.0, c(255, 255, 255, 0))], SpreadMode::Pad, Transform::identity()).unwrap();
    pm.fill_path(&pb.finish().unwrap(), &lp, FillRule::Winding, Transform::identity(), Some(&mask));
    let (pw, ph) = (s * 0.5, s * 0.36);
    let (px, py) = (s * 0.25, s * 0.03);
    fill_rrect(&mut pm, px, py + s * 0.03, pw, ph, [0.0, 0.0, ph * 0.5, ph * 0.5], [60, 40, 110], 0.28);
    fill_rrect(&mut pm, px, py, pw, ph, [0.0, 0.0, ph * 0.5, ph * 0.5], [8, 8, 14], 1.0);
    let d = ph * 0.4;
    fill_rrect(&mut pm, px + pw * 0.14, py + ph * 0.4, d, d, [d * 0.3; 4], [255, 138, 76], 1.0);
    fill_rrect(&mut pm, px + pw * 0.14 + d * 0.22, py + ph * 0.4 + d * 0.18, d * 0.6, d * 0.6, [d * 0.3; 4], [255, 80, 150], 0.9);
    bars(&mut pm, px + pw * 0.62, py + ph * 0.42, ph * 0.36, ph * 0.065, [255, 255, 255]);
    // song lines hanging under it
    fill_rrect(&mut pm, s * 0.28, s * 0.58, s * 0.44, s * 0.05, [s * 0.025; 4], [255, 255, 255], 0.95);
    fill_rrect(&mut pm, s * 0.34, s * 0.68, s * 0.32, s * 0.04, [s * 0.02; 4], [255, 255, 255], 0.6);
    fill_rrect(&mut pm, s * 0.4, s * 0.77, s * 0.2, s * 0.035, [s * 0.0175; 4], [255, 255, 255], 0.35);
    pm
}

/// L: a sonar ping. Rings spreading from a glowing dot.
fn l(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    let t = tile(&mut pm, s, &[(0.0, c(6, 40, 52, 255)), (1.0, c(2, 12, 24, 255))], (0.0, 0.0), (0.5, 1.0));
    glow(&mut pm, &t, s * 0.5, s * 0.5, s * 0.55, c(40, 255, 200, 110));
    let mut mask = tiny_skia::Mask::new(n, n).unwrap();
    mask.fill_path(&t, FillRule::Winding, true, Transform::identity());
    for (k, (r, al, w)) in [(0.15f32, 255u8, 0.03f32), (0.26, 190, 0.026), (0.37, 120, 0.022), (0.48, 60, 0.018)].into_iter().enumerate() {
        let _ = k;
        let mut pb = tiny_skia::PathBuilder::new();
        pb.push_circle(s * 0.5, s * 0.5, r * s);
        let mut sp = Paint::default();
        sp.anti_alias = true;
        sp.shader = LinearGradient::new(Point::from_xy(0.0, 0.0), Point::from_xy(s, s), vec![GradientStop::new(0.0, c(120, 255, 230, al)), GradientStop::new(1.0, c(70, 170, 255, al))], SpreadMode::Pad, Transform::identity()).unwrap();
        pm.stroke_path(&pb.finish().unwrap(), &sp, &Stroke { width: (s * w).max(1.0), ..Default::default() }, Transform::identity(), Some(&mask));
    }
    let mut pb = tiny_skia::PathBuilder::new();
    pb.push_circle(s * 0.5, s * 0.5, s * 0.075);
    let mut dp = Paint::default();
    dp.anti_alias = true;
    dp.shader = RadialGradient::new(Point::from_xy(s * 0.48, s * 0.47), Point::from_xy(s * 0.5, s * 0.5), s * 0.09, vec![GradientStop::new(0.0, c(255, 255, 255, 255)), GradientStop::new(1.0, c(90, 255, 215, 255))], SpreadMode::Pad, Transform::identity()).unwrap();
    pm.fill_path(&pb.finish().unwrap(), &dp, FillRule::Winding, Transform::identity(), None);
    pm
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let makers: [(&str, fn(u32) -> Pixmap); 6] = [("G", g), ("H", h), ("I", i), ("J", j), ("K", k), ("L", l)];
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
    sheet.save_png(format!("{out}/iconoptions2.png")).unwrap();
}
