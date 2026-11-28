# Contributing to Bloop

Thank you for helping. Bug reports, ideas and pull requests are all welcome.

## Reporting a problem

Open an [issue](../../issues/new/choose) and use the *Bug report* form. The most useful things to include:

* your Windows version (Settings → System → About) and Bloop's version (Settings → Updates);
* what you did, what you expected and what happened (a short screen recording helps a lot for animations);
* the log, if something does not work: `%TEMP%\bloop.log`.

## Working on the code

You do not need Windows for most things. The island and the settings window are drawn by portable code (see
[ARCHITECTURE.md](ARCHITECTURE.md)), so you can change them and look at the result as a PNG:

```
cargo test --release                       # tests
cargo run --release --bin panelview        # the settings window as PNG files
cargo run --release --bin preview          # the island in many states
```

Only the parts that talk to Windows (`win.rs`, `media.rs`, `notify.rs`, `sys.rs`, `volume.rs`, `popups.rs`, `sound.rs`, `net.rs`,
`settings_win.rs`, `update.rs`) need a Windows machine to try out. To build the real program:

```
cargo build --release --bin island        # on Windows
```

## Pull requests

* Keep a change small and about one thing.
* Run `cargo test --release` first. If you change how something looks, add a picture from the matching tool in `src/bin/`.
* Use the same style as the code around you: short comments that say *why*, plain English.
* Add a line to [CHANGELOG.md](CHANGELOG.md) for anything a user would notice.

## Ideas that fit

Small, calm, native. Bloop should stay light on CPU and memory when nothing is happening, and should never need a background
service, an account or an installer.
