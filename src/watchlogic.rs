//! Small pieces of the microphone / camera dot and the download pill that need no Windows (so they can be tested anywhere).

/// A readable name for the program Windows lists as using the microphone or camera. Windows names the keys either as a file path with
/// `#` instead of `\` ("C:#Program Files#Google#Chrome#Application#chrome.exe") or as a store app ("Microsoft.WindowsCamera_8wekyb3d8bbwe").
pub fn consent_app_name(key: &str) -> String {
    if key.contains('#') {
        let leaf = key.rsplit('#').next().unwrap_or(key);
        let stem = match leaf.rfind('.') {
            Some(i) if leaf[i..].eq_ignore_ascii_case(".exe") => &leaf[..i],
            _ => leaf,
        };
        return capitalize(stem);
    }
    let name = key.split('_').next().unwrap_or(key);
    let name = name.rsplit('.').next().unwrap_or(name);
    // "WindowsCamera" -> "Windows Camera"
    let mut out = String::new();
    let mut prev_lower = false;
    for c in name.chars() {
        if c.is_uppercase() && prev_lower {
            out.push(' ');
        }
        prev_lower = c.is_lowercase();
        out.push(c);
    }
    capitalize(&out)
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// The file a browser is still downloading ("film.mp4.crdownload") and what it will be called when it is done ("film.mp4").
pub fn partial_final_name(file: &str) -> Option<String> {
    let lower = file.to_lowercase();
    for suffix in [".crdownload", ".opdownload", ".part", ".download", ".partial"] {
        if lower.ends_with(suffix) {
            let stem = &file[..file.len() - suffix.len()];
            if stem.is_empty() {
                return None;
            }
            // Chrome does not know the real name yet at the very start
            if stem.starts_with("Unconfirmed ") {
                return Some("Download".to_string());
            }
            return Some(stem.to_string());
        }
    }
    None
}

/// "812 KB", "12.4 MB", "1.2 GB".
pub fn fmt_bytes(n: u64) -> String {
    const KB: f64 = 1024.0;
    let f = n as f64;
    if f < KB {
        format!("{n} B")
    } else if f < KB * KB {
        format!("{:.0} KB", f / KB)
    } else if f < KB * KB * KB {
        format!("{:.1} MB", f / (KB * KB))
    } else {
        format!("{:.1} GB", f / (KB * KB * KB))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_names() {
        assert_eq!(consent_app_name("C:#Program Files#Google#Chrome#Application#chrome.exe"), "Chrome");
        assert_eq!(consent_app_name("C:#Users#me#AppData#Local#Discord#app-1.0.9#Discord.exe"), "Discord");
        assert_eq!(consent_app_name("Microsoft.WindowsCamera_8wekyb3d8bbwe"), "Windows Camera");
        assert_eq!(consent_app_name("MSTeams_8wekyb3d8bbwe"), "MSTeams");
    }

    #[test]
    fn partial_files() {
        assert_eq!(partial_final_name("film.mp4.crdownload").as_deref(), Some("film.mp4"));
        assert_eq!(partial_final_name("setup.EXE.PART").as_deref(), Some("setup.EXE"));
        assert_eq!(partial_final_name("Unconfirmed 123456.crdownload").as_deref(), Some("Download"));
        assert_eq!(partial_final_name("notes.txt"), None);
        assert_eq!(partial_final_name(".part"), None);
    }

    #[test]
    fn sizes() {
        assert_eq!(fmt_bytes(512), "512 B");
        assert_eq!(fmt_bytes(812 * 1024), "812 KB");
        assert_eq!(fmt_bytes(13 * 1024 * 1024 + 400 * 1024), "13.4 MB");
        assert_eq!(fmt_bytes(3 * 1024 * 1024 * 1024), "3.0 GB");
    }
}
