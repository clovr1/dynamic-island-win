//! Draws the settings window to PNG files (to look at the design without Windows).
use island::panel::{Id, Panel, PAGES};
use island::settings::Settings;

fn run(p: &mut Panel, secs: f32) {
    let mut t = 0.0;
    while t < secs {
        p.tick(1.0 / 60.0);
        t += 1.0 / 60.0;
    }
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let size = (980, 790);
    for (i, name) in PAGES.iter().enumerate() {
        let mut p = Panel::new(1.0, Settings::default(), true, size);
        p.set_page(i as u8);
        run(&mut p, 2.0);
        p.render().save_png(format!("{out}/page_{i}_{}.png", name.to_lowercase())).unwrap();
    }
    // an unsaved change, the island open, a notification
    let mut p = Panel::new(1.0, Settings::default(), true, size);
    p.set_page(1);
    run(&mut p, 1.0);
    let _ = p.press(Id::Pill);
    run(&mut p, 1.0);
    p.mouse_move(490.0, 90.0);
    run(&mut p, 1.2);
    p.render().save_png(format!("{out}/unsaved_open.png")).unwrap();
}
