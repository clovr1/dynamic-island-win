# Bloop

A dynamic island for the Windows desktop. Bloop turns the top of your screen into a small, lively pill:
music with synced lyrics, notification cards and bubbles, Wi-Fi / Bluetooth / battery status pills,
a slim notch under maximized windows, and a settings window with a live preview.

Written in Rust. No web engine: everything is drawn by the app itself.

**Download:** <https://github.com/clovr1/dynamic-island-win/releases/latest/download/Bloop.exe>
· **Website:** <https://clovr1.github.io/dynamic-island-win/>

## Install
1. Download `Bloop.exe` and open it. If Windows SmartScreen warns, click *More info*, then *Run anyway*.
2. Click the tray icon to open Settings. Turn on *Start with Windows* if you like.

## Releases and updates
Push a tag like `v0.23.2`. The workflow in `.github/workflows/release.yml` builds `Bloop.exe` on Windows and
attaches it to the release. The app looks for new releases by itself (Settings → Updates).

## The website
`docs/` is a static page for GitHub Pages (Settings → Pages → Deploy from a branch → `main` → `/docs`).
The pictures in `docs/img` are drawn by the app itself: `cargo run --release --bin sitepics -- docs/img`.

## Building
```
cargo build --release --bin island        # on Windows
cargo test --release
```

## For whoever builds it
Settings → Updates → *Install from a file* (to try a build before publishing it) only appears when the file
`%APPDATA%\DynamicIsland\developer.txt` exists.
