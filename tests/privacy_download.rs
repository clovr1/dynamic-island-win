//! The microphone / camera dot and the download pill.
use island::settings::Settings;
use island::ui::{Mode, NotifData, StatusData, Ui, TOP, W};

fn run(ui: &mut Ui, secs: f32) {
    let mut t = 0.0;
    while t < secs {
        ui.tick(1.0 / 60.0);
        t += 1.0 / 60.0;
    }
}

/// Pixels that are not transparent in the strip left of the resting pill (where the dot sits).
fn pixels_left_of_pill(ui: &mut Ui) -> usize {
    let img = ui.render().clone();
    let w = img.width() as usize;
    let mut n = 0;
    for y in TOP as usize..(TOP + 36.0) as usize {
        for x in ((W / 2.0 - 90.0) as usize)..((W / 2.0 - 66.0) as usize) {
            if img.data()[(y * w + x) * 4 + 3] > 200 {
                n += 1;
            }
        }
    }
    n
}

fn dl(name: &str, bytes: u64, done: bool) -> StatusData {
    StatusData::Download { name: name.into(), bytes, done }
}

#[test]
fn a_program_that_starts_listening_gets_a_pill_and_a_dot() {
    let mut ui = Ui::new(1.0);
    ui.set_settings(&Settings::default());
    run(&mut ui, 0.5);
    assert_eq!(pixels_left_of_pill(&mut ui), 0, "nothing beside the pill");
    ui.set_privacy(Some("Chrome".into()), None, false);
    run(&mut ui, 0.8);
    assert_eq!(ui.mode(), Mode::Card, "a pill says which program");
    run(&mut ui, 5.0);
    assert_eq!(ui.mode(), Mode::Idle);
    assert!(pixels_left_of_pill(&mut ui) > 100, "the dot stays beside the island while the microphone is in use");
    ui.set_privacy(None, None, false);
    run(&mut ui, 1.5);
    assert_eq!(pixels_left_of_pill(&mut ui), 0, "and goes when the microphone is free again");
}

#[test]
fn the_first_reading_is_only_remembered() {
    let mut ui = Ui::new(1.0);
    ui.set_settings(&Settings::default());
    ui.set_privacy(None, Some("Zoom".into()), true);
    run(&mut ui, 0.8);
    assert_eq!(ui.mode(), Mode::Idle, "no pill for something that was already running");
    assert!(pixels_left_of_pill(&mut ui) > 100, "but the dot is there");
}

#[test]
fn the_dot_can_be_switched_off() {
    let mut ui = Ui::new(1.0);
    let mut s = Settings::default();
    s.privacy_dots = false;
    ui.set_settings(&s);
    ui.set_privacy(Some("Chrome".into()), None, false);
    run(&mut ui, 1.0);
    assert_eq!(ui.mode(), Mode::Idle);
    assert_eq!(pixels_left_of_pill(&mut ui), 0);
}

#[test]
fn a_download_pill_follows_the_progress_without_stacking_up() {
    let mut ui = Ui::new(1.0);
    ui.set_settings(&Settings::default());
    ui.push_status(dl("film.mp4", 1_000_000, false));
    run(&mut ui, 1.0);
    assert_eq!(ui.mode(), Mode::Card);
    // every second the size grows: it stays up, nothing queues behind it
    for i in 2..8 {
        ui.push_status(dl("film.mp4", 1_000_000 * i, false));
        run(&mut ui, 1.0);
        assert_eq!(ui.mode(), Mode::Card);
    }
    // the updates stop: it goes away by itself
    run(&mut ui, 5.0);
    assert_eq!(ui.mode(), Mode::Idle);
}

#[test]
fn a_finished_download_says_so_and_a_message_comes_first() {
    let mut ui = Ui::new(1.0);
    ui.set_settings(&Settings::default());
    ui.push_status(dl("film.mp4", 5_000_000, false));
    run(&mut ui, 1.0);
    ui.push_status(dl("film.mp4", 7_000_000, true));
    run(&mut ui, 1.0);
    assert_eq!(ui.mode(), Mode::Card, "the finished pill");
    run(&mut ui, 5.0);
    assert_eq!(ui.mode(), Mode::Idle);
    // a message is not kept waiting by a running download
    ui.push_status(dl("big.iso", 1_000_000, false));
    run(&mut ui, 1.0);
    ui.push_notification(NotifData { app: "Messages".into(), aumid: String::new(), title: "Budi".into(), body: "Hi".into(), icon: None });
    run(&mut ui, 1.2);
    ui.push_status(dl("big.iso", 2_000_000, false));
    run(&mut ui, 0.3);
    assert_eq!(ui.mode(), Mode::Card);
    // the message card is the one on show (a running download does not replace it): after its time everything is quiet again
    run(&mut ui, 8.0);
    assert_eq!(ui.mode(), Mode::Idle);
}

#[test]
fn the_download_pill_can_be_switched_off() {
    let mut ui = Ui::new(1.0);
    let mut s = Settings::default();
    s.download_pill = false;
    ui.set_settings(&s);
    ui.push_status(dl("film.mp4", 1_000_000, false));
    run(&mut ui, 1.0);
    assert_eq!(ui.mode(), Mode::Idle);
}
