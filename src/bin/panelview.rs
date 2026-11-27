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
    let size = (920, 660);
    for (i, name) in PAGES.iter().enumerate() {
        let mut p = Panel::new(1.0, Settings::default(), true, size);
        p.set_page(i as u8);
        run(&mut p, 1.5);
        p.render().save_png(format!("{out}/page_{i}_{}.png", name.to_lowercase())).unwrap();
    }
    // the theme picker, in a few themes
    for t in [0u8, 3, 5] {
        let mut s = Settings::default();
        s.theme = t;
        let mut p = Panel::new(1.0, s, true, size);
        p.set_page(4);
        run(&mut p, 1.0);
        p.render().save_png(format!("{out}/theme_{t}.png")).unwrap();
    }
    // an unsaved change: the bar at the bottom
    let mut p = Panel::new(1.0, Settings::default(), true, size);
    p.set_page(1);
    let _ = p.press(Id::Pill);
    run(&mut p, 1.0);
    p.render().save_png(format!("{out}/unsaved.png")).unwrap();
    // the island hovered (the player opens), then a notification, then a maximized window
    let mut p = Panel::new(1.0, Settings::default(), true, size);
    p.set_page(1);
    p.render();
    run(&mut p, 0.5);
    p.mouse_move(224.0 + 36.0 + 300.0, 60.0);
    p.render();
    run(&mut p, 1.2);
    p.render().save_png(format!("{out}/preview_open.png")).unwrap();
    p.mouse_move(5.0, 5.0);
    let _ = p.press(Id::DemoMax);
    run(&mut p, 1.5);
    p.render().save_png(format!("{out}/preview_max.png")).unwrap();
    let _ = p.press(Id::DemoNotif);
    run(&mut p, 1.2);
    p.render().save_png(format!("{out}/preview_notif.png")).unwrap();
}
