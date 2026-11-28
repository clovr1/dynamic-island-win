//! The "Copied" pill and the little face in the resting pill.
use island::settings::Settings;
use island::ui::{Mode, Need, StatusData, Ui, TOP, W};

fn run(ui: &mut Ui, secs: f32) {
    let mut t = 0.0;
    while t < secs {
        ui.tick(1.0 / 60.0);
        t += 1.0 / 60.0;
    }
}

fn bright_pixels_in_pill(ui: &mut Ui) -> usize {
    let s = 1.0;
    let img = ui.render().clone();
    let (w, h) = (img.width() as usize, img.height() as usize);
    let mut n = 0;
    for y in (TOP * s) as usize..((TOP + 36.0) * s) as usize {
        for x in ((W / 2.0 - 60.0) * s) as usize..((W / 2.0 + 60.0) * s) as usize {
            if x < w && y < h {
                let p = &img.data()[(y * w + x) * 4..(y * w + x) * 4 + 4];
                if p[3] > 200 && p[0] > 200 && p[1] > 200 && p[2] > 200 {
                    n += 1;
                }
            }
        }
    }
    n
}

#[test]
fn a_copy_shows_a_pill_that_goes_away() {
    let mut ui = Ui::new(1.0);
    ui.set_settings(&Settings::default());
    ui.push_status(StatusData::Copied { kind: "Text".into() });
    run(&mut ui, 0.8);
    assert_eq!(ui.mode(), Mode::Card, "the pill is up");
    run(&mut ui, 5.0);
    assert_eq!(ui.mode(), Mode::Idle, "and it goes away by itself");
}

#[test]
fn the_pill_can_be_switched_off() {
    let mut ui = Ui::new(1.0);
    let mut s = Settings::default();
    s.copied_pill = false;
    ui.set_settings(&s);
    ui.push_status(StatusData::Copied { kind: "Image".into() });
    run(&mut ui, 0.8);
    assert_eq!(ui.mode(), Mode::Idle);
}

#[test]
fn many_announcements_of_one_copy_make_one_pill() {
    let mut ui = Ui::new(1.0);
    ui.set_settings(&Settings::default());
    ui.push_status(StatusData::Copied { kind: "Text".into() });
    ui.push_status(StatusData::Copied { kind: "Text".into() });
    ui.push_status(StatusData::Copied { kind: "Text".into() });
    run(&mut ui, 4.6);
    // nothing queued behind it: after its time the island is at rest again
    assert_eq!(ui.mode(), Mode::Idle);
}

#[test]
fn the_face_is_there_when_nothing_plays_and_can_be_switched_off() {
    let mut ui = Ui::new(1.0);
    ui.set_settings(&Settings::default());
    run(&mut ui, 1.0);
    assert!(bright_pixels_in_pill(&mut ui) > 20, "two eyes");
    let mut s = Settings::default();
    s.mascot = false;
    ui.set_settings(&s);
    run(&mut ui, 1.0);
    assert_eq!(bright_pixels_in_pill(&mut ui), 0, "no face, a plain black pill");
}

#[test]
fn the_face_costs_little_and_nods_off() {
    let mut ui = Ui::new(1.0);
    ui.set_settings(&Settings::default());
    run(&mut ui, 1.0);
    // awake and idle: a slow timer is enough (or a faster one during a blink), never the full frame rate
    assert!(matches!(ui.need(), Need::Idle | Need::Slow));
    // after a long quiet it is asleep: nothing runs at all
    run(&mut ui, 110.0);
    assert_eq!(ui.need(), Need::Rest);
    // with the face off the island rests as before
    let mut s = Settings::default();
    s.mascot = false;
    ui.set_settings(&s);
    run(&mut ui, 1.0);
    assert_eq!(ui.need(), Need::Rest);
}
