//! Lyrics: parsing LRC (time-stamped) text and choosing the best match from LRCLIB's answer. No network and no Windows in here.

use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    /// seconds from the start of the song (0 for plain, unsynced lyrics)
    pub t: f32,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct Found {
    pub synced: bool,
    pub lines: Vec<Line>,
}

/// "[01:23.45] some text" -> (83.45, "some text"); a line may carry several time stamps.
pub fn parse_lrc(text: &str) -> Vec<Line> {
    let mut out = Vec::new();
    for raw in text.lines() {
        let mut rest = raw.trim_start();
        let mut stamps = Vec::new();
        while let Some(stripped) = rest.strip_prefix('[') {
            let Some(end) = stripped.find(']') else { break };
            let tag = &stripped[..end];
            match parse_stamp(tag) {
                Some(t) => stamps.push(t),
                None => break, // [ar:..] [ti:..] and other tags carry no time stamp
            }
            rest = stripped[end + 1..].trim_start();
        }
        if stamps.is_empty() {
            continue;
        }
        let txt = rest.trim().to_string();
        for t in stamps {
            out.push(Line { t, text: txt.clone() });
        }
    }
    out.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
    out.truncate(600);
    out
}

fn parse_stamp(tag: &str) -> Option<f32> {
    let (m, rest) = tag.split_once(':')?;
    let m: f32 = m.trim().parse().ok()?;
    if !tag.chars().next()?.is_ascii_digit() {
        return None;
    }
    let (s, frac) = match rest.split_once(|c| c == '.' || c == ':') {
        Some((s, f)) => (s, Some(f)),
        None => (rest, None),
    };
    let s: f32 = s.trim().parse().ok()?;
    let f: f32 = frac.map(|f| format!("0.{}", f.trim()).parse().unwrap_or(0.0)).unwrap_or(0.0);
    Some(m * 60.0 + s + f)
}

pub fn parse_plain(text: &str) -> Vec<Line> {
    text.lines().map(|l| Line { t: 0.0, text: l.trim().to_string() }).take(600).collect()
}

/// "Song (feat. X) - Remastered 2011" -> "Song"
pub fn clean_title(s: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for c in s.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = (depth - 1).max(0),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    if let Some(i) = out.find(" - ") {
        out.truncate(i);
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// "A, B & C" -> "A"
pub fn first_artist(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut cut = s.len();
    for sep in [",", "&", ";", " feat. ", " feat ", " ft. ", " ft ", " x "] {
        if let Some(i) = lower.find(sep) {
            cut = cut.min(i);
        }
    }
    s[..cut].trim().to_string()
}

/// Pick the best LRCLIB result: synced lyrics first (else plain), the one whose length is closest to the song's.
pub fn pick_best(list: &Value, duration: f64) -> Option<Found> {
    let arr = list.as_array()?;
    let field = |x: &Value, k: &str| x.get(k).and_then(|v| v.as_str()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let rank = |key: &str| -> Option<String> {
        let mut c: Vec<&Value> = arr.iter().filter(|x| field(x, key).is_some()).collect();
        if c.is_empty() {
            return None;
        }
        if duration > 0.0 {
            c.sort_by(|a, b| {
                let da = (a.get("duration").and_then(|v| v.as_f64()).unwrap_or(0.0) - duration).abs();
                let db = (b.get("duration").and_then(|v| v.as_f64()).unwrap_or(0.0) - duration).abs();
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        field(c[0], key)
    };
    if let Some(s) = rank("syncedLyrics") {
        let lines = parse_lrc(&s);
        if !lines.is_empty() {
            return Some(Found { synced: true, lines });
        }
    }
    if let Some(p) = rank("plainLyrics") {
        let lines = parse_plain(&p);
        if !lines.is_empty() {
            return Some(Found { synced: false, lines });
        }
    }
    None
}

pub fn url_encode(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            o.push(b as char);
        } else {
            o.push_str(&format!("%{:02X}", b));
        }
    }
    o
}

/// The searches to try, in order (same as the Electron version).
pub fn search_paths(title: &str, artist: &str) -> Vec<String> {
    let ct = clean_title(title);
    let ct = if ct.is_empty() { title.to_string() } else { ct };
    let fa = first_artist(artist);
    vec![
        format!("/api/search?track_name={}&artist_name={}", url_encode(title), url_encode(artist)),
        format!("/api/search?track_name={}&artist_name={}", url_encode(&ct), url_encode(&fa)),
        format!("/api/search?q={}", url_encode(format!("{} {}", ct, fa).trim())),
    ]
}

/// Index of the line being sung at `pos` seconds (-1 before the first line).
pub fn index_at(lines: &[Line], pos: f32) -> i32 {
    let mut i = -1;
    for (k, l) in lines.iter().enumerate() {
        if l.t <= pos { i = k as i32 } else { break }
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn lrc_basic_and_multi_stamps() {
        let l = parse_lrc("[ar:Someone]\n[00:12.50] first\n[01:05.00][02:10.25]  again \n[bad] x\n");
        assert_eq!(l.len(), 3);
        assert!((l[0].t - 12.5).abs() < 1e-3 && l[0].text == "first");
        assert!((l[1].t - 65.0).abs() < 1e-3 && l[1].text == "again");
        assert!((l[2].t - 130.25).abs() < 1e-3);
    }
    #[test]
    fn titles_and_artists() {
        assert_eq!(clean_title("Song (feat. X) - Remastered 2011"), "Song");
        assert_eq!(clean_title("Love Story [Live]"), "Love Story");
        assert_eq!(first_artist("Marshmello, Khalid"), "Marshmello");
        assert_eq!(first_artist("A & B"), "A");
        assert_eq!(first_artist("Solo Artist"), "Solo Artist");
        assert_eq!(first_artist("Drake feat. Rihanna"), "Drake");
    }
    #[test]
    fn picks_synced_closest_duration() {
        let v = json!([
            {"duration": 200.0, "syncedLyrics": "", "plainLyrics": "plain far"},
            {"duration": 181.0, "syncedLyrics": "[00:01.00] near", "plainLyrics": "x"},
            {"duration": 240.0, "syncedLyrics": "[00:01.00] far", "plainLyrics": "x"},
        ]);
        let f = pick_best(&v, 180.0).unwrap();
        assert!(f.synced && f.lines[0].text == "near");
        let p = pick_best(&json!([{"duration": 10, "plainLyrics": "a\nb"}]), 0.0).unwrap();
        assert!(!p.synced && p.lines.len() == 2);
        assert!(pick_best(&json!([]), 100.0).is_none());
    }
    #[test]
    fn current_line() {
        let l = parse_lrc("[00:05.00] a\n[00:10.00] b");
        assert_eq!(index_at(&l, 1.0), -1);
        assert_eq!(index_at(&l, 5.0), 0);
        assert_eq!(index_at(&l, 12.0), 1);
    }
    #[test]
    fn encoding() {
        assert_eq!(url_encode("a b&é"), "a%20b%26%C3%A9");
    }
}
