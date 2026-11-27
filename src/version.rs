//! The parts of the updater that need no Windows: version numbers and reading a GitHub release.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// "v1.2.3" / "1.2.3-beta" -> (1, 2, 3)
pub fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let v = v.trim().trim_start_matches(['v', 'V']);
    let core = v.split(['-', '+']).next()?;
    let mut it = core.split('.');
    let a = it.next()?.parse().ok()?;
    let b = it.next().unwrap_or("0").parse().ok()?;
    let c = it.next().unwrap_or("0").parse().ok()?;
    if it.next().is_some() {
        return None;
    }
    Some((a, b, c))
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Which file of a release is the program, and (if the release says) its SHA-256.
pub fn pick_asset(json: &serde_json::Value) -> Option<(String, Option<String>)> {
    let assets = json.get("assets")?.as_array()?;
    let mut best: Option<&serde_json::Value> = None;
    for a in assets {
        let name = a.get("name").and_then(|n| n.as_str()).unwrap_or("").to_lowercase();
        if name.ends_with(".exe") {
            // prefer the one that looks like the app itself
            if best.is_none() || name.contains("island") || name.contains("bloop") {
                best = Some(a);
            }
        }
    }
    let a = best?;
    let url = a.get("browser_download_url")?.as_str()?.to_string();
    let digest = a.get("digest").and_then(|d| d.as_str()).and_then(|d| d.strip_prefix("sha256:")).map(|d| d.to_lowercase());
    Some((url, digest))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert_eq!(parse_version("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("0.9"), Some((0, 9, 0)));
        assert_eq!(parse_version("2.0.0-beta.1"), Some((2, 0, 0)));
        assert_eq!(parse_version("nightly"), None);
        assert!(is_newer("0.10.0", "0.9.5"));
        assert!(is_newer("v1.0.0", "0.9.9"));
        assert!(!is_newer("0.9.0", "0.9.0"));
        assert!(!is_newer("0.8.9", "0.9.0"));
        assert!(!is_newer("garbage", "0.9.0"));
    }

    #[test]
    fn asset_choice() {
        let j: serde_json::Value = serde_json::from_str(
            r#"{"assets":[{"name":"notes.txt","browser_download_url":"https://github.com/a/b/x.txt"},
                {"name":"Dynamic-Island.exe","browser_download_url":"https://github.com/a/b/d.exe","digest":"sha256:ABCdef"}]}"#,
        )
        .unwrap();
        let (url, d) = pick_asset(&j).unwrap();
        assert!(url.ends_with("d.exe"));
        assert_eq!(d.as_deref(), Some("abcdef"));
    }
}
