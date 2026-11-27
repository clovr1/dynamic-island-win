//! Draws the settings window's contents to PNG files (to look at the design without Windows).
use island::panel::{Panel, PAGES};
use island::settings::Settings;

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    for (i, name) in PAGES.iter().enumerate() {
        let mut p = Panel::new(1.0, Settings::default(), true, (900, 600));
        p.set_page(i as u8);
        p.render().save_png(format!("{out}/page_{i}_{}.png", name.to_lowercase())).unwrap();
    }
}
