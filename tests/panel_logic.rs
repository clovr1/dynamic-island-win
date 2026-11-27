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

fn panel() -> Panel {
    Panel::new(1.0, Settings::default(), false, (900, 600))
}

// layout: the list of pages is 224 wide, its items are at y = 84 + 42 * i (38 tall); the rows start at y = 86 and are 68 tall + 6 gap

#[test]
fn pages_and_toggle() {
    let mut p = panel();
    p.render();
    // Music page, the first row (the small pill)
    let _ = click(&mut p, 100.0, 84.0 + 42.0 + 19.0);
    let s = click(&mut p, 500.0, 120.0).expect("toggle changes the settings");
    assert!(!s.show_music_pill);
    let s = click(&mut p, 500.0, 120.0).expect("and back");
    assert!(s.show_music_pill);
}

#[test]
fn slider_and_radio() {
    let mut p = panel();
    p.render();
    // Notifications page: the card-time slider is the third row (y = 86 + 2 * 74 = 234 .. 306)
    let _ = click(&mut p, 100.0, 84.0 + 84.0 + 19.0);
    p.render();
    // the slider's track is right of the middle of the row; find it by dragging across the whole width
    let _ = p.mouse_down(700.0, 270.0);
    let _ = p.mouse_move(10_000.0, 270.0);
    let _ = p.mouse_up(10_000.0, 270.0);
    assert_eq!(p.s.card_secs, 20, "dragged to the right end");
    let _ = p.mouse_down(700.0, 270.0);
    let _ = p.mouse_move(0.0, 270.0);
    let _ = p.mouse_up(0.0, 270.0);
    assert_eq!(p.s.card_secs, 2, "dragged to the left end");
    // Windows page, third option
    let _ = click(&mut p, 100.0, 84.0 + 126.0 + 19.0);
    p.render();
    let s = click(&mut p, 500.0, 86.0 + 68.0 + 2.0 * 42.0 + 20.0).expect("radio option");
    assert_eq!(s.maximized, MaxMode::Normal);
    // pressing on one thing and releasing on another does nothing
    p.render();
    let _ = p.mouse_down(500.0, 86.0 + 68.0 + 20.0);
    assert!(p.mouse_up(500.0, 20.0).is_none());
}

#[test]
fn quit_and_resize() {
    let mut p = panel();
    p.render();
    let _ = p.mouse_down(100.0, 600.0 - 52.0 + 19.0);
    assert!(matches!(p.mouse_up(100.0, 600.0 - 52.0 + 19.0), Some(Act::Quit)));
    p.resize(1400, 900);
    p.render();
    p.resize(700, 460);
    p.wheel(-5.0);
    p.render();
    assert_eq!(p.size_px(), (700, 460));
}
