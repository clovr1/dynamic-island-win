//! A sheet of the device pictures.
use island::deviceicons::{draw, Kind};
use tiny_skia::{Color, Pixmap};

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let mut pm = Pixmap::new(8 * 150, 170).unwrap();
    pm.fill(Color::BLACK);
    for (i, k) in [Kind::Speaker, Kind::Headphones, Kind::Controller, Kind::Keyboard, Kind::Mouse, Kind::Phone, Kind::Watch, Kind::Other].iter().enumerate() {
        draw(&mut pm, *k, 75.0 + i as f32 * 150.0, 85.0, 130.0, 1.0);
    }
    pm.save_png(format!("{out}/devices.png")).unwrap();
}
