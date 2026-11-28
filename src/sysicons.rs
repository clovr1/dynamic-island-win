//! The icons of the status pills (Wi-Fi, Bluetooth, airplane mode ...): the very same SVG shapes as the Electron version.

use crate::gfx::rrect;
use crate::svgpath;
use std::sync::OnceLock;
use tiny_skia::{LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap, Stroke, Transform};

/// The name shown in the pill and the colour of the icon while the setting is on.
pub fn info(key: &str) -> (&'static str, [u8; 3]) {
    match key {
        "dnd" => ("Do Not Disturb", [125, 122, 255]),
        "wifi" => ("Wi-Fi", [10, 132, 255]),
        "bluetooth" => ("Bluetooth", [10, 132, 255]),
        "airplane" => ("Airplane Mode", [255, 159, 10]),
        "cellular" => ("Cellular", [48, 209, 88]),
        "hotspot" => ("Hotspot", [48, 209, 88]),
        "nightlight" => ("Night Light", [255, 179, 64]),
        "energy" => ("Energy Saver", [255, 214, 10]),
        "clipboard" => ("Copied", [100, 210, 255]),
        _ => ("", [10, 132, 255]),
    }
}

struct Shapes {
    moon: Path,
    stars: Path,
    wifi_arcs: Path,
    wifi_dot: Path,
    bt: Path,
    plane: Path,
    bars: Path,
    hot_dot: Path,
    hot_waves: Path,
    nl_half: Path,
    nl_ring: Path,
    en_body: Path,
    en_cap: Path,
    en_fill: Path,
    slash: Path,
    bolt: Path,
    clip_board: Path,
    clip_tab: Path,
    clip_check: Path,
}

fn circle(cx: f32, cy: f32, r: f32) -> Path {
    let mut pb = PathBuilder::new();
    pb.push_circle(cx, cy, r);
    pb.finish().unwrap()
}

fn shapes() -> &'static Shapes {
    static S: OnceLock<Shapes> = OnceLock::new();
    S.get_or_init(|| {
        let p = |d: &str| svgpath::parse(d).expect("icon path");
        let mut stars = PathBuilder::new();
        stars.push_circle(16.4, 7.4, 1.15);
        stars.push_circle(19.6, 11.0, 0.8);
        let mut bars = PathBuilder::new();
        for (x, y, h) in [(3.0, 14.5, 5.5), (8.2, 10.8, 9.2), (13.4, 7.0, 13.0), (18.6, 3.2, 16.8)] {
            if let Some(r) = rrect(x, y, 3.4, h, [1.2; 4]) {
                bars.push_path(&r);
            }
        }
        Shapes {
            moon: p("M20.2 14.6A8.4 8.4 0 0 1 9.4 3.8a.5.5 0 0 0-.7-.6A9.4 9.4 0 1 0 20.8 15.3a.5.5 0 0 0-.6-.7z"),
            stars: stars.finish().unwrap(),
            wifi_arcs: p("M2.5 9.2a14 14 0 0 1 19 0M5.7 12.5a9.4 9.4 0 0 1 12.6 0M8.9 15.8a4.8 4.8 0 0 1 6.2 0"),
            wifi_dot: circle(12.0, 19.2, 1.4),
            bt: p("M7 7.5l10 9-5 4.5V3l5 4.5-10 9"),
            plane: p("M21.5 15.3v-1.6l-8.2-5V4.4a1.6 1.6 0 0 0-3.2 0v4.3l-8.2 5v1.6l8.2-2.4v4.2l-2.1 1.6v1.5l3.7-1 3.7 1v-1.5l-2.1-1.6v-4.2z"),
            bars: bars.finish().unwrap(),
            hot_dot: circle(12.0, 12.5, 1.7),
            hot_waves: p("M8.8 9.3a4.6 4.6 0 0 0 0 6.4M15.2 9.3a4.6 4.6 0 0 1 0 6.4M6 6.6a8.6 8.6 0 0 0 0 11.8M18 6.6a8.6 8.6 0 0 1 0 11.8"),
            nl_half: p("M12 3a9 9 0 1 0 0 18z"),
            nl_ring: circle(12.0, 12.0, 9.0),
            en_body: rrect(2.0, 7.0, 18.0, 10.4, [3.3; 4]).unwrap(),
            en_cap: rrect(21.0, 10.6, 1.9, 3.2, [0.95; 4]).unwrap(),
            en_fill: rrect(4.4, 9.4, 13.2, 5.6, [1.8; 4]).unwrap(),
            slash: p("M4 3.5L20 20.5"),
            bolt: p("M13.2 2.5L5 13.4h5.6l-1.1 8.1L18.2 10h-5.7z"),
            clip_board: rrect(5.0, 4.6, 14.0, 17.2, [2.6; 4]).unwrap(),
            clip_tab: rrect(8.4, 2.4, 7.2, 4.2, [1.7; 4]).unwrap(),
            clip_check: p("M8.8 14.2l2.3 2.4 4.3-5"),
        }
    })
}

fn paint(rgb: [u8; 3], a: f32) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(rgb[0], rgb[1], rgb[2], (a.clamp(0.0, 1.0) * 255.0).round() as u8);
    p.anti_alias = true;
    p
}

/// Draw the icon of `key` in a 24-unit box shown at `size` pixels, centred on (cx, cy). `on` = the setting is on (off: grey, with a slash
/// through the radio icons). `pop`: 0..1, the little entrance (the icon grows with a small overshoot).
pub fn draw(pm: &mut Pixmap, key: &str, cx: f32, cy: f32, size: f32, rgb: [u8; 3], alpha: f32, on: bool, pop: f32) {
    let sh = shapes();
    let k = size / 24.0;
    let grow = {
        let x = pop.clamp(0.0, 1.0) - 1.0;
        let c1 = 1.70158;
        0.55 + 0.45 * (1.0 + (c1 + 1.0) * x * x * x + c1 * x * x)
    };
    let a = alpha * pop.clamp(0.0, 1.0).min(1.0).max(0.0).sqrt();
    let k = k * grow;
    let t = Transform::from_row(k, 0.0, 0.0, k, cx - 12.0 * k, cy - 12.0 * k);
    let fill = |pm: &mut Pixmap, path: &Path| pm.fill_path(path, &paint(rgb, a), tiny_skia::FillRule::Winding, t, None);
    let stroke = |pm: &mut Pixmap, path: &Path, w: f32| {
        let st = Stroke { width: w, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Default::default() };
        pm.stroke_path(path, &paint(rgb, a), &st, t, None);
    };
    let slash = |pm: &mut Pixmap| {
        if !on {
            // a darker outline first, so the slash cuts through the icon
            let st = Stroke { width: 4.4, line_cap: LineCap::Round, ..Default::default() };
            pm.stroke_path(&sh.slash, &paint([0, 0, 0], a), &st, t, None);
            stroke(pm, &sh.slash, 2.0);
        }
    };
    match key {
        "dnd" => {
            fill(pm, &sh.moon);
            fill(pm, &sh.stars);
        }
        "wifi" => {
            stroke(pm, &sh.wifi_arcs, 1.9);
            fill(pm, &sh.wifi_dot);
            slash(pm);
        }
        "bluetooth" => {
            stroke(pm, &sh.bt, 1.9);
            slash(pm);
        }
        "airplane" => fill(pm, &sh.plane),
        "cellular" => {
            fill(pm, &sh.bars);
            slash(pm);
        }
        "hotspot" => {
            fill(pm, &sh.hot_dot);
            stroke(pm, &sh.hot_waves, 1.9);
            slash(pm);
        }
        "nightlight" => {
            fill(pm, &sh.nl_half);
            stroke(pm, &sh.nl_ring, 1.9);
        }
        "energy" => {
            stroke(pm, &sh.en_body, 1.8);
            fill(pm, &sh.en_cap);
            if on {
                fill(pm, &sh.en_fill);
            }
        }
        "bolt" => fill(pm, &sh.bolt),
        "clipboard" => {
            stroke(pm, &sh.clip_board, 1.8);
            fill(pm, &sh.clip_tab);
            stroke(pm, &sh.clip_check, 1.9);
        }
        _ => {}
    }
}

/// Windows announces quick-setting changes as toasts from "Settings" ("Bluetooth turned on"). Those become the island's own pill.
pub fn toast_to_sys(app: &str, title: &str) -> Option<(&'static str, bool)> {
    let app = app.trim().to_lowercase();
    if app != "settings" && app != "windows settings" {
        return None;
    }
    let t = title.trim().to_lowercase();
    let key = if t.contains("bluetooth") {
        "bluetooth"
    } else if t.contains("wi-fi") || t.contains("wifi") {
        "wifi"
    } else if t.contains("airplane") {
        "airplane"
    } else if t.contains("night light") {
        "nightlight"
    } else if t.contains("hotspot") {
        "hotspot"
    } else if t.contains("energy saver") || t.contains("battery saver") {
        "energy"
    } else {
        return None;
    };
    let t = t.trim_end_matches('.').trim();
    let on = if t.ends_with(" on") || t.contains("turned on") || t.contains("is on") || t.contains("now on") {
        true
    } else if t.ends_with(" off") || t.contains("turned off") || t.contains("is off") || t.contains("now off") {
        false
    } else {
        return None;
    };
    Some((key, on))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toasts() {
        assert_eq!(toast_to_sys("Settings", "Bluetooth turned on"), Some(("bluetooth", true)));
        assert_eq!(toast_to_sys("Windows Settings", "Airplane mode is off."), Some(("airplane", false)));
        assert_eq!(toast_to_sys("settings", "Wi-Fi is now on"), Some(("wifi", true)));
        assert_eq!(toast_to_sys("Settings", "Night light on"), Some(("nightlight", true)));
        assert_eq!(toast_to_sys("Chrome", "Bluetooth turned on"), None);
        assert_eq!(toast_to_sys("Settings", "Update available"), None);
    }
}
