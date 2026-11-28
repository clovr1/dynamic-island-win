//! The lyrics view at the very start of a song (before the first line is sung).
use island::lyrics::{parse_lrc, Found};
use island::settings::Settings;
use island::ui::{MediaInfo, Ui};

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let mut ui = Ui::new(1.5);
    ui.set_settings(&Settings::default());
    ui.set_media(MediaInfo { active: true, title: "Halo".into(), artist: "Beyonce".into(), playing: true, pos: 2.0, dur: 261.0 });
    ui.toggle_lyrics();
    let req = ui.take_lyrics_request().expect("lookup");
    let lrc = "[00:20.00]Remember those walls I built?\n[00:25.00]Well, baby, they're tumbling down\n[00:30.00]And they didn't even put up a fight\n[00:35.00]They didn't even make a sound\n[00:40.00]I found a way to let you in";
    ui.set_lyrics(&req.key, Some(Found { synced: true, lines: parse_lrc(lrc) }));
    ui.set_cursor(Some((350.0, 60.0)));
    for _ in 0..150 {
        ui.tick(1.0 / 60.0);
    }
    ui.render().save_png(format!("{out}/lyricstart.png")).unwrap();
}
