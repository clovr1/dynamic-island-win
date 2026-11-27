use island::settings::{MaxMode, Settings};
use island::ui::{ArtData, Env, MediaInfo, Mode, NotifData, Ui};

fn playing() -> MediaInfo {
    MediaInfo { active: true, title: "A".into(), artist: "B".into(), playing: true, pos: 5.0, dur: 200.0 }
}
fn notif(t: &str) -> NotifData {
    NotifData { app: "App".into(), aumid: String::new(), title: t.into(), body: "body".into(), icon: None }
}
fn run(ui: &mut Ui, secs: f32) {
    for _ in 0..(secs * 60.0) as i32 {
        ui.tick(1.0 / 60.0);
    }
}
fn art() -> ArtData {
    ArtData { w: 4, h: 4, rgba: vec![200, 100, 50, 255].repeat(16) }
}

#[test]
fn notifications_can_be_switched_off() {
    let mut ui = Ui::new(1.0);
    let mut s = Settings::default();
    s.notifications = false;
    ui.set_settings(&s);
    ui.push_notification(notif("x"));
    run(&mut ui, 1.0);
    assert_ne!(ui.mode(), Mode::Card);
}

#[test]
fn card_duration_follows_the_setting() {
    for (secs, expect_gone_after) in [(2u32, 3.0f32), (12, 13.0)] {
        let mut ui = Ui::new(1.0);
        let mut s = Settings::default();
        s.card_secs = secs;
        ui.set_settings(&s);
        ui.push_notification(notif("x"));
        run(&mut ui, secs as f32 - 0.5);
        assert_eq!(ui.mode(), Mode::Card, "still there at {}s", secs as f32 - 0.5);
        run(&mut ui, expect_gone_after - secs as f32 + 0.5);
        assert_ne!(ui.mode(), Mode::Card, "gone after {}s", expect_gone_after);
    }
}

#[test]
fn bubble_can_be_switched_off() {
    let mut ui = Ui::new(1.0);
    ui.set_art(Some(art()));
    ui.set_media(playing());
    run(&mut ui, 1.0);
    let mut s = Settings::default();
    s.use_bubble = false;
    ui.set_settings(&s);
    ui.push_notification(notif("x"));
    run(&mut ui, 1.0);
    assert_eq!(ui.mode(), Mode::Card, "a card instead of a bubble");
}

#[test]
fn music_pill_can_be_switched_off() {
    let mut ui = Ui::new(1.0);
    ui.set_media(playing());
    run(&mut ui, 1.0);
    assert_eq!(ui.mode(), Mode::Play);
    let mut s = Settings::default();
    s.show_music_pill = false;
    ui.set_settings(&s);
    assert_eq!(ui.mode(), Mode::Idle);
}

#[test]
fn spotify_in_front_rule_can_be_switched_off() {
    let mut ui = Ui::new(1.0);
    ui.set_media(playing());
    ui.set_env(Env { player_front: true, ..Env::default() });
    run(&mut ui, 1.0);
    assert_eq!(ui.mode(), Mode::Idle);
    let mut s = Settings::default();
    s.hide_when_player_front = false;
    ui.set_settings(&s);
    assert_eq!(ui.mode(), Mode::Play);
}

#[test]
fn maximized_modes() {
    for (mode, hidden_when_playing) in [(MaxMode::Notch, false), (MaxMode::Hide, true), (MaxMode::Normal, false)] {
        let mut ui = Ui::new(1.0);
        let mut s = Settings::default();
        s.maximized = mode;
        ui.set_settings(&s);
        ui.set_media(playing());
        ui.set_env(Env { maximized: true, ..Env::default() });
        run(&mut ui, 1.5);
        assert_eq!(ui.is_hidden(), hidden_when_playing, "{:?}", mode);
        let (_, y, _, _) = ui.rect_px();
        if mode == MaxMode::Notch {
            assert!(y < 1.0, "the notch touches the top edge");
        }
        if mode == MaxMode::Normal {
            assert!(y > 5.0, "unchanged: it floats");
        }
    }
}

#[test]
fn full_screen_rule_can_be_switched_off() {
    let mut ui = Ui::new(1.0);
    let mut s = Settings::default();
    s.fullscreen_hide = false;
    ui.set_settings(&s);
    ui.set_env(Env { fullscreen: true, ..Env::default() });
    run(&mut ui, 1.5);
    assert!(!ui.is_hidden());
}

#[test]
fn speed_changes_how_fast_it_moves() {
    let mut width_after = Vec::new();
    for pct in [50u32, 100, 200] {
        let mut ui = Ui::new(1.0);
        let mut s = Settings::default();
        s.speed_pct = pct;
        ui.set_settings(&s);
        ui.set_media(playing());
        run(&mut ui, 0.2);
        width_after.push(ui.rect_px().2);
    }
    assert!(width_after[0] < width_after[1] && width_after[1] < width_after[2], "{:?}", width_after);
}

#[test]
fn size_changes_the_picture() {
    let mut ui = Ui::new(1.0);
    let before = ui.size_px();
    let after = ui.set_scale(1.3);
    assert!(after.0 > before.0 && after.1 > before.1);
    ui.set_media(playing());
    run(&mut ui, 1.0);
    let _ = ui.render(); // draws at the new size without trouble
}
