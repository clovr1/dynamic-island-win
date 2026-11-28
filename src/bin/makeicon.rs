//! Draws the app icon (assets/app.ico + assets/app.png). Run: cargo run --release --bin makeicon
use island::gfx::fill_rrect;
use tiny_skia::Pixmap;

/// Bloop: a white pill, and bubbles in coral that rise from it, each bigger than the last (bloop, bloop, bloop). Flat, on near-black.
fn icon(n: u32) -> Pixmap {
    let s = n as f32;
    let mut pm = Pixmap::new(n, n).unwrap();
    fill_rrect(&mut pm, s * 0.03, s * 0.03, s * 0.94, s * 0.94, [s * 0.225; 4], [16, 16, 18], 1.0);
    let (pw, ph) = (s * 0.62, s * 0.2);
    fill_rrect(&mut pm, s * 0.5 - pw / 2.0, s * 0.72 - ph / 2.0, pw, ph, [ph / 2.0; 4], [255, 255, 255], 1.0);
    for (x, y, r) in [(0.34f32, 0.5f32, 0.035f32), (0.45, 0.36, 0.055), (0.62, 0.24, 0.085)] {
        fill_rrect(&mut pm, (x - r) * s, (y - r) * s, 2.0 * r * s, 2.0 * r * s, [r * s; 4], [255, 90, 60], 1.0);
    }
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
