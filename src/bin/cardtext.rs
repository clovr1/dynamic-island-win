//! The notification card, for looking at the lettering.
use island::settings::Settings;
use island::ui::{NotifData, Ui};

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let mut ui = Ui::new(1.25);
    ui.set_settings(&Settings::default());
    ui.push_notification(NotifData { app: "WhatsApp".into(), aumid: String::new(), title: "manrp".into(), body: "salim : laporan sek gak seh".into(), icon: None });
    for _ in 0..90 {
        ui.tick(1.0 / 60.0);
    }
    let pm = ui.render().clone();
    pm.save_png(format!("{out}/cardtext.png")).unwrap();
}
