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
    let partials = [(1.0f32, 1.0f32, 0.20f32), (2.0, 0.32, 0.09), (3.01, 0.12, 0.045), (4.2, 0.05, 0.03)];
    (0..n)
        .map(|i| {
            let t = i as f32 / SR as f32;
            let attack = 1.0 - (-t / 0.002).exp();
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

fn main() {
    let dir = "assets/sounds";
    std::fs::create_dir_all(dir).unwrap();
    // the island grows: a rising bloop
    write_wav(&format!("{dir}/open.wav"), &finish(bloop(420.0, 840.0, 0.15, 0.24, 0.075, 1.0), 0.55));
    // the island shrinks: a falling, softer bloop
    write_wav(&format!("{dir}/close.wav"), &finish(bloop(760.0, 390.0, 0.13, 0.2, 0.055, 1.0), 0.42));
    // a notification: two soft notes going up (B5, E6)
    let mut n = Vec::new();
    mix_at(&mut n, &bell(987.77, 0.5, 1.0), 0.0);
    mix_at(&mut n, &bell(1318.51, 0.55, 0.9), 0.09);
    write_wav(&format!("{dir}/notify.wav"), &finish(n, 0.6));
    // a status pill (Wi-Fi, battery ...): one short, bright blip (D6) with a low "pop" under it
    let mut s = bell(1174.66, 0.3, 1.0);
    mix_at(&mut s, &bloop(300.0, 180.0, 0.05, 0.08, 0.025, 0.5), 0.0);
    write_wav(&format!("{dir}/status.wav"), &finish(s, 0.5));
}
