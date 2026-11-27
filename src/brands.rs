//! Logos of well-known apps, drawn from the same shapes the Electron version used. They are chosen from the notification's app name,
//! its title, or the website a browser names in it ("www.instagram.com"), so a Chrome notification from Instagram gets the Instagram logo.

use crate::gfx;
use crate::svgpath;
use crate::ui::ArtData;
use tiny_skia::{Color, FillRule, GradientStop, LinearGradient, Paint, PathBuilder, Pixmap, Point, RadialGradient, SpreadMode, Stroke, Transform};

/// (key, names that match the app / title, parts of a website address that match the text)
const BRANDS: [(&str, &[&str], &[&str]); 10] = [
    ("whatsapp", &["whatsapp"], &["web.whatsapp.com"]),
    ("instagram", &["instagram"], &["instagram.com"]),
    ("telegram", &["telegram"], &["web.telegram.org", "t.me"]),
    ("discord", &["discord"], &["discord.com"]),
    ("messenger", &["messenger"], &["messenger.com"]),
    ("facebook", &["facebook"], &["facebook.com"]),
    ("gmail", &["gmail"], &["mail.google.com"]),
    ("youtube", &["youtube"], &["youtube.com", "youtu.be"]),
    ("tiktok", &["tiktok"], &["tiktok.com"]),
    ("x", &["twitter"], &["twitter.com", "x.com"]),
];

/// Which logo belongs to this notification? `rest` is the text after the title (browsers put the site there).
pub fn brand_of(app: &str, title: &str, rest: &str) -> Option<&'static str> {
    let names = [app.trim().to_lowercase(), title.trim().to_lowercase()];
    for n in &names {
        if n.is_empty() {
            continue;
        }
        for (key, words, _) in BRANDS.iter() {
            if words.iter().any(|w| n.contains(w)) {
                return Some(key);
            }
        }
        if n == "x" {
            return Some("x");
        }
    }
    let rest = rest.to_lowercase();
    if !rest.is_empty() {
        for (key, _, doms) in BRANDS.iter() {
            if doms.iter().any(|d| rest.contains(d)) {
                return Some(key);
            }
        }
    }
    None
}

fn paint(c: [u8; 3]) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(c[0], c[1], c[2], 255);
    p.anti_alias = true;
    p
}

fn fill_d(pm: &mut Pixmap, d: &str, c: [u8; 3], t: Transform) {
    if let Some(path) = svgpath::parse(d) {
        pm.fill_path(&path, &paint(c), FillRule::Winding, t, None);
    }
}

fn disc(pm: &mut Pixmap, cx: f32, cy: f32, r: f32, p: &Paint, t: Transform) {
    if let Some(path) = PathBuilder::from_circle(cx, cy, r) {
        pm.fill_path(&path, p, FillRule::Winding, t, None);
    }
}

fn square(pm: &mut Pixmap, c: [u8; 3], t: Transform) {
    if let Some(path) = gfx::rrect(0.0, 0.0, 48.0, 48.0, [0.0; 4]) {
        pm.fill_path(&path, &paint(c), FillRule::Winding, t, None);
    }
}

const WHITE: [u8; 3] = [255, 255, 255];

/// Draw a logo (square picture, `px` pixels).
pub fn render(key: &str, px: u32) -> Option<ArtData> {
    let mut pm = Pixmap::new(px, px)?;
    let k = px as f32 / 48.0;
    let t = Transform::from_scale(k, k);
    match key {
        "whatsapp" => {
            disc(&mut pm, 24.0, 24.0, 24.0, &paint([0x25, 0xD3, 0x66]), t);
            fill_d(&mut pm, "M24 10.5a13.4 13.4 0 0 0-11.5 20.3L10.5 37.5l7-1.9A13.4 13.4 0 1 0 24 10.5zm0 24.3c-2.1 0-4.1-.6-5.8-1.6l-.4-.2-4.1 1.1 1.1-4-.3-.4A11.2 11.2 0 1 1 24 34.8z", WHITE, t);
            fill_d(&mut pm, "M30.2 27.2c-.3-.2-2-1-2.3-1.1-.3-.1-.5-.2-.8.2-.2.3-.9 1.1-1.1 1.3-.2.2-.4.3-.7.1-1.9-.9-3.1-1.7-4.3-3.8-.3-.6.3-.5.9-1.7.1-.2 0-.4 0-.5l-1-2.4c-.3-.6-.5-.5-.8-.5h-.6c-.2 0-.6.1-.9.4-.3.3-1.2 1.2-1.2 2.9 0 1.7 1.2 3.4 1.4 3.6.2.2 2.4 3.7 5.9 5.2 2.2.9 3 1 4.1.8.7-.1 2-.8 2.3-1.6.3-.8.3-1.5.2-1.6-.1-.2-.3-.3-.6-.4z", WHITE, t);
        }
        "instagram" => {
            let stops = vec![
                GradientStop::new(0.0, Color::from_rgba8(0xfd, 0xf4, 0x97, 255)),
                GradientStop::new(0.05, Color::from_rgba8(0xfd, 0xf4, 0x97, 255)),
                GradientStop::new(0.45, Color::from_rgba8(0xfd, 0x59, 0x49, 255)),
                GradientStop::new(0.6, Color::from_rgba8(0xd6, 0x24, 0x9f, 255)),
                GradientStop::new(0.9, Color::from_rgba8(0x28, 0x5a, 0xeb, 255)),
            ];
            let c = Point::from_xy(14.4, 51.36);
            if let Some(sh) = RadialGradient::new(c, c, 67.2, stops, SpreadMode::Pad, Transform::identity()) {
                let mut p = Paint::default();
                p.shader = sh;
                p.anti_alias = true;
                if let Some(path) = gfx::rrect(0.0, 0.0, 48.0, 48.0, [0.0; 4]) {
                    pm.fill_path(&path, &p, FillRule::Winding, t, None);
                }
            }
            let st = Stroke { width: 3.0, ..Stroke::default() };
            if let Some(path) = gfx::rrect(11.5, 11.5, 25.0, 25.0, [7.5; 4]) {
                pm.stroke_path(&path, &paint(WHITE), &st, t, None);
            }
            if let Some(path) = PathBuilder::from_circle(24.0, 24.0, 6.0) {
                pm.stroke_path(&path, &paint(WHITE), &st, t, None);
            }
            disc(&mut pm, 31.4, 16.6, 1.8, &paint(WHITE), t);
        }
        "telegram" => {
            disc(&mut pm, 24.0, 24.0, 24.0, &paint([0x2A, 0xAB, 0xEE]), t);
            fill_d(&mut pm, "M10.8 23.5l24.6-9.5c1.1-.4 2.1.3 1.7 1.9l-4.2 19.7c-.3 1.4-1.1 1.7-2.3 1.1l-6.4-4.7-3.1 3c-.3.3-.6.6-1.3.6l.5-6.5L33.1 19c.5-.5-.1-.7-.8-.3l-14.8 9.3-6.4-2c-1.4-.4-1.4-1.4.3-2.1z", WHITE, t);
        }
        "discord" => {
            square(&mut pm, [0x58, 0x65, 0xF2], t);
            fill_d(&mut pm, "M33.6 15.5a21 21 0 0 0-5.2-1.6l-.7 1.4a19 19 0 0 0-5.4 0l-.7-1.4a21 21 0 0 0-5.2 1.6c-3.3 4.9-4.2 9.7-3.7 14.4a21 21 0 0 0 6.4 3.2l1.4-2.2c-.8-.3-1.5-.7-2.2-1.1l.5-.4a15 15 0 0 0 12.8 0l.5.4c-.7.4-1.4.8-2.2 1.1l1.4 2.2a21 21 0 0 0 6.4-3.2c.6-5.4-1-10.2-3.8-14.4zM19.7 27.5c-1.2 0-2.2-1.1-2.2-2.5s1-2.5 2.2-2.5 2.2 1.1 2.2 2.5-1 2.5-2.2 2.5zm8.6 0c-1.2 0-2.2-1.1-2.2-2.5s1-2.5 2.2-2.5 2.2 1.1 2.2 2.5-1 2.5-2.2 2.5z", WHITE, t);
        }
        "messenger" => {
            let stops = vec![GradientStop::new(0.0, Color::from_rgba8(0x00, 0x99, 0xFF, 255)), GradientStop::new(1.0, Color::from_rgba8(0xA0, 0x33, 0xFF, 255))];
            if let Some(sh) = LinearGradient::new(Point::from_xy(0.0, 48.0), Point::from_xy(48.0, 0.0), stops, SpreadMode::Pad, Transform::identity()) {
                let mut p = Paint::default();
                p.shader = sh;
                p.anti_alias = true;
                disc(&mut pm, 24.0, 24.0, 24.0, &p, t);
            }
            fill_d(&mut pm, "M24 9C15.7 9 9 15.2 9 22.9c0 4.3 2.1 8.1 5.4 10.6V39l5-2.8c1.4.4 2.9.6 4.6.6 8.3 0 15-6.2 15-13.9S32.3 9 24 9zm1.5 18.7l-3.8-4-7.4 4 8.2-8.7 3.9 4 7.3-4-8.2 8.7z", WHITE, t);
        }
        "facebook" => {
            disc(&mut pm, 24.0, 24.0, 24.0, &paint([0x18, 0x77, 0xF2]), t);
            fill_d(&mut pm, "M26.5 41V26.5h4.9l.8-5.7h-5.7v-3.6c0-1.6.5-2.8 2.8-2.8h3V9.3c-.5-.1-2.3-.2-4.4-.2-4.3 0-7.3 2.6-7.3 7.5v4.2h-5v5.7h5V41z", WHITE, t);
        }
        "gmail" => {
            square(&mut pm, WHITE, t);
            fill_d(&mut pm, "M8 14.5v19a2.5 2.5 0 0 0 2.5 2.5H14V21l10 7.5L34 21v15h3.5a2.5 2.5 0 0 0 2.5-2.5v-19L24 26z", [0xEA, 0x43, 0x35], t);
            fill_d(&mut pm, "M34 21v15h3.5a2.5 2.5 0 0 0 2.5-2.5V16z", [0x42, 0x85, 0xF4], t);
            fill_d(&mut pm, "M14 21v15h-3.5A2.5 2.5 0 0 1 8 33.5V16z", [0x34, 0xA8, 0x53], t);
            fill_d(&mut pm, "M34 12v9l6-5v-.8c0-2.5-2.9-3.9-4.8-2.4z", [0xFB, 0xBC, 0x04], t);
            fill_d(&mut pm, "M14 12v9l-6-5v-.8c0-2.5 2.9-3.9 4.8-2.4L24 21l11.2-8.2", [0xC5, 0x22, 0x1F], t);
        }
        "youtube" => {
            square(&mut pm, [0xFF, 0x00, 0x00], t);
            fill_d(&mut pm, "M19 15.5v17l15-8.5z", WHITE, t);
        }
        "tiktok" => {
            square(&mut pm, [0, 0, 0], t);
            let note = "M26.5 10h4.2c.3 3 2.1 5.1 5.3 5.4v4.3c-2 0-3.8-.6-5.3-1.7v8.9c0 4.7-3.5 7.6-7.4 7.6-4.1 0-7.3-3.2-7.3-7.3 0-4.5 3.9-7.8 8.5-7v4.5c-2.1-.6-4 .7-4 2.6 0 1.5 1.2 2.7 2.7 2.7 1.8 0 2.9-1.3 2.9-3.2z";
            fill_d(&mut pm, note, [0x25, 0xF4, 0xEE], t.pre_translate(-1.6, -1.6));
            fill_d(&mut pm, note, [0xFE, 0x2C, 0x55], t.pre_translate(1.6, 1.6));
            fill_d(&mut pm, note, WHITE, t);
        }
        "x" => {
            square(&mut pm, [0, 0, 0], t);
            fill_d(&mut pm, "M14.5 13h5.6l5 7 6-7h3l-7.7 9 9.1 13h-5.6l-5.4-7.6-6.5 7.6h-3l8.3-9.7z", WHITE, t);
        }
        _ => return None,
    }
    let mut rgba = Vec::with_capacity((px * px * 4) as usize);
    for p in pm.data().chunks_exact(4) {
        let a = p[3] as u32;
        let un = |c: u8| if a == 0 { 0 } else { ((c as u32 * 255 + a / 2) / a).min(255) as u8 };
        rgba.extend_from_slice(&[un(p[0]), un(p[1]), un(p[2]), p[3]]);
    }
    Some(ArtData { w: px, h: px, rgba })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_apps_titles_and_websites() {
        assert_eq!(brand_of("Google Chrome", "Instagram", "You have unseen notifications."), Some("instagram"));
        assert_eq!(brand_of("WhatsApp", "Budi", "hi"), Some("whatsapp"));
        assert_eq!(brand_of("Google Chrome", "New message", "www.instagram.com"), Some("instagram"));
        assert_eq!(brand_of("Google Chrome", "Mail", "mail.google.com"), Some("gmail"));
        assert_eq!(brand_of("Settings", "Bluetooth turned on", ""), None);
        assert_eq!(brand_of("X", "post", ""), Some("x"));
    }

    #[test]
    fn every_logo_draws() {
        for (key, _, _) in BRANDS.iter() {
            let a = render(key, 96).expect(key);
            let opaque = a.rgba.chunks_exact(4).filter(|p| p[3] > 200).count();
            assert!(opaque > 96 * 96 / 2, "{key} is mostly empty: {opaque}");
        }
    }
}
