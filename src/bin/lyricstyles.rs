//! The four ways the lyrics can move, in the middle of a line.
use island::lyrics::{parse_lrc, Found};
use island::settings::Settings;
use island::ui::{MediaInfo, Ui};
use tiny_skia::{Color, Pixmap, PixmapPaint, Transform};

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let mut sheet = Pixmap::new(940, 560).unwrap();
    sheet.fill(Color::from_rgba8(40, 70, 60, 255));
    for style in 0..2u8 {
        let mut ui = Ui::new(1.0);
        let mut s = Settings::default();
        s.lyrics_anim = style;
        ui.set_settings(&s);
        ui.set_media(MediaInfo { active: true, title: "Halo".into(), artist: "Beyonce".into(), playing: true, pos: 20.0, dur: 261.0 });
        ui.set_cursor(Some((235.0, 40.0)));
        for _ in 0..40 {
            ui.tick(1.0 / 60.0);
        }
        ui.toggle_lyrics();
        let req = ui.take_lyrics_request().unwrap();
        let lrc = "[00:08.00]Remember those walls I built?\n[00:14.00]Well, baby, they're tumbling down\n[00:20.00]And they didn't even put up a fight\n[00:26.00]They didn't even make a sound\n[00:32.00]I found a way to let you in";
        ui.set_lyrics(&req.key, Some(Found { synced: true, lines: parse_lrc(lrc) }));
        for _ in 0..(60 * 3) {
            ui.tick(1.0 / 60.0);
        }
        let img = ui.render().clone();
        sheet.draw_pixmap((style as i32 % 2) * 470, (style as i32 / 2) * 280, img.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
    }
    sheet.save_png(format!("{out}/lyricstyles.png")).unwrap();
}
