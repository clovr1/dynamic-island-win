//! The island's sound effects: small WAV files inside the program, played through Windows (no extra libraries).

use crate::ui::Sound;
use std::sync::Mutex;
use windows::core::PCWSTR;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};

static OPEN: &[u8] = include_bytes!("../assets/sounds/open.wav");
static CLOSE: &[u8] = include_bytes!("../assets/sounds/close.wav");
static NOTIFY: &[u8] = include_bytes!("../assets/sounds/notify.wav");
static STATUS: &[u8] = include_bytes!("../assets/sounds/status.wav");
static PAUSE: &[u8] = include_bytes!("../assets/sounds/pause.wav");
static PLAY: &[u8] = include_bytes!("../assets/sounds/play.wav");
static SKIP: &[u8] = include_bytes!("../assets/sounds/skip.wav");

/// The buffers Windows may still be playing from (it reads them while it plays): the last few are kept.
static KEEP: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());

/// Play `s` at `volume` (0..=100). A new sound replaces the one still playing.
pub fn play(s: Sound, volume: u32) {
    if volume == 0 {
        return;
    }
    let src = match s {
        Sound::Open => OPEN,
        Sound::Close => CLOSE,
        Sound::Notify => NOTIFY,
        Sound::Status => STATUS,
        Sound::Pause => PAUSE,
        Sound::Play => PLAY,
        Sound::Skip => SKIP,
    };
    let mut buf = src.to_vec();
    // the files are already quiet; 100 % is about twice the file (the samples start after the 44-byte header)
    let gain = volume as f32 / 100.0 * 2.0;
    for c in buf[44..].chunks_exact_mut(2) {
        let v = i16::from_le_bytes([c[0], c[1]]) as f32 * gain;
        c.copy_from_slice(&(v.clamp(-32767.0, 32767.0) as i16).to_le_bytes());
    }
    let ptr = buf.as_ptr();
    if let Ok(mut k) = KEEP.lock() {
        k.push(buf);
        if k.len() > 8 {
            k.remove(0);
        }
    }
    unsafe {
        let _ = PlaySoundW(PCWSTR(ptr as *const u16), HMODULE::default(), SND_MEMORY | SND_ASYNC | SND_NODEFAULT);
    }
}
