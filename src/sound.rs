//! The island's sound effects: small WAV files inside the program, played through Windows (no extra libraries).
//!
//! To start without delay the output device is opened once and kept awake with an inaudible, looping buffer (a sound that is started
//! on a sleeping audio device is heard a moment late). If the device cannot be opened, the older `PlaySound` is used.

use crate::ui::Sound;
use std::sync::Mutex;
use windows::core::PCWSTR;
use windows::Win32::Foundation::HMODULE;
use windows::core::PSTR;
use windows::Win32::Media::Audio::{
    waveOutClose, waveOutOpen, waveOutPrepareHeader, waveOutReset, waveOutUnprepareHeader, waveOutWrite, PlaySoundW, CALLBACK_NULL, HWAVEOUT, SND_ASYNC, SND_MEMORY, SND_NODEFAULT, WAVEFORMATEX, WAVEHDR,
    WAVE_FORMAT_PCM, WAVE_MAPPER, WHDR_BEGINLOOP, WHDR_ENDLOOP,
};

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

/// The open output device and what it is playing (the buffers must stay alive while Windows reads them).
struct Player {
    keep: HWAVEOUT,
    keep_hdr: Box<WAVEHDR>,
    #[allow(dead_code)]
    keep_buf: Vec<u8>,
    out: HWAVEOUT,
    cur: Option<(Box<WAVEHDR>, Vec<u8>)>,
}

// (the handles are only used while the PLAYER lock is held)
unsafe impl Send for Player {}

static PLAYER: Mutex<Option<Player>> = Mutex::new(None);
const HDR: u32 = std::mem::size_of::<WAVEHDR>() as u32;

fn format() -> WAVEFORMATEX {
    WAVEFORMATEX { wFormatTag: WAVE_FORMAT_PCM as u16, nChannels: 1, nSamplesPerSec: 22_050, nAvgBytesPerSec: 44_100, nBlockAlign: 2, wBitsPerSample: 16, cbSize: 0 }
}

fn open_player() -> Option<Player> {
    unsafe {
        let fmt = format();
        let (mut keep, mut out) = (HWAVEOUT::default(), HWAVEOUT::default());
        if waveOutOpen(Some(&mut keep), WAVE_MAPPER, &fmt, 0, 0, CALLBACK_NULL) != 0 {
            return None;
        }
        if waveOutOpen(Some(&mut out), WAVE_MAPPER, &fmt, 0, 0, CALLBACK_NULL) != 0 {
            let _ = waveOutClose(keep);
            return None;
        }
        // one second of "almost silence" (a single least-significant bit, far below anything audible), looping for ever
        let mut keep_buf: Vec<u8> = Vec::with_capacity(44_100);
        for i in 0..22_050 {
            keep_buf.extend_from_slice(&(if i % 2 == 0 { 1i16 } else { -1i16 }).to_le_bytes());
        }
        let mut hdr = Box::new(WAVEHDR { lpData: PSTR(keep_buf.as_mut_ptr()), dwBufferLength: keep_buf.len() as u32, dwFlags: WHDR_BEGINLOOP | WHDR_ENDLOOP, dwLoops: u32::MAX, ..Default::default() });
        if waveOutPrepareHeader(keep, &mut *hdr, HDR) != 0 || waveOutWrite(keep, &mut *hdr, HDR) != 0 {
            let _ = waveOutClose(keep);
            let _ = waveOutClose(out);
            return None;
        }
        Some(Player { keep, keep_hdr: hdr, keep_buf, out, cur: None })
    }
}

/// Keep the audio device ready (called when the sound effects are switched on), or let it go (when they are switched off).
pub fn set_enabled(on: bool) {
    let Ok(mut g) = PLAYER.lock() else { return };
    if on {
        if g.is_none() {
            *g = open_player();
        }
    } else if let Some(mut p) = g.take() {
        unsafe {
            let _ = waveOutReset(p.out);
            if let Some((mut h, _)) = p.cur.take() {
                let _ = waveOutUnprepareHeader(p.out, &mut *h, HDR);
            }
            let _ = waveOutReset(p.keep);
            let _ = waveOutUnprepareHeader(p.keep, &mut *p.keep_hdr, HDR);
            let _ = waveOutClose(p.out);
            let _ = waveOutClose(p.keep);
        }
    }
}

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
    if let Ok(mut g) = PLAYER.lock() {
        if g.is_none() {
            *g = open_player();
        }
        if let Some(p) = g.as_mut() {
            unsafe {
                // stop what is still playing, then start this one at once
                let _ = waveOutReset(p.out);
                if let Some((mut h, _)) = p.cur.take() {
                    let _ = waveOutUnprepareHeader(p.out, &mut *h, HDR);
                }
                let mut hdr = Box::new(WAVEHDR { lpData: PSTR(buf.as_mut_ptr().add(44)), dwBufferLength: (buf.len() - 44) as u32, ..Default::default() });
                if waveOutPrepareHeader(p.out, &mut *hdr, HDR) == 0 && waveOutWrite(p.out, &mut *hdr, HDR) == 0 {
                    p.cur = Some((hdr, buf));
                    return;
                }
            }
        }
    }
    // the fallback: the plain Windows call (the buffer is kept alive for a while: Windows reads it while it plays)
    static KEEP: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());
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
