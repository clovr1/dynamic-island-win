//! Synthesizes the island's sound effects (original sounds, made from sine waves): assets/sounds/*.wav
//! Run: cargo run --release --bin makesounds
use std::f32::consts::TAU;

const SR: u32 = 44_100;

fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// A soft "bloop": a sine that glides from `f0` to `f1`, with a gentle overtone and a quick, round decay.
fn bloop(f0: f32, f1: f32, glide: f32, len: f32, tau: f32, amp: f32) -> Vec<f32> {
    let n = (len * SR as f32) as usize;
    let mut out = Vec::with_capacity(n);
    let mut phase = 0.0f32;
    let mut lp = 0.0f32;
    for i in 0..n {
        let t = i as f32 / SR as f32;
        let f = f0 + (f1 - f0) * ease(t / glide);
        phase += TAU * f / SR as f32;
        let env = (1.0 - (-t / 0.006).exp()) * (-t / tau).exp();
        let s = (phase.sin() * 0.85 + (2.0 * phase).sin() * 0.14 + (3.0 * phase).sin() * 0.03) * env * amp;
        lp += (s - lp) * 0.55; // takes the edge off
        out.push(lp);
    }
    out
}

/// A soft bell-like note (a marimba / glass tap): a few partials, the higher ones dying faster.
fn bell(freq: f32, len: f32, amp: f32) -> Vec<f32> {
    let n = (len * SR as f32) as usize;
    let partials = [(1.0f32, 1.0f32, 0.16f32), (2.0, 0.14, 0.06), (3.01, 0.04, 0.03)];
    (0..n)
        .map(|i| {
            let t = i as f32 / SR as f32;
            let attack = 1.0 - (-t / 0.006).exp(); // a soft start: no click
            let mut s = 0.0;
            for (m, a, tau) in partials {
                s += (TAU * freq * m * t).sin() * a * (-t / tau).exp();
            }
            s * attack * amp
        })
        .collect()
}

fn mix_at(dst: &mut Vec<f32>, src: &[f32], at: f32) {
    let off = (at * SR as f32) as usize;
    if dst.len() < off + src.len() {
        dst.resize(off + src.len(), 0.0);
    }
    for (i, v) in src.iter().enumerate() {
        dst[off + i] += v;
    }
}

fn finish(mut s: Vec<f32>, peak: f32) -> Vec<i16> {
    // a short fade at the end, then the loudest point set to `peak`
    let fade = (0.012 * SR as f32) as usize;
    let n = s.len();
    for i in 0..fade.min(n) {
        s[n - 1 - i] *= i as f32 / fade as f32;
    }
    let m = s.iter().fold(0.0f32, |a, v| a.max(v.abs())).max(1e-6);
    s.iter().map(|v| (v / m * peak * 32767.0) as i16).collect()
}

fn write_wav(path: &str, pcm: &[i16]) {
    let data_len = (pcm.len() * 2) as u32;
    let mut b = Vec::with_capacity(44 + pcm.len() * 2);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes()); // PCM
    b.extend_from_slice(&1u16.to_le_bytes()); // mono
    b.extend_from_slice(&SR.to_le_bytes());
    b.extend_from_slice(&(SR * 2).to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for v in pcm {
        b.extend_from_slice(&v.to_le_bytes());
    }
    std::fs::write(path, b).unwrap();
    println!("{path}: {:.0} ms, peak {}", pcm.len() as f32 * 1000.0 / SR as f32, pcm.iter().map(|v| v.unsigned_abs()).max().unwrap_or(0));
}

/// Band-limited noise that sweeps in pitch: a soft "whoosh" (a breath of air).
fn air(f0: f32, f1: f32, len: f32, peak_at: f32, amp: f32) -> Vec<f32> {
    let n = (len * SR as f32) as usize;
    let mut seed = 0x1234_5678u32;
    let mut noise = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed as f32 / u32::MAX as f32) * 2.0 - 1.0
    };
    let (mut lp1, mut lp2) = (0.0f32, 0.0f32);
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let f = f0 + (f1 - f0) * ease(t);
            let a = (TAU * f / SR as f32).min(1.2).sin().abs().min(0.9); // a one-pole low-pass that follows the sweep
            lp1 += (noise() - lp1) * a;
            lp2 += (lp1 - lp2) * a;
            let band = lp1 - lp2; // between the two: a band around f
            // rises until `peak_at`, then falls away smoothly
            let env = if t < peak_at { ease(t / peak_at) } else { 1.0 - ease((t - peak_at) / (1.0 - peak_at)) };
            band * env * amp * 6.0
        })
        .collect()
}

/// A tap on glass: inharmonic partials that die quickly (a small, clear "tink").
fn glass(freq: f32, len: f32, amp: f32) -> Vec<f32> {
    let n = (len * SR as f32) as usize;
    let partials = [(1.0f32, 1.0f32, 0.11f32), (2.76, 0.42, 0.06), (5.40, 0.16, 0.035), (8.93, 0.05, 0.02)];
    (0..n)
        .map(|i| {
            let t = i as f32 / SR as f32;
            let attack = 1.0 - (-t / 0.0015).exp();
            let mut s = 0.0;
            for (m, a, tau) in partials {
                s += (TAU * freq * m * t).sin() * a * (-t / tau).exp();
            }
            s * attack * amp
        })
        .collect()
}

/// A plucked wooden note (kalimba / marimba): the note and a bright overtone that fades before it, with a soft thump at the start.
fn pluck(freq: f32, len: f32, amp: f32) -> Vec<f32> {
    let n = (len * SR as f32) as usize;
    (0..n)
        .map(|i| {
            let t = i as f32 / SR as f32;
            let attack = 1.0 - (-t / 0.003).exp();
            let body = (TAU * freq * t).sin() * (-t / 0.11).exp();
            let over = (TAU * freq * 4.0 * t).sin() * 0.22 * (-t / 0.03).exp();
            let thump = (TAU * freq * 0.5 * t).sin() * 0.3 * (-t / 0.02).exp();
            (body + over + thump) * attack * amp
        })
        .collect()
}

/// A small tap, like a fingertip on glass or a haptic "tick": a very short, dry thud (a quickly falling low sine) with a tiny click on top.
fn tap(freq: f32, len: f32, amp: f32) -> Vec<f32> {
    let n = (len * SR as f32) as usize;
    let mut seed = 0x9e37_79b9u32;
    let mut lp = 0.0f32;
    let mut phase = 0.0f32;
    (0..n)
        .map(|i| {
            let t = i as f32 / SR as f32;
            let f = freq * (0.55 + 0.45 * (-t / 0.012).exp());
            phase += TAU * f / SR as f32;
            let body = phase.sin() * (-t / 0.014).exp();
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let n1 = (seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
            lp += (n1 - lp) * 0.35;
            let click = lp * (-t / 0.0025).exp() * 0.35;
            (body + click) * (1.0 - (-t / 0.0008).exp()) * amp
        })
        .collect()
}

/// The island's sounds, in the spirit of the iPhone's system sounds (original recordings-free synthesis, not copies):
///  notification = two soft marimba notes ("Note" / "Bamboo"), status = "bloop - ding" like the charging sound,
///  pause = a low "tok" like the lock sound, play = a light keyboard-click tick, skip = a soft swoosh like "mail sent".
fn pack_ios(dir: &str) {
    let mut n = pluck(783.99, 0.45, 1.0);
    mix_at(&mut n, &pluck(1174.66, 0.5, 0.8), 0.12);
    write_wav(&format!("{dir}/notify.wav"), &finish(n, 0.2));
    let mut s = bloop(260.0, 430.0, 0.07, 0.14, 0.04, 0.9);
    mix_at(&mut s, &bell(880.0, 0.45, 0.9), 0.1);
    write_wav(&format!("{dir}/status.wav"), &finish(s, 0.16));
    write_wav(&format!("{dir}/pause.wav"), &finish(tap(165.0, 0.07, 1.0), 0.2));
    let mut p = tap(310.0, 0.05, 1.0);
    mix_at(&mut p, &tap(900.0, 0.025, 0.3), 0.0);
    write_wav(&format!("{dir}/play.wav"), &finish(p, 0.18));
    write_wav(&format!("{dir}/skip.wav"), &finish(air(700.0, 3200.0, 0.22, 0.55, 1.0), 0.11));
}

fn main() {
    let dir = "assets/sounds";
    std::fs::create_dir_all(dir).unwrap();
    pack_ios(dir);
}
