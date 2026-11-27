//! Random sequences of everything the island can do, checking every frame for stray drawing or broken numbers.
//! `cargo run --release --bin fuzz`
use island::lyrics::{parse_lrc, Found};
use island::ui::{ArtData, Env, MediaInfo, NotifData, Ui, TOP, W};

const S: f32 = 1.0;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
    fn pick(&mut self, n: u32) -> u32 {
        self.next() % n
    }
}

fn art() -> ArtData {
    ArtData { w: 8, h: 8, rgba: (0..64).flat_map(|i| [(i * 3) as u8, 120, 200, 255]).collect() }
}

fn stray(ui: &mut Ui) -> usize {
    let (rx, ry, rw, rh) = ui.rect_px();
    let m = 15.0 * S; // (the notch's curved ears reach a little to the sides)
    let pm = ui.render();
    let (w, h) = (pm.width() as usize, pm.height() as usize);
    let d = pm.data();
    let mut n = 0;
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f32, y as f32);
            if fx >= rx - m && fx <= rx + rw + m && fy >= ry - m && fy <= ry + rh + m {
                continue;
            }
            if d[(y * w + x) * 4 + 3] > 12 {
                n += 1;
            }
        }
    }
    n
}

fn main() {
    let lrc = "[00:04.00] We were never meant to leave\n[00:09.00] There's a stranger in my bed\n[00:14.00] You haven't felt right for days and days and days and days\n[00:19.00] Hold on";
    let mut bad = 0;
    for seed in 0..40u64 {
        let mut r = Rng(seed * 7919 + 13);
        let mut ui = Ui::new(S);
        let mut pos = 10.0;
        let mut log: Vec<String> = Vec::new();
        for step in 0..45 {
            let ev = r.pick(14);
            match ev {
                0 => { ui.set_art(Some(art())); ui.set_media(MediaInfo { active: true, title: format!("Song {}", r.pick(3)), artist: "Artist".into(), playing: r.pick(3) != 0, pos, dur: if r.pick(4) == 0 { 0.0 } else { 240.0 } }); log.push("media".into()); }
                1 => { ui.set_media(MediaInfo::default()); log.push("no media".into()); }
                2 => { ui.set_cursor(Some((W / 2.0 * S + r.pick(40) as f32, (TOP + r.pick(30) as f32) * S))); log.push("hover".into()); }
                3 => { ui.set_cursor(None); log.push("leave".into()); }
                4 => { ui.toggle_lyrics(); log.push("toggle lyrics".into()); }
                5 => { if let Some(q) = ui.take_lyrics_request() { ui.set_lyrics(&q.key, Some(Found { synced: r.pick(4) != 0, lines: parse_lrc(lrc) })); log.push("lyrics arrive".into()); } }
                6 => { ui.set_env(Env { maximized: r.pick(2) == 0, fullscreen: r.pick(5) == 0, peek: r.pick(4) == 0, player_front: r.pick(4) == 0 }); log.push("env".into()); }
                7 => { ui.push_notification(NotifData { app: "App".into(), aumid: String::new(), title: "Title".into(), body: "A fairly long body text that has to be cut somewhere".into(), icon: if r.pick(2) == 0 { Some(art()) } else { None } }); log.push("notification".into()); }
                8 => { pos += 7.0; ui.set_media(MediaInfo { active: true, title: "Song 0".into(), artist: "Artist".into(), playing: true, pos, dur: 240.0 }); log.push("time".into()); }
                9 => { ui.mouse_down(W / 2.0 * S, (TOP + 12.0) * S); log.push("down".into()); }
                10 => { ui.mouse_up(W / 2.0 * S, (TOP + 12.0) * S); log.push("up".into()); }
                11 => { ui.wheel(1.0); }
                _ => {}
            }
            for _ in 0..(1 + r.pick(40)) {
                ui.tick(1.0 / 60.0);
                let n = if ui.is_transformed() { 0 } else { stray(&mut ui) };
                let (x, y, w, h) = ui.rect_px();
                if n > 0 || !(x.is_finite() && y.is_finite() && w.is_finite() && h.is_finite()) || w < 0.0 || h < 0.0 {
                    bad += 1;
                    if bad <= 4 {
                        println!("seed {seed} step {step}: {n} stray pixels, rect {:?}; events: {}", (x, y, w, h), log.join(" > "));
                        ui.render().save_png(format!("preview/fuzz_{bad}.png")).ok();
                    }
                    break;
                }
            }
        }
    }
    println!("{} bad frames in 40 random runs", bad);
}
