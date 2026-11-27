use island::panel::{Act, Panel};
use island::settings::{MaxMode, Settings};

fn click(p: &mut Panel, x: f32, y: f32) -> Option<Settings> {
    p.render();
    let _ = p.mouse_move(x, y);
    let down = p.mouse_down(x, y);
    let up = p.mouse_up(x, y);
    for a in [down, up].into_iter().flatten() {
        if let Act::Changed(s) = a {
            return Some(s);
        }
    }
    None
}

#[test]
fn toggle_slider_segment() {
    let mut p = Panel::new(1.0, Settings::default(), false, 1400.0);
    // the music pill row
    let s = click(&mut p, 200.0, 231.0).expect("toggle changes the settings");
    assert!(!s.show_music_pill);
    let s = click(&mut p, 200.0, 231.0).expect("and back");
    assert!(s.show_music_pill);
    // the card-time slider: the middle of the track
    p.render();
    let _ = p.mouse_down(220.0, 576.0).map(|a| matches!(a, Act::Changed(_)));
    let a = p.mouse_move(220.0, 576.0);
    let _ = a;
    let _ = p.mouse_up(220.0, 576.0);
    assert!(p.s.card_secs > 8 && p.s.card_secs < 14, "card_secs = {}", p.s.card_secs);
    // the third segment
    let s = click(&mut p, 340.0, 755.0).expect("segment");
    assert_eq!(s.maximized, MaxMode::Normal);
    // a click that goes down on one thing and up on another does nothing
    p.render();
    let _ = p.mouse_down(200.0, 231.0);
    assert!(p.mouse_up(200.0, 600.0).is_none());
}

#[test]
fn header_and_close() {
    let mut p = Panel::new(1.0, Settings::default(), false, 900.0);
    p.render();
    assert!(matches!(p.mouse_down(100.0, 30.0), Some(Act::DragWindow)));
    let _ = p.mouse_down(396.0, 40.0);
    assert!(matches!(p.mouse_up(396.0, 40.0), Some(Act::Close)));
}

#[test]
fn scrolls_when_short() {
    let mut p = Panel::new(1.0, Settings::default(), false, 500.0);
    assert_eq!(p.size_px().1, 500);
    p.render();
    p.wheel(-3.0);
    p.render();
    p.wheel(-30.0);
    p.render(); // (does not run past the end)
}
