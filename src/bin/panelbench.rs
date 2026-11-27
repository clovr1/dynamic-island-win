//! How long does a frame of the settings window take? (full redraw vs. only the live island)
use island::panel::Panel;
use island::settings::Settings;
use std::time::Instant;

fn main() {
    let mut p = Panel::new(1.25, Settings::default(), true, (1150, 825));
    p.render();
    for _ in 0..30 {
        p.tick(1.0 / 60.0);
        let _ = p.frame();
    }
    let t = Instant::now();
    for _ in 0..200 {
        p.render();
    }
    println!("full redraw : {:.2} ms", t.elapsed().as_secs_f64() * 1000.0 / 200.0);
    let t = Instant::now();
    for _ in 0..200 {
        p.tick(1.0 / 60.0);
    }
    println!("tick only   : {:.2} ms", t.elapsed().as_secs_f64() * 1000.0 / 200.0);
    let t = Instant::now();
    for _ in 0..200 {
        let _ = p.frame();
    }
    println!("frame only  : {:.2} ms", t.elapsed().as_secs_f64() * 1000.0 / 200.0);
    let t = Instant::now();
    for _ in 0..200 {
        p.tick(1.0 / 60.0);
        let _ = p.frame();
    }
    println!("island only : {:.2} ms (tick + draw)", t.elapsed().as_secs_f64() * 1000.0 / 200.0);
}
