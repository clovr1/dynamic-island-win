pub mod anim;
pub mod brands;
pub mod gfx;
pub mod lyrics;
pub mod panel;
pub mod settings;
pub mod svgpath;
pub mod ui;
pub mod version;

#[cfg(windows)]
pub mod media;
#[cfg(windows)]
pub mod net;
#[cfg(windows)]
pub mod settings_win;
#[cfg(windows)]
pub mod update;
#[cfg(windows)]
pub mod notify;
#[cfg(windows)]
pub mod win;
