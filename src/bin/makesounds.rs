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
            // the pitch falls fast: it is a "tok", not a note
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

/// Tap: only small taps, different in pitch and number.
fn pack_tap(dir: &str) {
    let mut n = tap(240.0, 0.06, 1.0);
    mix_at(&mut n, &tap(300.0, 0.06, 0.8), 0.085);
    write_wav(&format!("{dir}/notify.wav"), &finish(n, 0.22));
    write_wav(&format!("{dir}/status.wav"), &finish(tap(280.0, 0.06, 1.0), 0.2));
    write_wav(&format!("{dir}/pause.wav"), &finish(tap(190.0, 0.055, 1.0), 0.2));
    write_wav(&format!("{dir}/play.wav"), &finish(tap(260.0, 0.055, 1.0), 0.2));
    let mut k = tap(330.0, 0.04, 0.9);
    mix_at(&mut k, &tap(380.0, 0.04, 0.7), 0.05);
    write_wav(&format!("{dir}/skip.wav"), &finish(k, 0.17));
}

fn pack_bloop(dir: &str) {
    write_wav(&format!("{dir}/notify.wav"), &finish({ let mut n = Vec::new(); mix_at(&mut n, &bell(659.25, 0.45, 1.0), 0.0); mix_at(&mut n, &bell(880.0, 0.5, 0.9), 0.1); n }, 0.2));
    write_wav(&format!("{dir}/status.wav"), &finish({ let mut s = bell(880.0, 0.3, 1.0); mix_at(&mut s, &bloop(240.0, 160.0, 0.05, 0.08, 0.025, 0.4), 0.0); s }, 0.15));
    write_wav(&format!("{dir}/pause.wav"), &finish(bloop(480.0, 320.0, 0.06, 0.14, 0.032, 1.0), 0.13));
    write_wav(&format!("{dir}/play.wav"), &finish(bloop(320.0, 500.0, 0.06, 0.14, 0.034, 1.0), 0.14));
    write_wav(&format!("{dir}/skip.wav"), &finish({ let mut k = bell(784.0, 0.12, 1.0); mix_at(&mut k, &bell(1046.5, 0.14, 0.85), 0.045); k }, 0.11));
}

/// Glass: small, clear taps.
fn pack_glass(dir: &str) {
    let mut n = Vec::new();
    mix_at(&mut n, &glass(1318.5, 0.5, 1.0), 0.0);
    mix_at(&mut n, &glass(1760.0, 0.55, 0.8), 0.11);
    write_wav(&format!("{dir}/notify.wav"), &finish(n, 0.15));
    write_wav(&format!("{dir}/status.wav"), &finish(glass(1568.0, 0.35, 1.0), 0.12));
    write_wav(&format!("{dir}/pause.wav"), &finish(glass(1046.5, 0.18, 1.0), 0.1));
    write_wav(&format!("{dir}/play.wav"), &finish(glass(1396.9, 0.18, 1.0), 0.1));
    let mut k = glass(1760.0, 0.1, 1.0);
    mix_at(&mut k, &glass(2349.0, 0.12, 0.7), 0.04);
    write_wav(&format!("{dir}/skip.wav"), &finish(k, 0.08));
}

/// Wood: warm plucked notes (kalimba).
fn pack_wood(dir: &str) {
    let mut n = Vec::new();
    mix_at(&mut n, &pluck(523.25, 0.4, 1.0), 0.0);
    mix_at(&mut n, &pluck(659.25, 0.4, 0.9), 0.09);
    mix_at(&mut n, &pluck(783.99, 0.5, 0.85), 0.18);
    write_wav(&format!("{dir}/notify.wav"), &finish(n, 0.2));
    write_wav(&format!("{dir}/status.wav"), &finish(pluck(698.46, 0.3, 1.0), 0.16));
    write_wav(&format!("{dir}/pause.wav"), &finish(pluck(392.0, 0.2, 1.0), 0.15));
    write_wav(&format!("{dir}/play.wav"), &finish(pluck(587.33, 0.2, 1.0), 0.15));
    let mut k = pluck(783.99, 0.1, 1.0);
    mix_at(&mut k, &pluck(1046.5, 0.12, 0.7), 0.04);
    write_wav(&format!("{dir}/skip.wav"), &finish(k, 0.1));
}

/// Air: breaths and soft whooshes, almost no pitch at all.
fn pack_air(dir: &str) {
    let mut n = air(500.0, 2600.0, 0.4, 0.45, 1.0);
    mix_at(&mut n, &glass(1568.0, 0.35, 0.18), 0.17);
    write_wav(&format!("{dir}/notify.wav"), &finish(n, 0.13));
    write_wav(&format!("{dir}/status.wav"), &finish(air(900.0, 2200.0, 0.22, 0.4, 1.0), 0.1));
    write_wav(&format!("{dir}/pause.wav"), &finish(air(1800.0, 600.0, 0.16, 0.35, 1.0), 0.1));
    write_wav(&format!("{dir}/play.wav"), &finish(air(600.0, 1800.0, 0.16, 0.6, 1.0), 0.1));
    write_wav(&format!("{dir}/skip.wav"), &finish(air(800.0, 3200.0, 0.14, 0.5, 1.0), 0.09));
}

fn main() {
    for (name, f) in [("bloop", pack_bloop as fn(&str)), ("glass", pack_glass), ("wood", pack_wood), ("air", pack_air), ("tap", pack_tap)] {
        let dir = format!("assets/sounds/{name}");
        std::fs::create_dir_all(&dir).unwrap();
        f(&dir);
    }
}
