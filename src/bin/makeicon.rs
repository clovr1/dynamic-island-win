//! Draws the app icon (assets/app.ico + assets/app.png). Run: cargo run --release --bin makeicon
use island::gfx::fill_rrect;
use tiny_skia::Pixmap;

/// Soft blobs that melt together (metaballs): shapes are painted into a coverage grid, blurred, then cut at a threshold.
fn blob(pm: &mut Pixmap, s: f32, shapes: &[(f32, f32, f32, f32, f32)], top: [u8; 3], bottom: [u8; 3]) {
    let n = pm.width() as usize;
    let mut a = vec![0.0f32; n * n];
    for &(x, y, w, h, r) in shapes {
        let (x0, y0, x1, y1) = ((x * s) as i32, (y * s) as i32, ((x + w) * s) as i32, ((y + h) * s) as i32);
        for py in y0.max(0)..y1.min(n as i32) {
            for px in x0.max(0)..x1.min(n as i32) {
                // inside a rounded rectangle with corner radius r?
                let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
                let rr = r * s;
                let dx = (fx - (x * s + rr)).min(0.0).min((x * s + w * s - rr) - fx).abs().max(0.0);
                let _ = dx;
                let cx = fx.max(x * s + rr).min((x * s + w * s - rr).max(x * s + rr));
                let cy = fy.max(y * s + rr).min((y * s + h * s - rr).max(y * s + rr));
                if (fx - cx).hypot(fy - cy) <= rr {
                    a[py as usize * n + px as usize] = 1.0;
                }
            }
        }
    }
    let rad = ((s * 0.028) as usize).max(1);
    for _ in 0..3 {
        let mut t = vec![0.0f32; n * n];
        for y in 0..n {
            for x in 0..n {
                let (lo, hi) = (x.saturating_sub(rad), (x + rad).min(n - 1));
                t[y * n + x] = (lo..=hi).map(|i| a[y * n + i]).sum::<f32>() / (hi - lo + 1) as f32;
            }
        }
        for y in 0..n {
            for x in 0..n {
                let (lo, hi) = (y.saturating_sub(rad), (y + rad).min(n - 1));
                a[y * n + x] = (lo..=hi).map(|i| t[i * n + x]).sum::<f32>() / (hi - lo + 1) as f32;
            }
        }
    }
    let data = pm.data_mut();
    for y in 0..n {
        let k = y as f32 / n as f32;
        let k = ((k - 0.15) / 0.6).clamp(0.0, 1.0);
        for x in 0..n {
            let v = ((a[y * n + x] - 0.40) / 0.035).clamp(0.0, 1.0);
            let v = v * v * (3.0 - 2.0 * v);
            if v <= 0.0 {
                continue;
            }
            let i = (y * n + x) * 4;
            for q in 0..3 {
                let col = top[q] as f32 * (1.0 - k) + bottom[q] as f32 * k;
                data[i + q] = (col * v + data[i + q] as f32 * (1.0 - v)).round() as u8;
            }
            data[i + 3] = 255;
        }
    }
}

/// Bloop: an orange sun-like circle on cream, and a black pill that stretches and lets go of a drop, like water. Flat colours.
fn icon(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    fill_rrect(&mut pm, s * 0.03, s * 0.03, s * 0.94, s * 0.94, [s * 0.225; 4], [244, 238, 225], 1.0);
    let r = 0.2;
    fill_rrect(&mut pm, (0.4 - r) * s, (0.4 - r) * s, 2.0 * r * s, 2.0 * r * s, [r * s; 4], [255, 92, 40], 1.0);
    blob(&mut pm, s, &[(0.14, 0.5, 0.48, 0.22, 0.11), (0.56, 0.54, 0.14, 0.12, 0.05), (0.66, 0.48, 0.2, 0.24, 0.12)], [18, 18, 20], [18, 18, 20]);
    pm
}

fn main() {
    let sizes = [16u32, 20, 24, 32, 40, 48, 64, 128, 256];
    let mut pngs: Vec<(u32, Vec<u8>)> = Vec::new();
    for n in sizes {
        pngs.push((n, icon(n).encode_png().unwrap()));
    }
    // ICO file with PNG pictures
    let mut out: Vec<u8> = vec![0, 0, 1, 0];
    out.extend_from_slice(&(pngs.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * pngs.len();
    for (n, d) in &pngs {
        out.push(if *n >= 256 { 0 } else { *n as u8 });
        out.push(if *n >= 256 { 0 } else { *n as u8 });
        out.extend_from_slice(&[0, 0, 1, 0, 32, 0]);
        out.extend_from_slice(&(d.len() as u32).to_le_bytes());
        out.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += d.len();
    }
    for (_, d) in &pngs {
        out.extend_from_slice(d);
    }
    std::fs::write("assets/app.ico", out).unwrap();
    icon(512).save_png("assets/app.png").unwrap();
    println!("assets/app.ico written ({} pictures)", pngs.len());
}
