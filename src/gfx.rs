//! Drawing: rounded shapes, icons, album art and text. Everything is drawn on a premultiplied RGBA pixmap.

use std::collections::HashMap;
use tiny_skia::{FillRule, FilterQuality, LineCap, LineJoin, Mask, Paint, Path, PathBuilder, Pixmap, PixmapPaint, Stroke, Transform};

#[derive(Clone, Copy, PartialEq)]
pub enum Weight {
    Regular,
    Semibold,
    Bold,
}

pub struct Fonts {
    regular: fontdue::Font,
    semibold: fontdue::Font,
    bold: fontdue::Font,
    index: HashMap<(u8, u32, char), usize>,
    store: Vec<(fontdue::Metrics, Vec<u8>)>,
}

impl Fonts {
    pub fn new() -> Fonts {
        let load = |b: &'static [u8]| fontdue::Font::from_bytes(b, fontdue::FontSettings::default()).expect("font");
        Fonts {
            regular: load(include_bytes!("../assets/Inter-Regular.ttf")),
            semibold: load(include_bytes!("../assets/Inter-Semibold.ttf")),
            bold: load(include_bytes!("../assets/Inter-Bold.ttf")),
            index: HashMap::new(),
            store: Vec::new(),
        }
    }

    fn glyph(&mut self, w: Weight, px: f32, c: char) -> usize {
        let wk = match w {
            Weight::Regular => 0u8,
            Weight::Semibold => 1,
            Weight::Bold => 2,
        };
        let key = (wk, (px * 10.0) as u32, c);
        if let Some(&i) = self.index.get(&key) {
            return i;
        }
        let g = match w {
            Weight::Regular => self.regular.rasterize(c, px),
            Weight::Semibold => self.semibold.rasterize(c, px),
            Weight::Bold => self.bold.rasterize(c, px),
        };
        self.store.push(g);
        self.index.insert(key, self.store.len() - 1);
        self.store.len() - 1
    }

    pub fn measure(&mut self, w: Weight, text: &str, px: f32) -> f32 {
        text.chars().map(|c| { let i = self.glyph(w, px, c); self.store[i].0.advance_width }).sum()
    }

    /// Shortens `text` with an ellipsis so it is at most `max` pixels wide.
    pub fn fit(&mut self, w: Weight, text: &str, px: f32, max: f32) -> String {
        if self.measure(w, text, px) <= max {
            return text.to_string();
        }
        let mut s: Vec<char> = text.chars().collect();
        while !s.is_empty() {
            s.pop();
            while s.last() == Some(&' ') {
                s.pop();
            }
            let cand: String = s.iter().collect::<String>() + "…";
            if self.measure(w, &cand, px) <= max {
                return cand;
            }
        }
        "…".to_string()
    }

    /// Draws `text` with its left end at `x` on the given baseline.
    pub fn draw(&mut self, pm: &mut Pixmap, w: Weight, text: &str, x: f32, baseline: f32, px: f32, rgb: [u8; 3], alpha: f32) {
        self.draw_faded(pm, w, text, x, baseline, px, rgb, alpha, None);
    }

    /// Like `draw`, but inside a window `(top, bottom, fade)` (pixels): the text fades out towards the top and bottom edge and is cut off
    /// beyond them (the same soft mask the lyrics had in the Electron version).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_faded(&mut self, pm: &mut Pixmap, w: Weight, text: &str, x: f32, baseline: f32, px: f32, rgb: [u8; 3], alpha: f32, window: Option<(f32, f32, f32)>) {
        if alpha <= 0.003 {
            return;
        }
        let (pw, ph) = (pm.width() as i32, pm.height() as i32);
        let mut cx = x;
        for c in text.chars() {
            let gi = self.glyph(w, px, c);
            let (m, bmp) = &self.store[gi];
            let gx = (cx + m.xmin as f32).round() as i32;
            let gy = (baseline - m.ymin as f32 - m.height as f32).round() as i32;
            let data = pm.data_mut();
            for row in 0..m.height as i32 {
                let y = gy + row;
                if y < 0 || y >= ph {
                    continue;
                }
                let row_a = match window {
                    None => 1.0,
                    Some((top, bottom, fade)) => {
                        let yf = y as f32;
                        if yf < top || yf > bottom {
                            continue;
                        }
                        let t = ((yf - top) / fade).clamp(0.0, 1.0).min(((bottom - yf) / fade).clamp(0.0, 1.0));
                        t * t * (3.0 - 2.0 * t)
                    }
                };
                for col in 0..m.width as i32 {
                    let xx = gx + col;
                    if xx < 0 || xx >= pw {
                        continue;
                    }
                    let cov = bmp[(row as usize) * m.width + col as usize];
                    if cov == 0 {
                        continue;
                    }
                    let a = cov as f32 / 255.0 * alpha * row_a;
                    let i = ((y * pw + xx) * 4) as usize;
                    for k in 0..3 {
                        data[i + k] = (rgb[k] as f32 * a + data[i + k] as f32 * (1.0 - a)).round().min(255.0) as u8;
                    }
                    data[i + 3] = (255.0 * a + data[i + 3] as f32 * (1.0 - a)).round().min(255.0) as u8;
                }
            }
            cx += m.advance_width;
        }
    }
}

/// Rounded rectangle with a separate radius for each corner: top-left, top-right, bottom-right, bottom-left.
pub fn rrect(x: f32, y: f32, w: f32, h: f32, r: [f32; 4]) -> Option<Path> {
    let lim = (w.min(h) / 2.0).max(0.0);
    let r = [r[0].min(lim), r[1].min(lim), r[2].min(lim), r[3].min(lim)];
    const K: f32 = 0.552_284_75;
    let mut p = PathBuilder::new();
    p.move_to(x + r[0], y);
    p.line_to(x + w - r[1], y);
    p.cubic_to(x + w - r[1] + r[1] * K, y, x + w, y + r[1] - r[1] * K, x + w, y + r[1]);
    p.line_to(x + w, y + h - r[2]);
    p.cubic_to(x + w, y + h - r[2] + r[2] * K, x + w - r[2] + r[2] * K, y + h, x + w - r[2], y + h);
    p.line_to(x + r[3], y + h);
    p.cubic_to(x + r[3] - r[3] * K, y + h, x, y + h - r[3] + r[3] * K, x, y + h - r[3]);
    p.line_to(x, y + r[0]);
    p.cubic_to(x, y + r[0] - r[0] * K, x + r[0] - r[0] * K, y, x + r[0], y);
    p.close();
    p.finish()
}

fn paint(rgb: [u8; 3], alpha: f32) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(rgb[0], rgb[1], rgb[2], (alpha.clamp(0.0, 1.0) * 255.0).round() as u8);
    p.anti_alias = true;
    p
}

pub fn fill_rrect(pm: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: [f32; 4], rgb: [u8; 3], alpha: f32) {
    if alpha <= 0.003 || w <= 0.0 || h <= 0.0 {
        return;
    }
    if let Some(path) = rrect(x, y, w, h, r) {
        pm.fill_path(&path, &paint(rgb, alpha), FillRule::Winding, Transform::identity(), None);
    }
}

/// Album art clipped to a rounded square. `art` is square; it is scaled to `size`.
pub fn draw_art(pm: &mut Pixmap, art: &Pixmap, x: f32, y: f32, size: f32, radius: f32, alpha: f32) {
    if alpha <= 0.003 || size <= 1.0 {
        return;
    }
    let Some(path) = rrect(x, y, size, size, [radius; 4]) else { return };
    let Some(mut mask) = Mask::new(pm.width(), pm.height()) else { return };
    mask.fill_path(&path, FillRule::Winding, true, Transform::identity());
    let s = size / art.width() as f32;
    let pp = PixmapPaint { opacity: alpha.clamp(0.0, 1.0), blend_mode: tiny_skia::BlendMode::SourceOver, quality: FilterQuality::Bilinear };
    pm.draw_pixmap(0, 0, art.as_ref(), &pp, Transform::from_row(s, 0.0, 0.0, s, x, y), Some(&mask));
}

// ----- icons (all centred on cx, cy; `s` is the icon's nominal size) -----

pub fn icon_play(pm: &mut Pixmap, cx: f32, cy: f32, s: f32, rgb: [u8; 3], a: f32) {
    let mut p = PathBuilder::new();
    let (w, h) = (s * 0.78, s * 0.9);
    let x0 = cx - w * 0.38;
    p.move_to(x0, cy - h / 2.0);
    p.line_to(x0 + w, cy);
    p.line_to(x0, cy + h / 2.0);
    p.close();
    if let Some(path) = p.finish() {
        let pa = paint(rgb, a);
        pm.fill_path(&path, &pa, FillRule::Winding, Transform::identity(), None);
        let st = Stroke { width: s * 0.16, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Stroke::default() };
        pm.stroke_path(&path, &pa, &st, Transform::identity(), None);
    }
}

pub fn icon_pause(pm: &mut Pixmap, cx: f32, cy: f32, s: f32, rgb: [u8; 3], a: f32) {
    let (bw, bh, gap) = (s * 0.26, s * 0.86, s * 0.2);
    fill_rrect(pm, cx - gap / 2.0 - bw, cy - bh / 2.0, bw, bh, [bw * 0.3; 4], rgb, a);
    fill_rrect(pm, cx + gap / 2.0, cy - bh / 2.0, bw, bh, [bw * 0.3; 4], rgb, a);
}

/// `dir` = -1 for "previous" (triangle pointing left), +1 for "next".
pub fn icon_skip(pm: &mut Pixmap, cx: f32, cy: f32, s: f32, dir: f32, rgb: [u8; 3], a: f32) {
    let (w, h) = (s * 0.5, s * 0.74);
    let tri = |off: f32| {
        let mut p = PathBuilder::new();
        let x0 = cx + dir * off;
        p.move_to(x0 - dir * w / 2.0, cy - h / 2.0);
        p.line_to(x0 + dir * w / 2.0, cy);
        p.line_to(x0 - dir * w / 2.0, cy + h / 2.0);
        p.close();
        p.finish()
    };
    let pa = paint(rgb, a);
    let st = Stroke { width: s * 0.14, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Stroke::default() };
    for off in [-w * 0.42, w * 0.42] {
        if let Some(path) = tri(off) {
            pm.fill_path(&path, &pa, FillRule::Winding, Transform::identity(), None);
            pm.stroke_path(&path, &pa, &st, Transform::identity(), None);
        }
    }
}

/// A speech bubble with two lines of text: the "lyrics" button.
pub fn icon_lyrics(pm: &mut Pixmap, cx: f32, cy: f32, s: f32, rgb: [u8; 3], a: f32) {
    let pa = paint(rgb, a);
    let st = Stroke { width: s * 0.085, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Stroke::default() };
    let (w, h) = (s * 0.86, s * 0.62);
    let (x, y) = (cx - w / 2.0, cy - h / 2.0 - s * 0.06);
    let mut p = PathBuilder::new();
    let r = s * 0.16;
    p.move_to(x + r, y);
    p.line_to(x + w - r, y);
    p.quad_to(x + w, y, x + w, y + r);
    p.line_to(x + w, y + h - r);
    p.quad_to(x + w, y + h, x + w - r, y + h);
    p.line_to(x + w * 0.42, y + h);
    p.line_to(x + w * 0.22, y + h + s * 0.2);
    p.line_to(x + w * 0.25, y + h);
    p.line_to(x + r, y + h);
    p.quad_to(x, y + h, x, y + h - r);
    p.line_to(x, y + r);
    p.quad_to(x, y, x + r, y);
    p.close();
    if let Some(path) = p.finish() {
        pm.stroke_path(&path, &pa, &st, Transform::identity(), None);
    }
    let mut l = PathBuilder::new();
    l.move_to(x + w * 0.22, y + h * 0.36);
    l.line_to(x + w * 0.78, y + h * 0.36);
    l.move_to(x + w * 0.22, y + h * 0.66);
    l.line_to(x + w * 0.6, y + h * 0.66);
    if let Some(path) = l.finish() {
        pm.stroke_path(&path, &pa, &st, Transform::identity(), None);
    }
}

/// A chevron pointing down: "back from lyrics".
pub fn icon_chevron_down(pm: &mut Pixmap, cx: f32, cy: f32, s: f32, rgb: [u8; 3], a: f32) {
    let mut p = PathBuilder::new();
    p.move_to(cx - s * 0.3, cy - s * 0.12);
    p.line_to(cx, cy + s * 0.16);
    p.line_to(cx + s * 0.3, cy - s * 0.12);
    if let Some(path) = p.finish() {
        let st = Stroke { width: s * 0.11, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Stroke::default() };
        pm.stroke_path(&path, &paint(rgb, a), &st, Transform::identity(), None);
    }
}

// ---------- the icons of the Electron version: the very same SVG shapes ----------

use std::sync::OnceLock;

pub struct Icons {
    pub play: Path,
    pub pause: Path,
    pub fwd: Path,
    pub lyrics_bubble: Path,
    pub lyrics_lines: Path,
}

pub fn icons() -> &'static Icons {
    static I: OnceLock<Icons> = OnceLock::new();
    I.get_or_init(|| {
        let p = |d: &str| crate::svgpath::parse(d).expect("icon path");
        Icons {
            play: p("M8 5.14v13.72a1 1 0 0 0 1.5.86l11-6.86a1 1 0 0 0 0-1.72l-11-6.86A1 1 0 0 0 8 5.14z"),
            pause: p("M7 4h3.5a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1zm6.5 0H17a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1h-3.5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1z"),
            fwd: p("M2 6.2v11.6a1 1 0 0 0 1.55.83l8-5.8a1 1 0 0 0 0-1.66l-8-5.8A1 1 0 0 0 2 6.2zM12.2 6.2v11.6a1 1 0 0 0 1.55.83l8-5.8a1 1 0 0 0 0-1.66l-8-5.8a1 1 0 0 0-1.55.83z"),
            lyrics_bubble: p("M5.5 4.5h13a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H12l-4.5 3.5V16.5h-2a2 2 0 0 1-2-2v-8a2 2 0 0 1 2-2z"),
            lyrics_lines: p("M8 9.3h8M8 12.5h5"),
        }
    })
}

/// Fill an icon drawn in a 24x24 box, shown at `size` pixels, centred on (cx, cy). `flip` mirrors it left-right.
pub fn fill_icon(pm: &mut Pixmap, path: &Path, cx: f32, cy: f32, size: f32, flip: bool, rgb: [u8; 3], alpha: f32) {
    if alpha <= 0.003 {
        return;
    }
    let k = size / 24.0;
    let t = if flip {
        Transform::from_row(-k, 0.0, 0.0, k, cx + size / 2.0, cy - size / 2.0)
    } else {
        Transform::from_row(k, 0.0, 0.0, k, cx - size / 2.0, cy - size / 2.0)
    };
    pm.fill_path(path, &paint(rgb, alpha), FillRule::Winding, t, None);
}

/// Stroke an icon (round caps and joins) with a line width given in the 24x24 box.
pub fn stroke_icon(pm: &mut Pixmap, path: &Path, cx: f32, cy: f32, size: f32, width: f32, rgb: [u8; 3], alpha: f32) {
    if alpha <= 0.003 {
        return;
    }
    let k = size / 24.0;
    let t = Transform::from_row(k, 0.0, 0.0, k, cx - size / 2.0, cy - size / 2.0);
    let st = Stroke { width, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Stroke::default() };
    pm.stroke_path(path, &paint(rgb, alpha), &st, t, None);
}

/// hsl(h, s%, l%) -> rgb
pub fn hsl(h: f32, s: f32, l: f32) -> [u8; 3] {
    let (s, l) = (s / 100.0, l / 100.0);
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = (h.rem_euclid(360.0)) / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    [((r + m) * 255.0).round() as u8, ((g + m) * 255.0).round() as u8, ((b + m) * 255.0).round() as u8]
}

/// A circle filled with a CSS-style `linear-gradient(145deg, a, b)`.
pub fn fill_gradient_circle(pm: &mut Pixmap, x: f32, y: f32, size: f32, a: [u8; 3], b: [u8; 3], alpha: f32) {
    use tiny_skia::{Color, GradientStop, LinearGradient, Point, SpreadMode};
    if alpha <= 0.003 {
        return;
    }
    let (cx, cy) = (x + size / 2.0, y + size / 2.0);
    let ang = 145.0f32.to_radians();
    let (dx, dy) = (ang.sin(), -ang.cos());
    let half = size * (ang.sin().abs() + ang.cos().abs()) / 2.0;
    let col = |c: [u8; 3]| Color::from_rgba8(c[0], c[1], c[2], (alpha.clamp(0.0, 1.0) * 255.0) as u8);
    let Some(shader) = LinearGradient::new(Point::from_xy(cx - dx * half, cy - dy * half), Point::from_xy(cx + dx * half, cy + dy * half), vec![GradientStop::new(0.0, col(a)), GradientStop::new(1.0, col(b))], SpreadMode::Pad, Transform::identity()) else { return };
    let mut p = Paint::default();
    p.shader = shader;
    p.anti_alias = true;
    if let Some(path) = rrect(x, y, size, size, [size / 2.0; 4]) {
        pm.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None);
    }
}
