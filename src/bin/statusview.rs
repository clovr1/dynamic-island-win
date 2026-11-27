//! Draws the status pills and the home view to PNG files.
use island::settings::Settings;
use island::ui::{NotifData, StatusData, Ui};

fn run(ui: &mut Ui, secs: f32) {
    let mut t = 0.0;
    while t < secs {
        ui.tick(1.0 / 60.0);
        t += 1.0 / 60.0;
    }
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
        let mut shots: Vec<tiny_skia::Pixmap> = Vec::new();
    let mk = || {
        let mut ui = Ui::new(1.0);
        ui.set_settings(&Settings::default());
        ui.set_clock("Friday, 27 November  14:05".into());
        ui
    };
    for (key, on) in [("wifi", true), ("bluetooth", false), ("airplane", true), ("dnd", true), ("energy", true), ("nightlight", true), ("hotspot", false), ("cellular", true)] {
        let mut ui = mk();
        ui.push_status(StatusData::Sys { key: key.into(), on });
        run(&mut ui, 1.0);
        shots.push(ui.render().clone());
    }
    for (label, level, charging, low) in [("Charging", 64, true, false), ("Unplugged", 64, false, false), ("Low Battery", 18, false, true)] {
        let mut ui = mk();
        ui.push_status(StatusData::Battery { label: label.into(), level, charging, low });
        run(&mut ui, 1.0);
        shots.push(ui.render().clone());
    }
    // home view: empty, then with notifications
    let mut ui = mk();
    ui.set_cursor(Some((235.0, 28.0)));
    run(&mut ui, 1.5);
    shots.push(ui.render().clone());
    for (a, t, b) in [("Messages", "Alex", "Are we still on for tonight?"), ("Mail", "Nova", "Your build finished: 0 errors"), ("Calendar", "Standup", "In 10 minutes - room 4")] {
        ui.push_notification(NotifData { app: a.into(), aumid: String::new(), title: t.into(), body: b.into(), icon: None });
        run(&mut ui, 7.0);
    }
    ui.set_cursor(None);
    run(&mut ui, 12.0);
    ui.set_cursor(Some((235.0, 28.0)));
    run(&mut ui, 1.5);
    shots.push(ui.render().clone());
    let n = shots.len();
    let cols = 3usize;
    let rows = (n + cols - 1) / cols;
    let mut sheet = tiny_skia::Pixmap::new(470 * cols as u32, 150 * rows as u32).unwrap();
    sheet.fill(tiny_skia::Color::from_rgba8(60, 62, 80, 255));
    for (i, p) in shots.iter().enumerate() {
        let (cx, cy) = ((i % cols) as i32 * 470, (i / cols) as i32 * 150);
        sheet.draw_pixmap(cx, cy - 0, p.as_ref(), &tiny_skia::PixmapPaint::default(), tiny_skia::Transform::identity(), None);
    }
    sheet.save_png(format!("{out}/status_sheet.png")).unwrap();
}
