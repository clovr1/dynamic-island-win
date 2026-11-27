//! Draws the app icon (assets/app.ico + assets/app.png). Run: cargo run --release --bin makeicon
use island::gfx::fill_rrect;
use tiny_skia::{Color, FillRule, GradientStop, LinearGradient, Paint, PathBuilder, Pixmap, Point, RadialGradient, SpreadMode, Stroke, Transform};

fn icon(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    // the tile: a rounded square with a violet -> cyan gradient
    let r = s * 0.225;
    let tile = island::gfx::rrect(s * 0.03, s * 0.03, s * 0.94, s * 0.94, [r; 4]).unwrap();
    let mut p = Paint::default();
    p.anti_alias = true;
    p.shader = LinearGradient::new(
        Point::from_xy(0.0, 0.0),
        Point::from_xy(s, s),
        vec![GradientStop::new(0.0, Color::from_rgba8(138, 64, 255, 255)), GradientStop::new(0.55, Color::from_rgba8(52, 120, 255, 255)), GradientStop::new(1.0, Color::from_rgba8(0, 214, 200, 255))],
        SpreadMode::Pad,
        Transform::identity(),
    )
    .unwrap();
    pm.fill_path(&tile, &p, FillRule::Winding, Transform::identity(), None);
    // a soft light from the top
    let mut g = Paint::default();
    g.anti_alias = true;
    g.shader = RadialGradient::new(Point::from_xy(s * 0.5, s * 0.1), Point::from_xy(s * 0.5, s * 0.1), s * 0.75, vec![GradientStop::new(0.0, Color::from_rgba8(255, 255, 255, 80)), GradientStop::new(1.0, Color::from_rgba8(255, 255, 255, 0))], SpreadMode::Pad, Transform::identity()).unwrap();
    pm.fill_path(&tile, &g, FillRule::Winding, Transform::identity(), None);

    // ripples spreading from the island (dynamic!)
    let cx = s * 0.5;
    let top = s * 0.27;
    for (k, a) in [(0.34f32, 70u8), (0.50, 42), (0.66, 22)] {
        let mut pb = PathBuilder::new();
        let rad = s * k;
        pb.push_oval(tiny_skia::Rect::from_xywh(cx - rad, top - rad * 0.55, rad * 2.0, rad * 1.1).unwrap());
        let path = pb.finish().unwrap();
        let mut sp = Paint::default();
        sp.set_color_rgba8(255, 255, 255, a);
        sp.anti_alias = true;
        let mut mask = tiny_skia::Mask::new(n, n).unwrap();
        mask.fill_path(&tile, FillRule::Winding, true, Transform::identity());
        pm.stroke_path(&path, &sp, &Stroke { width: (s * 0.012).max(1.0), ..Default::default() }, Transform::identity(), Some(&mask));
    }

    // the island: a black pill with the cover and the equalizer
    let (pw, ph) = (s * 0.62, s * 0.22);
    let (px, py) = (cx - pw / 2.0, s * 0.16);
    fill_rrect(&mut pm, px, py + s * 0.012, pw, ph, [ph / 2.0; 4], [0, 0, 0], 0.28); // a little shadow
    fill_rrect(&mut pm, px, py, pw, ph, [ph / 2.0; 4], [8, 8, 12], 1.0);
    let d = ph * 0.58;
    fill_rrect(&mut pm, px + ph * 0.22, py + (ph - d) / 2.0, d, d, [d * 0.3; 4], [255, 138, 76], 1.0);
    fill_rrect(&mut pm, px + ph * 0.22 + d * 0.22, py + (ph - d) / 2.0 + d * 0.18, d * 0.6, d * 0.6, [d * 0.3; 4], [255, 80, 150], 0.85);
    let bw = (ph * 0.085).max(1.0);
    for (i, h) in [0.30f32, 0.52, 0.38, 0.6].iter().enumerate() {
        let bh = ph * h;
        fill_rrect(&mut pm, px + pw - ph * 0.62 + i as f32 * bw * 1.85, py + ph / 2.0 - bh / 2.0, bw, bh, [bw / 2.0; 4], [255, 255, 255], 0.95);
    }
    // under the island: a floating song card hint (two lines) so it reads as "now playing"
    fill_rrect(&mut pm, cx - s * 0.2, s * 0.6, s * 0.4, s * 0.05, [s * 0.025; 4], [255, 255, 255], 0.9);
    fill_rrect(&mut pm, cx - s * 0.14, s * 0.7, s * 0.28, s * 0.04, [s * 0.02; 4], [255, 255, 255], 0.55);
    pm
}

fn main() {
    let sizes = [16u32, 20, 24, 32, 40, 48, 64, 128, 256];
    let mut pngs: Vec<(u32, Vec<u8>)> = Vec::new();
    for n in sizes {
        pngs.push((n, icon(n).encode_png().unwrap()));
    }
    // ICO file with PNG pictures
    let mut out: Vec<u8> = vec![0, 0, 1, 0];
    out.extend_from_slice(&(pngs.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * pngs.len();
    for (n, d) in &pngs {
        out.push(if *n >= 256 { 0 } else { *n as u8 });
        out.push(if *n >= 256 { 0 } else { *n as u8 });
        out.extend_from_slice(&[0, 0, 1, 0, 32, 0]);
        out.extend_from_slice(&(d.len() as u32).to_le_bytes());
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += d.len();
    }
    for (_, d) in &pngs {
        out.extend_from_slice(d);
    }
    std::fs::write("assets/app.ico", out).unwrap();
    icon(512).save_png("assets/app.png").unwrap();
    println!("assets/app.ico written ({} pictures)", pngs.len());
}
