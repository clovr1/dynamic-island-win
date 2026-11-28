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
    // changing the page: the frames of the slide-in
    let mut worst = 0.0f64;
    let mut sum = 0.0f64;
    let mut n = 0;
    for page in [2u8, 4, 1, 3, 5, 0] {
        p.set_page(page);
        for _ in 0..40 {
            p.tick(1.0 / 60.0);
            let t = Instant::now();
            let _ = p.frame();
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            worst = worst.max(ms);
            sum += ms;
            n += 1;
        }
    }
    println!("page change : average {:.2} ms, worst frame {:.2} ms", sum / n as f64, worst);
    // changing the theme: the fade
    p.set_page(4);
    for _ in 0..60 {
        p.tick(1.0 / 60.0);
        let _ = p.frame();
    }
    let _ = p.press(island::panel::Id::Theme(3));
    let (mut worst, mut sum, mut n) = (0.0f64, 0.0f64, 0);
    for _ in 0..40 {
        p.tick(1.0 / 60.0);
        let t = Instant::now();
        let _ = p.frame();
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        worst = worst.max(ms);
        sum += ms;
        n += 1;
    }
    println!("theme change: average {:.2} ms, worst frame {:.2} ms", sum / n as f64, worst);
    println!("island only : {:.2} ms (tick + draw)", t.elapsed().as_secs_f64() * 1000.0 / 200.0);
}
