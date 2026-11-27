#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
fn main() {
    island::win::run();
}

#[cfg(not(windows))]
fn main() {
    eprintln!("This program is for Windows. Run the `preview` binary to render its look to PNG files.");
}
