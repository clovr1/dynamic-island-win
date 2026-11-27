//! The user's settings: a small text file (`key = value` lines) that is easy to read and to repair by hand.

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MaxMode {
    /// under a maximized window the island becomes a slim notch (music) / hides (nothing playing)
    Notch,
    /// under a maximized window the island always hides (the top edge brings it back)
    Hide,
    /// ignore maximized windows: the island stays as it is
    Normal,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Settings {
    pub show_music_pill: bool,
    pub hide_when_player_front: bool,
    pub notifications: bool,
    pub use_bubble: bool,
    /// seconds a notification card stays (2..=20)
    pub card_secs: u32,
    pub lyrics_button: bool,
    pub maximized: MaxMode,
    pub fullscreen_hide: bool,
    /// size of the whole island, percent (80..=140)
    pub scale_pct: u32,
    /// animation speed, percent (50..=200)
    pub speed_pct: u32,
    /// equalizer in the cover's colour (false = white)
    pub accent_eq: bool,
    pub check_updates: bool,
    /// "owner/repo" on GitHub that new versions are published to
    pub update_repo: String,
    /// colour theme of the settings window (index into panel::THEMES)
    pub theme: u8,
    /// quick-setting and battery pills (Wi-Fi, Bluetooth, charging ...)
    pub status_pills: bool,
    /// notifications stay quiet (only listed) while Windows' Do Not Disturb is on
    pub dnd_quiet: bool,
    /// soft sound effects (opening, closing, notifications)
    pub sounds: bool,
    /// 0..=100
    pub volume: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            show_music_pill: true,
            hide_when_player_front: true,
            notifications: true,
            use_bubble: true,
            card_secs: 5,
            lyrics_button: true,
            maximized: MaxMode::Notch,
            fullscreen_hide: true,
            scale_pct: 100,
            speed_pct: 100,
            accent_eq: true,
            check_updates: true,
            update_repo: "clovr1/dynamic-island-win".to_string(),
            theme: 0,
            status_pills: true,
            dnd_quiet: true,
            sounds: true,
            volume: 40,
        }
    }
}

fn parse_bool(v: &str) -> Option<bool> {
    match v.trim().to_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

impl Settings {
    /// Lenient: unknown lines are ignored, bad values fall back to the default, numbers are kept in their range.
    pub fn parse(text: &str) -> Settings {
        let mut s = Settings::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, v) = (k.trim().to_lowercase(), v.trim());
            let b = parse_bool(v);
            let n = v.parse::<u32>().ok();
            match k.as_str() {
                "show_music_pill" => s.show_music_pill = b.unwrap_or(s.show_music_pill),
                "hide_when_player_front" => s.hide_when_player_front = b.unwrap_or(s.hide_when_player_front),
                "notifications" => s.notifications = b.unwrap_or(s.notifications),
                "use_bubble" => s.use_bubble = b.unwrap_or(s.use_bubble),
                "card_secs" => s.card_secs = n.unwrap_or(s.card_secs),
                "lyrics_button" => s.lyrics_button = b.unwrap_or(s.lyrics_button),
                "maximized" => {
                    s.maximized = match v.to_lowercase().as_str() {
                        "notch" => MaxMode::Notch,
                        "hide" => MaxMode::Hide,
                        "normal" => MaxMode::Normal,
                        _ => s.maximized,
                    }
                }
                "fullscreen_hide" => s.fullscreen_hide = b.unwrap_or(s.fullscreen_hide),
                "scale_pct" => s.scale_pct = n.unwrap_or(s.scale_pct),
                "speed_pct" => s.speed_pct = n.unwrap_or(s.speed_pct),
                "accent_eq" => s.accent_eq = b.unwrap_or(s.accent_eq),
                "check_updates" => s.check_updates = b.unwrap_or(s.check_updates),
                "update_repo" => s.update_repo = v.to_string(),
                "status_pills" => s.status_pills = parse_bool(v).unwrap_or(s.status_pills),
                "sounds" => s.sounds = parse_bool(v).unwrap_or(s.sounds),
                "volume" => s.volume = v.trim().parse().unwrap_or(40),
                "dnd_quiet" => s.dnd_quiet = parse_bool(v).unwrap_or(s.dnd_quiet),
                "theme" => s.theme = v.trim().parse().unwrap_or(0),
                _ => {}
            }
        }
        s.clamp();
        s
    }

    pub fn clamp(&mut self) {
        self.card_secs = self.card_secs.clamp(2, 20);
        self.scale_pct = self.scale_pct.clamp(80, 140);
        self.speed_pct = self.speed_pct.clamp(50, 200);
        self.theme = self.theme.min(5);
        self.volume = self.volume.min(100);
        // an "owner/repo" is letters, digits, - _ . and one slash; anything else is dropped
        let ok = self.update_repo.split('/').count() == 2 && self.update_repo.chars().all(|c| c.is_ascii_alphanumeric() || "-_./".contains(c));
        if !ok {
            // an empty or odd value (an old settings file) falls back to the project's own repository
            self.update_repo = Settings::default().update_repo;
        }
    }

    pub fn to_text(&self) -> String {
        let b = |v: bool| if v { "yes" } else { "no" };
        format!(
            "# Dynamic Island settings (you can edit this file; the island reads it when it starts)\n\
             show_music_pill = {}\nhide_when_player_front = {}\nnotifications = {}\nuse_bubble = {}\ncard_secs = {}\nlyrics_button = {}\n\
             maximized = {}\nfullscreen_hide = {}\nscale_pct = {}\nspeed_pct = {}\naccent_eq = {}\ncheck_updates = {}\nupdate_repo = {}\ntheme = {}\nstatus_pills = {}\ndnd_quiet = {}\nsounds = {}\nvolume = {}\n",
            b(self.show_music_pill),
            b(self.hide_when_player_front),
            b(self.notifications),
            b(self.use_bubble),
            self.card_secs,
            b(self.lyrics_button),
            match self.maximized {
                MaxMode::Notch => "notch",
                MaxMode::Hide => "hide",
                MaxMode::Normal => "normal",
            },
            b(self.fullscreen_hide),
            self.scale_pct,
            self.speed_pct,
            b(self.accent_eq),
            b(self.check_updates),
            self.update_repo,
            self.theme,
            b(self.status_pills),
            b(self.dnd_quiet),
            b(self.sounds),
            self.volume
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let mut s = Settings::default();
        s.card_secs = 9;
        s.maximized = MaxMode::Hide;
        s.scale_pct = 115;
        s.update_repo = "someone/dynamic-island".into();
        s.accent_eq = false;
        assert_eq!(Settings::parse(&s.to_text()), s);
    }

    #[test]
    fn bad_input_is_harmless() {
        let s = Settings::parse("card_secs = 9999\nscale_pct = 5\nmaximized = sideways\nupdate_repo = ../../etc\nnonsense\nnotifications = maybe\n");
        assert_eq!(s.card_secs, 20);
        assert_eq!(s.scale_pct, 80);
        assert_eq!(s.maximized, MaxMode::Notch);
        assert_eq!(s.update_repo, Settings::default().update_repo);
        assert!(s.notifications);
    }
}
