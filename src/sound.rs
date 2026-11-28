//! The island's sound effects: small WAV files inside the program, played through Windows (no extra libraries).

use crate::ui::Sound;
use std::sync::Mutex;
use windows::core::PCWSTR;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};

static NOTIFY: &[u8] = include_bytes!("../assets/sounds/notify.wav");
static BUBBLE: &[u8] = include_bytes!("../assets/sounds/bubble.wav");
static PAUSE: &[u8] = include_bytes!("../assets/sounds/pause.wav");
static PLAY: &[u8] = include_bytes!("../assets/sounds/play.wav");
static SKIP: &[u8] = include_bytes!("../assets/sounds/skip.wav");

macro_rules! status {
    ($($f:literal),*) => { [$(include_bytes!(concat!("../assets/sounds/", $f, ".wav")) as &[u8]),*] };
}

/// The status pills' sounds, in the order of `ui::status_sound_code`: for each of wifi, bluetooth, hotspot, cellular, airplane, dnd,
/// nightlight, energy: off then on; then the three battery sounds (charging, unplugged, low).
static STATUS: [&[u8]; 19] = status!(
    "status_wifi_off", "status_wifi_on", "status_bluetooth_off", "status_bluetooth_on", "status_hotspot_off", "status_hotspot_on",
    "status_cellular_off", "status_cellular_on", "status_airplane_off", "status_airplane_on", "status_dnd_off", "status_dnd_on",
    "status_nightlight_off", "status_nightlight_on", "status_energy_off", "status_energy_on", "battery_charging", "battery_unplugged", "battery_low"
);

/// The buffers Windows may still be playing from (it reads them while it plays): the last few are kept.
static KEEP: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());

/// Play `s` at `volume` (0..=100). A new sound replaces the one still playing.
pub fn play(s: Sound, volume: u32) {
    if volume == 0 {
        return;
    }
    let src = match s {
        Sound::Notify => NOTIFY,
        Sound::Bubble => BUBBLE,
        Sound::Status(c) => STATUS[(c as usize).min(STATUS.len() - 1)],
        Sound::Pause => PAUSE,
        Sound::Play => PLAY,
        Sound::Skip => SKIP,
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
