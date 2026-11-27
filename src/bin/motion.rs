//! Looks for drawing outside the island's shape while it opens / closes (a leftover would show on screen as stray pieces),
//! and prints how the size moves. `cargo run --release --bin motion`
use island::lyrics::{parse_lrc, Found};
use island::ui::{Env, MediaInfo, Ui, TOP, W};

const S: f32 = 2.0;

fn outside(ui: &mut Ui) -> usize {
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

fn scenario(name: &str, env: Env, lyrics: bool) {
    let mut ui = Ui::new(S);
    let lrc = "[00:04.00] We were never meant to leave\n[00:09.00] There's a stranger in my bed\n[00:14.00] Every wing a little song that carries us along\n[00:19.00] Hold on";
    ui.set_media(MediaInfo { active: true, title: "Let Down".into(), artist: "Radiohead".into(), playing: false, pos: 16.0, dur: 299.0 });
    ui.set_env(env);
    for _ in 0..90 { ui.tick(1.0 / 60.0); }
    ui.set_cursor(Some((W / 2.0 * S, (TOP + 12.0) * S)));
    for _ in 0..90 { ui.tick(1.0 / 60.0); }
    if lyrics {
        ui.toggle_lyrics();
        let req = ui.take_lyrics_request().unwrap();
        ui.set_lyrics(&req.key, Some(Found { synced: true, lines: parse_lrc(lrc) }));
        for _ in 0..90 { ui.tick(1.0 / 60.0); }
    }
    let mut worst = 0usize;
    // the open player closes (the cursor leaves)
    ui.set_cursor(None);
    for _ in 0..120 {
        ui.tick(1.0 / 60.0);
        let o = outside(&mut ui);
        if o > worst {
            worst = o;
            ui.render().save_png(format!("preview/stray_{}.png", name.split(',').next().unwrap().replace(' ', "_"))).ok();
        }
    }
    // ... and opens again
    ui.set_cursor(Some((W / 2.0 * S, (TOP + 12.0) * S)));
    for _ in 0..120 {
        ui.tick(1.0 / 60.0);
        worst = worst.max(outside(&mut ui));
    }
    println!("{:<46} stray pixels outside the island, worst frame: {}  {}", name, worst, if worst == 0 { "OK" } else { "LEFTOVER" });
}

fn main() {
    scenario("player, normal window", Env::default(), false);
    scenario("player, hanging from the top (maximized)", Env { maximized: true, ..Env::default() }, false);
    scenario("lyrics view, normal window", Env::default(), true);
    scenario("lyrics view, hanging from the top", Env { maximized: true, ..Env::default() }, true);
}
