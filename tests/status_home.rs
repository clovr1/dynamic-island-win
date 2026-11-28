use island::settings::Settings;
use island::ui::{Hit, Mode, NotifData, StatusData, Ui};

fn run(ui: &mut Ui, secs: f32) {
    let mut t = 0.0;
    while t < secs {
        ui.tick(1.0 / 60.0);
        t += 1.0 / 60.0;
    }
}

fn note(app: &str, title: &str) -> NotifData {
    NotifData { app: app.into(), aumid: format!("{app}!app"), title: title.into(), body: "hello".into(), icon: None }
}

fn ui() -> Ui {
    let mut ui = Ui::new(1.0);
    ui.set_settings(&Settings::default());
    ui
}

#[test]
fn status_pill_shows_and_goes() {
    let mut ui = ui();
    ui.push_status(StatusData::Sys { key: "wifi".into(), on: true });
    run(&mut ui, 1.0);
    assert_eq!(ui.mode(), Mode::Card);
    let (_, _, w, h) = ui.rect_px();
    assert!(w < 290.0 && h < 50.0, "slim pill: {w} x {h}");
    run(&mut ui, 4.0);
    assert_ne!(ui.mode(), Mode::Card);
    // the same change twice in a row: shown once
    ui.push_status(StatusData::Sys { key: "wifi".into(), on: true });
    run(&mut ui, 4.0);
    ui.push_status(StatusData::Sys { key: "wifi".into(), on: true });
    run(&mut ui, 0.5);
    assert_ne!(ui.mode(), Mode::Card, "the repeat is ignored");
}

#[test]
fn status_pill_can_be_switched_off() {
    let mut ui = ui();
    let mut s = Settings::default();
    s.status_pills = false;
    ui.set_settings(&s);
    ui.push_status(StatusData::Battery { label: "Charging".into(), level: 50, charging: true, low: false });
    run(&mut ui, 1.0);
    assert_ne!(ui.mode(), Mode::Card);
}

#[test]
fn dnd_keeps_notifications_quiet_but_listed() {
    let mut ui = ui();
    ui.set_dnd(true);
    ui.push_notification(note("Messages", "Alex"));
    run(&mut ui, 1.0);
    assert_ne!(ui.mode(), Mode::Card, "nothing pops up");
    assert!(ui.has_log(), "but it is in the list");
    ui.set_dnd(false);
    ui.push_notification(note("Mail", "Nova"));
    run(&mut ui, 1.0);
    assert_eq!(ui.mode(), Mode::Card);
}

#[test]
fn home_view_lists_and_opens() {
    let mut ui = ui();
    ui.set_dnd(true);
    ui.push_notification(note("Messages", "Alex"));
    ui.push_notification(note("Mail", "Nova"));
    // hover the empty island: the home view opens, 2 rows
    ui.set_cursor(Some((235.0, 28.0)));
    run(&mut ui, 1.5);
    assert_eq!(ui.mode(), Mode::Open);
    let (x, y, w, _) = ui.rect_px();
    // the first row (newest: Mail)
    let (cx, cy) = (x + w / 2.0, y + 50.0 + 20.0);
    assert!(matches!(ui.hit(cx, cy), Hit::HomeRow(0)));
    let aumid = ui.home_take(0);
    assert_eq!(aumid, "Mail!app");
    run(&mut ui, 1.0);
    assert_ne!(ui.mode(), Mode::Open, "closes after a click, even with the mouse still there");
    ui.clear_log();
    assert!(!ui.has_log());
}

#[test]
fn sounds_are_asked_for() {
    use island::ui::Sound;
    let mut ui = ui();
    ui.push_notification(note("Messages", "Alex"));
    run(&mut ui, 0.5);
    assert!(ui.take_sounds().contains(&Sound::Notify), "a card = the Milestone tone");
    // while music plays the notification is a small bubble: the Droplet tone
    let mut m = Ui::new(1.0);
    m.set_settings(&Settings::default());
    m.set_media(island::ui::MediaInfo { active: true, title: "A".into(), artist: "B".into(), playing: true, pos: 1.0, dur: 100.0 });
    run(&mut m, 1.0);
    m.push_notification(note("Messages", "Alex"));
    assert!(m.take_sounds().contains(&Sound::Bubble));
    ui.push_status(StatusData::Sys { key: "wifi".into(), on: true });
    assert!(ui.take_sounds().iter().any(|s| matches!(s, Sound::Status(_))));
    // every pill has its own sound: wifi on / off, battery charging / unplugged / low are all different
    use island::ui::status_sound_code as code;
    let a = code(&StatusData::Sys { key: "wifi".into(), on: true });
    let b = code(&StatusData::Sys { key: "wifi".into(), on: false });
    let c = code(&StatusData::Sys { key: "bluetooth".into(), on: true });
    let d = code(&StatusData::Battery { label: "x".into(), level: 50, charging: true, low: false });
    let e = code(&StatusData::Battery { label: "x".into(), level: 10, charging: false, low: true });
    let all = [a, b, c, d, e];
    for i in 0..all.len() {
        for j in i + 1..all.len() {
            assert_ne!(all[i], all[j]);
        }
    }
    assert!(all.iter().all(|c| *c < 19));
    // hovering (opening / closing the island) is silent
    let mut ui = Ui::new(1.0);
    ui.set_settings(&Settings::default());
    ui.set_cursor(Some((235.0, 28.0)));
    run(&mut ui, 1.5);
    ui.set_cursor(None);
    run(&mut ui, 2.0);
    assert!(ui.take_sounds().is_empty(), "no sound for hovering");
    // the buttons do make a sound
    ui.play_sound(Sound::Skip);
    assert_eq!(ui.take_sounds(), vec![Sound::Skip]);
}

#[test]
fn a_test_notification_ignores_do_not_disturb_but_real_ones_do_not() {
    let mut ui = ui();
    ui.set_dnd(true);
    ui.push_notification(note("Messages", "Alex"));
    run(&mut ui, 0.5);
    assert_ne!(ui.mode(), Mode::Card, "a real one is kept quiet");
    ui.push_test_notification(note("Bloop", "Test"));
    run(&mut ui, 0.5);
    assert_eq!(ui.mode(), Mode::Card, "the test is shown");
    // with "Respect Do Not Disturb" off, real notifications show too
    let mut ui = ui_with_respect(false);
    ui.set_dnd(true);
    ui.push_notification(note("Messages", "Alex"));
    run(&mut ui, 0.5);
    assert_eq!(ui.mode(), Mode::Card);
}

fn ui_with_respect(respect: bool) -> Ui {
    let mut u = Ui::new(1.0);
    let mut s = Settings::default();
    s.dnd_quiet = respect;
    u.set_settings(&s);
    u
}
