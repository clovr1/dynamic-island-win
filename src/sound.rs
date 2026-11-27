//! The island's sound effects: small WAV files inside the program, played through Windows (no extra libraries).

use crate::ui::Sound;
use std::sync::Mutex;
use windows::core::PCWSTR;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};

macro_rules! pack {
    ($d:literal) => {
        [
            include_bytes!(concat!("../assets/sounds/", $d, "/notify.wav")) as &[u8],
            include_bytes!(concat!("../assets/sounds/", $d, "/status.wav")),
            include_bytes!(concat!("../assets/sounds/", $d, "/pause.wav")),
            include_bytes!(concat!("../assets/sounds/", $d, "/play.wav")),
            include_bytes!(concat!("../assets/sounds/", $d, "/skip.wav")),
        ]
    };
}

/// notify, status, pause, play, skip - for each set of sounds (Bloop, Glass, Wood, Air)
static PACKS: [[&[u8]; 5]; 4] = [pack!("bloop"), pack!("glass"), pack!("wood"), pack!("air")];

/// The buffers Windows may still be playing from (it reads them while it plays): the last few are kept.
static KEEP: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());

/// Play `s` at `volume` (0..=100). A new sound replaces the one still playing.
pub fn play(s: Sound, volume: u32, pack: u8) {
    if volume == 0 {
        return;
    }
    let p = &PACKS[(pack as usize).min(PACKS.len() - 1)];
    let src = match s {
        Sound::Notify => p[0],
        Sound::Status => p[1],
        Sound::Pause => p[2],
        Sound::Play => p[3],
        Sound::Skip => p[4],
        // (hovering is silent)
        Sound::Open | Sound::Close => return,
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
