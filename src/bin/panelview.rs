//! Draws the settings panel to PNG files (to look at its design without Windows).
use island::panel::{Act, Panel};
use island::settings::Settings;

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let scale = 1.0;
    let mut p = Panel::new(scale, Settings::default(), true, 1200.0);
    p.render().save_png(format!("{out}/panel_full.png")).unwrap();
    println!("size {:?}", p.size_px());

    // interaction: a toggle, a slider drag, the segmented control, hover on a button
    let mut p = Panel::new(scale, Settings::default(), true, 700.0);
    p.render();
    p.wheel(-1.0);
    p.render();
    let _ = p.mouse_move(200.0, 300.0);
    let mut acts = 0;
    for (x, y) in [(300.0, 150.0)] {
        let _ = p.mouse_down(x, y);
        if let Some(Act::Changed(_)) = p.mouse_up(x, y) {
            acts += 1;
        }
    }
    println!("toggle acts: {acts}");
    p.render().save_png(format!("{out}/panel_scroll.png")).unwrap();
}
