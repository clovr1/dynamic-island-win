//! A tiny SVG path reader (only what the icons use: M L H V C A Z, absolute and relative), so the icons are the very same shapes
//! the Electron version used.

use tiny_skia::{Path, PathBuilder};

fn numbers(s: &str, i: &mut usize) -> Option<f32> {
    let b = s.as_bytes();
    while *i < b.len() && (b[*i] == b' ' || b[*i] == b',' || b[*i] == b'\n' || b[*i] == b'\t') {
        *i += 1;
    }
    let start = *i;
    if *i < b.len() && (b[*i] == b'-' || b[*i] == b'+') {
        *i += 1;
    }
    let mut dot = false;
    while *i < b.len() {
        match b[*i] {
            b'0'..=b'9' => *i += 1,
            b'.' if !dot => {
                dot = true;
                *i += 1
            }
            b'e' | b'E' => {
                *i += 1;
                if *i < b.len() && (b[*i] == b'-' || b[*i] == b'+') {
                    *i += 1;
                }
            }
            _ => break,
        }
    }
    if *i == start {
        return None;
    }
    s[start..*i].parse().ok()
}

/// SVG arc -> cubic curves (SVG spec, appendix F.6)
fn arc(pb: &mut PathBuilder, x1: f32, y1: f32, mut rx: f32, mut ry: f32, rot_deg: f32, large: bool, sweep: bool, x2: f32, y2: f32) {
    if rx == 0.0 || ry == 0.0 || (x1 == x2 && y1 == y2) {
        pb.line_to(x2, y2);
        return;
    }
    rx = rx.abs();
    ry = ry.abs();
    let phi = rot_deg.to_radians();
    let (cp, sp) = (phi.cos(), phi.sin());
    let (dx, dy) = ((x1 - x2) / 2.0, (y1 - y2) / 2.0);
    let (x1p, y1p) = (cp * dx + sp * dy, -sp * dx + cp * dy);
    let lam = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
    if lam > 1.0 {
        let s = lam.sqrt();
        rx *= s;
        ry *= s;
    }
    let num = rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p;
    let den = rx * rx * y1p * y1p + ry * ry * x1p * x1p;
    let mut co = (num / den).max(0.0).sqrt();
    if large == sweep {
        co = -co;
    }
    let (cxp, cyp) = (co * rx * y1p / ry, -co * ry * x1p / rx);
    let (cx, cy) = (cp * cxp - sp * cyp + (x1 + x2) / 2.0, sp * cxp + cp * cyp + (y1 + y2) / 2.0);
    let ang = |ux: f32, uy: f32, vx: f32, vy: f32| {
        let a = (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
        a
    };
    let th1 = ang(1.0, 0.0, (x1p - cxp) / rx, (y1p - cyp) / ry);
    let mut dth = ang((x1p - cxp) / rx, (y1p - cyp) / ry, (-x1p - cxp) / rx, (-y1p - cyp) / ry);
    if !sweep && dth > 0.0 {
        dth -= std::f32::consts::TAU;
    } else if sweep && dth < 0.0 {
        dth += std::f32::consts::TAU;
    }
    let segs = (dth.abs() / (std::f32::consts::FRAC_PI_2)).ceil().max(1.0) as i32;
    let step = dth / segs as f32;
    let t = 4.0 / 3.0 * (step / 4.0).tan();
    let mut a = th1;
    for _ in 0..segs {
        let (c1, s1, c2, s2) = (a.cos(), a.sin(), (a + step).cos(), (a + step).sin());
        let p = |x: f32, y: f32| (cp * rx * x - sp * ry * y + cx, sp * rx * x + cp * ry * y + cy);
        let (e1x, e1y) = p(c1 - t * s1, s1 + t * c1);
        let (e2x, e2y) = p(c2 + t * s2, s2 - t * c2);
        let (ex, ey) = p(c2, s2);
        pb.cubic_to(e1x, e1y, e2x, e2y, ex, ey);
        a += step;
    }
}

pub fn parse(d: &str) -> Option<Path> {
    let mut pb = PathBuilder::new();
    let (mut x, mut y, mut sx, mut sy) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    let b = d.as_bytes();
    let mut i = 0;
    let mut cmd = b' ';
    // the second control point of the previous curve (for the smooth S command)
    let mut last_ctrl: Option<(f32, f32)> = None;
    loop {
        while i < b.len() && (b[i] == b' ' || b[i] == b',' || b[i] == b'\n') {
            i += 1;
        }
        if i >= b.len() {
            break;
        }
        if b[i].is_ascii_alphabetic() {
            cmd = b[i];
            i += 1;
            if cmd == b'z' || cmd == b'Z' {
                pb.close();
                x = sx;
                y = sy;
                continue;
            }
        }
        let rel = cmd.is_ascii_lowercase();
        let (ox, oy) = if rel { (x, y) } else { (0.0, 0.0) };
        let prev_ctrl = last_ctrl.take();
        match cmd.to_ascii_uppercase() {
            b'M' => {
                let (nx, ny) = (numbers(d, &mut i)? + ox, numbers(d, &mut i)? + oy);
                pb.move_to(nx, ny);
                x = nx;
                y = ny;
                sx = nx;
                sy = ny;
                cmd = if rel { b'l' } else { b'L' };
            }
            b'L' => {
                let (nx, ny) = (numbers(d, &mut i)? + ox, numbers(d, &mut i)? + oy);
                pb.line_to(nx, ny);
                x = nx;
                y = ny;
            }
            b'H' => {
                let nx = numbers(d, &mut i)? + ox;
                pb.line_to(nx, y);
                x = nx;
            }
            b'V' => {
                let ny = numbers(d, &mut i)? + oy;
                pb.line_to(x, ny);
                y = ny;
            }
            b'C' => {
                let mut v = [0.0f32; 6];
                for (k, item) in v.iter_mut().enumerate() {
                    *item = numbers(d, &mut i)? + if k % 2 == 0 { ox } else { oy };
                }
                pb.cubic_to(v[0], v[1], v[2], v[3], v[4], v[5]);
                last_ctrl = Some((v[2], v[3]));
                x = v[4];
                y = v[5];
            }
            b'S' => {
                let mut v = [0.0f32; 4];
                for (k, item) in v.iter_mut().enumerate() {
                    *item = numbers(d, &mut i)? + if k % 2 == 0 { ox } else { oy };
                }
                // the first control point mirrors the previous curve's second one
                let (c1x, c1y) = match prev_ctrl {
                    Some((px, py)) => (2.0 * x - px, 2.0 * y - py),
                    None => (x, y),
                };
                pb.cubic_to(c1x, c1y, v[0], v[1], v[2], v[3]);
                last_ctrl = Some((v[0], v[1]));
                x = v[2];
                y = v[3];
            }
            b'A' => {
                let (rx, ry, rot) = (numbers(d, &mut i)?, numbers(d, &mut i)?, numbers(d, &mut i)?);
                let (large, sweep) = (numbers(d, &mut i)? != 0.0, numbers(d, &mut i)? != 0.0);
                let (nx, ny) = (numbers(d, &mut i)? + ox, numbers(d, &mut i)? + oy);
                arc(&mut pb, x, y, rx, ry, rot, large, sweep, nx, ny);
                x = nx;
                y = ny;
            }
            _ => return None,
        }
    }
    pb.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn play_triangle_bounds() {
        let p = parse("M8 5.14v13.72a1 1 0 0 0 1.5.86l11-6.86a1 1 0 0 0 0-1.72l-11-6.86A1 1 0 0 0 8 5.14z").unwrap();
        let b = p.bounds();
        assert!((b.left() - 8.0).abs() < 0.2 && (b.right() - 21.0).abs() < 0.3, "{:?}", b);
        assert!((b.top() - 4.1).abs() < 0.3 && (b.bottom() - 19.9).abs() < 0.3, "{:?}", b);
    }

    #[test]
    fn smooth_curves() {
        // the Discord and Messenger logos use "s": they must parse
        assert!(parse("M0 0c1 1 2 1 3 0s2-1 3 0z").is_some());
        assert!(parse("M24 9C15.7 9 9 15.2 9 22.9c0 4.3 2.1 8.1 5.4 10.6V39l5-2.8c1.4.4 2.9.6 4.6.6 8.3 0 15-6.2 15-13.9S32.3 9 24 9z").is_some());
    }

    #[test]
    fn compact_numbers() {
        // "1.5.86" is two numbers (1.5 and .86)
        let p = parse("M0 0a1 1 0 0 0 1.5.86").unwrap();
        assert!(p.bounds().right() > 1.4);
    }
}
