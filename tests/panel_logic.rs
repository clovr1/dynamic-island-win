use island::panel::{Act, Id, Panel};
use island::settings::{MaxMode, Settings};

fn panel() -> Panel {
    Panel::new(1.0, Settings::default(), false, (920, 660))
}

fn changed(a: Option<Act>) -> Option<Settings> {
    match a {
        Some(Act::Changed(s)) => Some(s),
        _ => None,
    }
}

#[test]
fn pages_and_toggle() {
    let mut p = panel();
    p.render();
    p.set_page(1);
    let s = changed(p.press(Id::Pill)).expect("toggle changes the settings");
    assert!(!s.show_music_pill);
    let s = changed(p.press(Id::Pill)).expect("and back");
    assert!(s.show_music_pill);
}

#[test]
fn slider_and_radio() {
    let mut p = panel();
    p.set_page(2);
    // a dial: the mouse straight above its centre (12 o'clock) is the middle of its range
    let (cx, cy) = p.center_of(Id::CardSecs).expect("the dial");
    let s = changed(p.mouse_down(cx, cy - 30.0)).expect("dial");
    let _ = p.mouse_up(cx, cy - 30.0);
    assert!(s.card_secs > 8 && s.card_secs < 14, "middle of the dial: {}", s.card_secs);
    // all the way round to the left stop (7 o'clock) and the right stop (5 o'clock)
    let _ = p.mouse_down(cx - 20.0, cy + 20.0);
    assert_eq!(p.s.card_secs, 2);
    let _ = p.mouse_move(cx + 20.0, cy + 20.0);
    assert_eq!(p.s.card_secs, 20);
    let _ = p.mouse_up(cx + 20.0, cy + 20.0);
    p.set_page(3);
    let s = changed(p.press(Id::MaxOpt(2))).expect("radio option");
    assert_eq!(s.maximized, MaxMode::Normal);
    // pressing on one thing and releasing on another does nothing
    p.render();
    let _ = p.mouse_down(500.0, 400.0);
    assert!(p.mouse_up(500.0, 20.0).is_none());
}

#[test]
fn preview_buttons_and_quit() {
    let mut p = panel();
    // the preview's own buttons do not change any setting
    assert!(p.press(Id::DemoMax).is_none());
    assert!(p.press(Id::DemoNotif).is_none());
    for _ in 0..120 {
        p.tick(1.0 / 60.0);
    }
    p.render();
    assert!(matches!(p.press(Id::Quit), Some(Act::Quit)));
    p.resize(1400, 900);
    p.render();
    p.resize(700, 460);
    p.wheel(-5.0);
    p.render();
    assert_eq!(p.size_px(), (700, 460));
}

#[test]
fn the_preview_follows_the_settings() {
    let mut p = panel();
    p.set_page(4);
    let before = p.render().data().to_vec();
    let s = changed(p.press(Id::Scale)).expect("size slider");
    assert!(s.scale_pct != 100);
    for _ in 0..90 {
        p.tick(1.0 / 60.0);
    }
    let after = p.render().data().to_vec();
    assert_ne!(before, after, "the live island looks different");
}

#[test]
fn changes_wait_for_save() {
    let mut p = panel();
    assert!(!p.dirty());
    p.set_page(1);
    let s = changed(p.press(Id::Pill)).expect("edited");
    assert!(!s.show_music_pill);
    assert!(p.dirty(), "an unsaved change");
    // Save hands the draft out and clears the mark once the window says it is applied
    p.render();
    match p.press(Id::Save) {
        Some(Act::Save(s, _)) => assert!(!s.show_music_pill),
        _ => panic!("Save should give the settings out"),
    }
    p.mark_saved();
    assert!(!p.dirty());
    // Discard goes back to what was saved
    let _ = p.press(Id::Pill);
    assert!(p.dirty());
    p.render();
    let _ = p.press(Id::Discard);
    assert!(!p.dirty());
    assert!(!p.s.show_music_pill);
    // the start-with-Windows switch is a draft too
    p.set_page(0);
    let _ = p.press(Id::Autostart);
    assert!(p.dirty());
}
