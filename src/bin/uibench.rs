use island::settings::Settings;
use island::ui::{MediaInfo, Ui};
use std::time::Instant;
fn main() {
    for scale in [1.0f32, 1.25] {
        let mut ui = Ui::new(scale);
        ui.set_settings(&Settings::default());
        ui.set_media(MediaInfo { active: true, title: "Song".into(), artist: "Artist".into(), playing: true, pos: 5.0, dur: 200.0 });
        for _ in 0..60 {
            ui.tick(1.0 / 60.0);
        }
        let t = Instant::now();
        for _ in 0..300 {
            ui.tick(1.0 / 60.0);
            let _ = ui.render();
        }
        println!("scale {scale}: pill tick+render {:.2} ms", t.elapsed().as_secs_f64() * 1000.0 / 300.0);
        ui.set_cursor(Some((235.0 * scale, 28.0 * scale)));
        for _ in 0..60 {
            ui.tick(1.0 / 60.0);
        }
        let t = Instant::now();
        for _ in 0..300 {
            ui.tick(1.0 / 60.0);
            let _ = ui.render();
        }
        println!("scale {scale}: open player tick+render {:.2} ms", t.elapsed().as_secs_f64() * 1000.0 / 300.0);
    }
}
