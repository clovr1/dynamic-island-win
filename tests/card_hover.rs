use island::settings::Settings;
use island::ui::{MediaInfo, Mode, NotifData, Ui};

fn note() -> NotifData {
    NotifData { app: "Dynamic Island".into(), aumid: String::new(), title: "Test notification".into(), body: "If you can see this, cards work".into(), icon: None }
}

fn run(ui: &mut Ui, secs: f32) {
    let mut t = 0.0;
    while t < secs {
        ui.tick(1.0 / 60.0);
        t += 1.0 / 60.0;
    }
}

#[test]
fn card_goes_away_after_hover_idle() {
    let mut ui = Ui::new(1.0);
    ui.set_settings(&Settings::default());
    ui.push_notification(note());
    run(&mut ui, 1.0);
    assert_eq!(ui.mode(), Mode::Card);
    // hover the card (the middle of the island, near the top)
    let (x, y, w, h) = ui.rect_px();
    ui.set_cursor(Some((x + w / 2.0, y + h / 2.0)));
    run(&mut ui, 12.0);
    assert_eq!(ui.mode(), Mode::Card, "stays while hovered");
    ui.set_cursor(None);
    run(&mut ui, 4.0);
    assert_ne!(ui.mode(), Mode::Card, "gone after the mouse left");
}

#[test]
fn card_with_music_returns_to_pill() {
    let mut ui = Ui::new(1.0);
    ui.set_settings(&Settings::default());
    ui.set_media(MediaInfo { active: true, title: "A".into(), artist: "B".into(), playing: true, pos: 5.0, dur: 200.0 });
    run(&mut ui, 1.0);
    ui.push_notification(note());
    run(&mut ui, 1.0);
    let (x, y, w, h) = ui.rect_px();
    ui.set_cursor(Some((x + w / 2.0, y + h / 2.0)));
    run(&mut ui, 3.0);
    ui.set_cursor(None);
    run(&mut ui, 4.0);
    assert_eq!(ui.mode(), Mode::Play, "back to the small pill");
}

#[test]
fn hovered_bubble_becomes_a_card_then_the_pill_without_the_player_popping_open() {
    let mut ui = Ui::new(1.0);
    ui.set_settings(&Settings::default());
    ui.set_media(MediaInfo { active: true, title: "A".into(), artist: "B".into(), playing: true, pos: 5.0, dur: 200.0 });
    run(&mut ui, 1.0);
    ui.push_notification(note());
    run(&mut ui, 1.0);
    assert_eq!(ui.mode(), Mode::Play, "a bubble beside the pill, not a card");
    // hover the bubble: it is absorbed into the card
    let (x, y, w, h) = ui.rect_px();
    ui.set_cursor(Some((x + w - 20.0, y + h / 2.0)));
    run(&mut ui, 1.5);
    assert_eq!(ui.mode(), Mode::Card);
    assert!(!ui.has_bubble(), "the bubble was absorbed");
    // the mouse stays right where it is (the card stays while it is read, at most 25 s): then it ends, and the player must NOT open
    run(&mut ui, 30.0);
    assert_eq!(ui.mode(), Mode::Play, "back to the small pill (mouse still there)");
    assert!(!ui.has_bubble());
    // once the mouse has left and come back, hovering opens the player again
    ui.set_cursor(None);
    run(&mut ui, 1.0);
    let (x, y, w, h) = ui.rect_px();
    ui.set_cursor(Some((x + w / 2.0, y + h / 2.0)));
    run(&mut ui, 1.0);
    assert_eq!(ui.mode(), Mode::Open);
}

#[test]
fn leaving_the_card_ends_it() {
    let mut ui = Ui::new(1.0);
    ui.set_settings(&Settings::default());
    ui.set_media(MediaInfo { active: true, title: "A".into(), artist: "B".into(), playing: true, pos: 5.0, dur: 200.0 });
    run(&mut ui, 1.0);
    ui.push_notification(note());
    run(&mut ui, 1.0);
    let (x, y, w, h) = ui.rect_px();
    ui.set_cursor(Some((x + w - 20.0, y + h / 2.0)));
    run(&mut ui, 1.5);
    assert_eq!(ui.mode(), Mode::Card);
    ui.set_cursor(None);
    run(&mut ui, 2.0);
    assert_eq!(ui.mode(), Mode::Play, "the small pill again, no bubble");
    assert!(!ui.has_bubble());
}

#[test]
fn long_message_unfolds_on_click() {
    let mut ui = Ui::new(1.0);
    ui.set_settings(&Settings::default());
    let long = "Dinner at 7 tonight, don't be late. I made your favourite and everyone is already waiting for you.";
    ui.push_notification(NotifData { app: "Messages".into(), aumid: String::new(), title: "Mum".into(), body: long.into(), icon: None });
    run(&mut ui, 1.0);
    let (_, _, _, h0) = ui.rect_px();
    // hovering alone does not unfold it
    let (x, y, w, h) = ui.rect_px();
    ui.set_cursor(Some((x + w / 2.0, y + h / 2.0)));
    run(&mut ui, 1.5);
    assert!(ui.rect_px().3 < h0 + 5.0, "no unfolding without a click");
    // the first click unfolds, the second one is for the app
    assert!(ui.card_click(), "the click unfolded it");
    run(&mut ui, 1.2);
    let h1 = ui.rect_px().3;
    assert!(h1 > h0 + 15.0, "unfolded: {h0} -> {h1}");
    assert!(!ui.card_click(), "a second click opens the app");
    // a short message is never unfolded: the click opens the app
    let mut ui2 = Ui::new(1.0);
    ui2.set_settings(&Settings::default());
    ui2.push_notification(NotifData { app: "Mail".into(), aumid: String::new(), title: "Nova".into(), body: "Hi".into(), icon: None });
    run(&mut ui2, 1.0);
    assert!(!ui2.card_click());
}
