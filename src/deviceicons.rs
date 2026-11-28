//! Little "3D" pictures of the Bluetooth devices that can connect (a speaker, headphones, a controller ...): shaded shapes with a highlight and a
//! soft shadow underneath, drawn from simple pieces.

use tiny_skia::{FillRule, GradientStop, LinearGradient, Paint, PathBuilder, Pixmap, Point, SpreadMode, Stroke, Transform};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Speaker,
    Headphones,
    /// small wireless earbuds (AirPods and the like)
    Earbuds,
    Controller,
    Keyboard,
    Mouse,
    Phone,
    Watch,
    Other,
}

/// Which kind of device is it? From the Bluetooth "class of device" number when there is one, otherwise from the name.
pub fn classify(class: Option<u32>, name: &str) -> Kind {
    let n = name.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| n.contains(w));
    // earbuds are told by their name first (their "class" is just "headset")
    if has(&["airpods", "buds", "earbuds", "pods", "wf-", "freebuds", "jabra elite", "powerbeats"]) {
        return Kind::Earbuds;
    }
    if let Some(c) = class.filter(|c| *c != 0) {
        let (major, minor) = ((c >> 8) & 0x1F, (c >> 2) & 0x3F);
        match major {
            4 => {
                return match minor {
                    5 | 8 | 0x0A | 0x0C => Kind::Speaker,   // loudspeaker, car audio, hi-fi, video display + loudspeaker
                    1 | 2 | 6 | 7 => Kind::Headphones,       // headset, hands-free, headphones, portable audio
                    _ => {
                        if has(&["speaker", "soundbar", "boom", "flip", "charge", "jbl", "echo"]) { Kind::Speaker } else { Kind::Headphones }
                    }
                };
            }
            5 => {
                // a keyboard (bit 4), a pointing device (bit 5), or a joystick / gamepad (the low four bits)
                if minor & 0x10 != 0 {
                    return Kind::Keyboard;
                }
                if minor & 0x20 != 0 {
                    return Kind::Mouse;
                }
                if matches!(minor & 0x0F, 1 | 2) {
                    return Kind::Controller;
                }
            }
            2 => return Kind::Phone,
            7 => return Kind::Watch,
            8 => return Kind::Controller,
            _ => {}
        }
    }
    if has(&["controller", "gamepad", "xbox", "dualsense", "dualshock", "joy-con", "joycon", "pro controller", "8bitdo"]) {
        Kind::Controller
    } else if has(&["keyboard", "keychron", "k380", "mx keys"]) {
        Kind::Keyboard
    } else if has(&["mouse", "mx master", "trackpad", "magic mouse"]) {
        Kind::Mouse
    } else if has(&["buds", "airpods", "headphone", "headset", "earbuds", "earphone", "wh-", "wf-", "quietcomfort", "momentum"]) {
        Kind::Headphones
    } else if has(&["speaker", "soundbar", "boom", "megaboom", "flip", "charge", "jbl", "echo", "homepod", "marshall"]) {
        Kind::Speaker
    } else if has(&["watch", "band", "fitbit"]) {
        Kind::Watch
    } else if has(&["phone", "iphone", "galaxy", "pixel", "redmi", "oppo", "vivo"]) {
        Kind::Phone
    } else {
        Kind::Other
    }
}

fn grad(top: [u8; 3], bottom: [u8; 3], y0: f32, y1: f32) -> Paint<'static> {
    let mut p = Paint::default();
    p.shader = LinearGradient::new(
        Point::from_xy(0.0, y0),
        Point::from_xy(0.0, y1),
        vec![GradientStop::new(0.0, tiny_skia::Color::from_rgba8(top[0], top[1], top[2], 255)), GradientStop::new(1.0, tiny_skia::Color::from_rgba8(bottom[0], bottom[1], bottom[2], 255))],
        SpreadMode::Pad,
        Transform::identity(),
    )
    .unwrap_or(tiny_skia::Shader::SolidColor(tiny_skia::Color::BLACK));
    p.anti_alias = true;
    p
}

fn solid(c: [u8; 3], a: f32) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(c[0], c[1], c[2], (a.clamp(0.0, 1.0) * 255.0) as u8);
    p.anti_alias = true;
    p
}

/// A rounded box filled with a top-to-bottom shade.
fn shaded(pm: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, top: [u8; 3], bottom: [u8; 3], alpha: f32) {
    if let Some(path) = crate::gfx::rrect(x, y, w, h, [r; 4]) {
        let mut p = grad(top, bottom, y, y + h);
        if alpha < 0.999 {
            // (the shade is drawn at full strength; fading is done by drawing it into a layer elsewhere - here simply dimmer)
            p.shader = LinearGradient::new(
                Point::from_xy(0.0, y),
                Point::from_xy(0.0, y + h),
                vec![
                    GradientStop::new(0.0, tiny_skia::Color::from_rgba8(top[0], top[1], top[2], (alpha * 255.0) as u8)),
                    GradientStop::new(1.0, tiny_skia::Color::from_rgba8(bottom[0], bottom[1], bottom[2], (alpha * 255.0) as u8)),
                ],
                SpreadMode::Pad,
                Transform::identity(),
            )
            .unwrap_or(tiny_skia::Shader::SolidColor(tiny_skia::Color::BLACK));
        }
        pm.fill_path(&path, &p, FillRule::Winding, Transform::identity(), None);
    }
}

fn disc(pm: &mut Pixmap, cx: f32, cy: f32, r: f32, c: [u8; 3], a: f32) {
    let mut pb = PathBuilder::new();
    pb.push_circle(cx, cy, r);
    if let Some(p) = pb.finish() {
        pm.fill_path(&p, &solid(c, a), FillRule::Winding, Transform::identity(), None);
    }
}

fn shade_disc(pm: &mut Pixmap, cx: f32, cy: f32, r: f32, top: [u8; 3], bottom: [u8; 3], a: f32) {
    shaded(pm, cx - r, cy - r, 2.0 * r, 2.0 * r, r, top, bottom, a);
}

/// The colour of the soft glow behind a device.
fn glow_of(kind: Kind) -> [u8; 3] {
    match kind {
        Kind::Speaker => [90, 160, 255],
        Kind::Headphones => [190, 140, 255],
        Kind::Earbuds => [160, 200, 255],
        Kind::Controller => [150, 120, 255],
        Kind::Keyboard | Kind::Mouse => [120, 200, 255],
        Kind::Phone => [120, 150, 255],
        Kind::Watch => [255, 150, 110],
        Kind::Other => [80, 150, 255],
    }
}

/// The device, floating: a soft glow behind it, it rises and sinks a little with `lift` (0..1, from a slow wave), and its shadow on the
/// ground gets smaller and fainter as it rises.
pub fn draw_floating(pm: &mut Pixmap, kind: Kind, cx: f32, cy: f32, s: f32, a: f32, lift: f32) {
    if a <= 0.01 {
        return;
    }
    let g = glow_of(kind);
    let mut paint = Paint::default();
    let r = s * 0.78;
    let (gx, gy) = (cx, cy - lift * s * 0.04);
    paint.shader = tiny_skia::RadialGradient::new(
        Point::from_xy(gx, gy),
        Point::from_xy(gx, gy),
        r,
        vec![
            GradientStop::new(0.0, tiny_skia::Color::from_rgba8(g[0], g[1], g[2], (70.0 * a * (0.7 + 0.3 * lift)) as u8)),
            GradientStop::new(1.0, tiny_skia::Color::from_rgba8(g[0], g[1], g[2], 0)),
        ],
        SpreadMode::Pad,
        Transform::identity(),
    )
    .unwrap_or(tiny_skia::Shader::SolidColor(tiny_skia::Color::TRANSPARENT));
    if let Some(rect) = tiny_skia::Rect::from_xywh(gx - r, gy - r, 2.0 * r, 2.0 * r) {
        pm.fill_rect(rect, &paint, Transform::identity(), None);
    }
    draw_lifted(pm, kind, cx, cy - lift * s * 0.07, s, a, lift);
}

/// Draw the device picture in a square of side `s` centred on (cx, cy). `a` is 0..1.
pub fn draw(pm: &mut Pixmap, kind: Kind, cx: f32, cy: f32, s: f32, a: f32) {
    draw_lifted(pm, kind, cx, cy, s, a, 0.0);
}

fn draw_lifted(pm: &mut Pixmap, kind: Kind, cx: f32, cy: f32, s: f32, a: f32, lift: f32) {
    if a <= 0.01 {
        return;
    }
    let (x0, y0) = (cx - s / 2.0, cy - s / 2.0);
    let u = s / 100.0; // 100 units across
    let p = |x: f32| x0 + x * u;
    let q = |y: f32| y0 + y * u;
    // the shadow on the "ground"
    // (the picture is lifted by `lift`: the shadow stays on the ground, and becomes smaller and fainter)
    let (sw, sa) = (72.0 * (1.0 - 0.22 * lift), 0.35 - 0.16 * lift);
    let mut sh = PathBuilder::new();
    sh.push_oval(tiny_skia::Rect::from_xywh(p(50.0 - sw / 2.0), q(86.0) + lift * 0.07 * s, sw * u, (9.0 - 2.0 * lift) * u).unwrap());
    if let Some(path) = sh.finish() {
        pm.fill_path(&path, &solid([0, 0, 0], sa * a), FillRule::Winding, Transform::identity(), None);
    }
    match kind {
        Kind::Speaker => {
            shaded(pm, p(24.0), q(6.0), 52.0 * u, 80.0 * u, 16.0 * u, [150, 154, 168], [48, 50, 60], a);
            shaded(pm, p(28.0), q(8.0), 8.0 * u, 70.0 * u, 4.0 * u, [255, 255, 255], [150, 154, 168], 0.35 * a); // a highlight down the left
            shade_disc(pm, p(50.0), q(24.0), 8.0 * u, [30, 32, 40], [70, 74, 86], a);
            disc(pm, p(50.0), q(24.0), 3.0 * u, [150, 154, 168], 0.8 * a);
            shade_disc(pm, p(50.0), q(58.0), 19.0 * u, [20, 21, 28], [74, 78, 92], a);
            shade_disc(pm, p(50.0), q(58.0), 12.0 * u, [48, 50, 62], [14, 15, 20], a);
            disc(pm, p(50.0), q(58.0), 4.0 * u, [96, 100, 114], 0.9 * a);
        }
        Kind::Headphones => {
            // the band
            let mut pb = PathBuilder::new();
            pb.move_to(p(22.0), q(58.0));
            pb.cubic_to(p(22.0), q(10.0), p(78.0), q(10.0), p(78.0), q(58.0));
            if let Some(path) = pb.finish() {
                pm.stroke_path(&path, &solid([70, 74, 88], a), &Stroke { width: 9.0 * u, line_cap: tiny_skia::LineCap::Round, ..Default::default() }, Transform::identity(), None);
                pm.stroke_path(&path, &solid([170, 174, 190], 0.5 * a), &Stroke { width: 2.5 * u, ..Default::default() }, Transform::from_translate(0.0, -2.5 * u), None);
            }
            // the ear cups
            shaded(pm, p(12.0), q(50.0), 24.0 * u, 36.0 * u, 11.0 * u, [160, 164, 180], [44, 46, 58], a);
            shaded(pm, p(64.0), q(50.0), 24.0 * u, 36.0 * u, 11.0 * u, [160, 164, 180], [44, 46, 58], a);
            shaded(pm, p(18.0), q(58.0), 12.0 * u, 20.0 * u, 6.0 * u, [40, 42, 52], [20, 21, 28], 0.9 * a);
            shaded(pm, p(70.0), q(58.0), 12.0 * u, 20.0 * u, 6.0 * u, [40, 42, 52], [20, 21, 28], 0.9 * a);
        }
        Kind::Earbuds => {
            // two small earbuds, tilted a little, floating side by side
            for (bx, tilt) in [(32.0f32, -1.0f32), (68.0, 1.0)] {
                // the stem
                shaded(pm, p(bx - 4.5 + tilt * 3.0), q(40.0), 9.0 * u, 42.0 * u, 4.5 * u, [255, 255, 255], [196, 200, 214], a);
                // the head
                shade_disc(pm, p(bx), q(34.0), 16.0 * u, [255, 255, 255], [190, 194, 210], a);
                disc(pm, p(bx - tilt * 5.0), q(36.0), 5.0 * u, [150, 154, 170], 0.55 * a); // the speaker mesh
                shaded(pm, p(bx - 9.0), q(22.0), 8.0 * u, 3.0 * u, 1.5 * u, [255, 255, 255], [255, 255, 255], 0.7 * a); // a glint
            }
        }
        Kind::Controller => {
            shaded(pm, p(6.0), q(28.0), 88.0 * u, 44.0 * u, 20.0 * u, [150, 154, 170], [52, 54, 66], a);
            // the grips
            shade_disc(pm, p(24.0), q(66.0), 18.0 * u, [120, 124, 140], [40, 42, 52], a);
            shade_disc(pm, p(76.0), q(66.0), 18.0 * u, [120, 124, 140], [40, 42, 52], a);
            shaded(pm, p(12.0), q(30.0), 76.0 * u, 12.0 * u, 6.0 * u, [255, 255, 255], [190, 194, 208], 0.3 * a);
            // the stick and the cross
            shade_disc(pm, p(30.0), q(48.0), 8.0 * u, [26, 27, 34], [80, 84, 98], a);
            disc(pm, p(62.0), q(60.0), 0.0, [0, 0, 0], 0.0);
            shaded(pm, p(52.0), q(58.0), 20.0 * u, 6.0 * u, 3.0 * u, [30, 31, 38], [50, 52, 62], a);
            shaded(pm, p(59.0), q(51.0), 6.0 * u, 20.0 * u, 3.0 * u, [30, 31, 38], [50, 52, 62], a);
            // the four buttons
            for (dx, dy, c) in [(0.0, -9.0, [255, 205, 60]), (9.0, 0.0, [255, 90, 90]), (0.0, 9.0, [70, 200, 120]), (-9.0, 0.0, [80, 150, 255])] {
                disc(pm, p(76.0 + dx * 0.7), q(46.0 + dy * 0.7), 3.6 * u, c, a);
            }
        }
        Kind::Keyboard => {
            shaded(pm, p(4.0), q(30.0), 92.0 * u, 48.0 * u, 9.0 * u, [160, 164, 180], [60, 62, 76], a);
            for row in 0..3 {
                let n = if row == 2 { 5 } else { 8 };
                let off = if row == 2 { 20.0 } else { 0.0 };
                for k in 0..n {
                    let w = if row == 2 && k == 2 { 28.0 } else { 8.0 };
                    let x = if row == 2 && k > 2 { 12.0 + 8.0 * (k - 3) as f32 + 28.0 + 3.0 } else { 10.0 + off + 10.0 * k as f32 };
                    if x + w > 92.0 {
                        continue;
                    }
                    shaded(pm, p(x), q(36.0 + 13.0 * row as f32), w * u, 9.0 * u, 2.0 * u, [236, 238, 246], [168, 172, 188], 0.9 * a);
                }
            }
        }
        Kind::Mouse => {
            shaded(pm, p(30.0), q(8.0), 40.0 * u, 76.0 * u, 20.0 * u, [170, 174, 190], [54, 56, 70], a);
            shaded(pm, p(48.5), q(10.0), 3.0 * u, 26.0 * u, 1.5 * u, [30, 31, 38], [20, 21, 28], 0.7 * a);
            shaded(pm, p(46.0), q(18.0), 8.0 * u, 14.0 * u, 4.0 * u, [255, 255, 255], [190, 194, 208], 0.85 * a);
            shaded(pm, p(34.0), q(10.0), 6.0 * u, 60.0 * u, 3.0 * u, [255, 255, 255], [170, 174, 190], 0.25 * a);
        }
        Kind::Phone => {
            shaded(pm, p(30.0), q(4.0), 40.0 * u, 82.0 * u, 9.0 * u, [190, 194, 208], [70, 72, 86], a);
            shaded(pm, p(33.0), q(8.0), 34.0 * u, 74.0 * u, 6.0 * u, [30, 34, 60], [10, 12, 28], a);
            shaded(pm, p(35.0), q(12.0), 30.0 * u, 20.0 * u, 4.0 * u, [90, 130, 255], [140, 90, 255], 0.85 * a);
            disc(pm, p(50.0), q(11.0), 1.4 * u, [150, 154, 170], 0.9 * a);
        }
        Kind::Watch => {
            shaded(pm, p(38.0), q(4.0), 24.0 * u, 26.0 * u, 6.0 * u, [90, 94, 110], [44, 46, 58], a);
            shaded(pm, p(38.0), q(66.0), 24.0 * u, 22.0 * u, 6.0 * u, [90, 94, 110], [44, 46, 58], a);
            shaded(pm, p(28.0), q(24.0), 44.0 * u, 48.0 * u, 13.0 * u, [190, 194, 208], [66, 68, 82], a);
            shaded(pm, p(32.0), q(28.0), 36.0 * u, 40.0 * u, 10.0 * u, [20, 22, 40], [8, 9, 20], a);
            disc(pm, p(50.0), q(48.0), 9.0 * u, [255, 255, 255], 0.0);
            shaded(pm, p(49.0), q(36.0), 2.4 * u, 13.0 * u, 1.2 * u, [255, 255, 255], [200, 204, 218], a);
            shaded(pm, p(49.0), q(47.0), 10.0 * u, 2.4 * u, 1.2 * u, [255, 140, 90], [255, 100, 90], a);
        }
        Kind::Other => {
            // the Bluetooth sign on a blue ball
            shade_disc(pm, p(50.0), q(46.0), 40.0 * u, [120, 175, 255], [20, 80, 220], a);
            let mut pb = PathBuilder::new();
            pb.move_to(p(38.0), q(35.0));
            pb.line_to(p(62.0), q(57.0));
            pb.line_to(p(50.0), q(68.0));
            pb.line_to(p(50.0), q(24.0));
            pb.line_to(p(62.0), q(35.0));
            pb.line_to(p(38.0), q(57.0));
            if let Some(path) = pb.finish() {
                pm.stroke_path(&path, &solid([255, 255, 255], a), &Stroke { width: 4.6 * u, line_cap: tiny_skia::LineCap::Round, line_join: tiny_skia::LineJoin::Round, ..Default::default() }, Transform::identity(), None);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_from_class_and_name() {
        // audio / video: loudspeaker (minor 5) and headphones (minor 6): class = major 4 << 8 | minor << 2
        assert_eq!(classify(Some(4 << 8 | 5 << 2), "X"), Kind::Speaker);
        assert_eq!(classify(Some(4 << 8 | 6 << 2), "X"), Kind::Headphones);
        assert_eq!(classify(Some(5 << 8 | 0x10 << 2), "X"), Kind::Keyboard);
        assert_eq!(classify(Some(5 << 8 | 0x20 << 2), "X"), Kind::Mouse);
        assert_eq!(classify(Some(5 << 8 | 2 << 2), "X"), Kind::Controller);
        assert_eq!(classify(None, "Xbox Wireless Controller"), Kind::Controller);
        assert_eq!(classify(None, "JBL Flip 6"), Kind::Speaker);
        assert_eq!(classify(None, "WH-1000XM5"), Kind::Headphones);
        assert_eq!(classify(None, "AirPods Pro"), Kind::Earbuds);
        assert_eq!(classify(Some(4 << 8 | 1 << 2), "Galaxy Buds2"), Kind::Earbuds);
        assert_eq!(classify(Some(0), "Something"), Kind::Other);
    }
}
