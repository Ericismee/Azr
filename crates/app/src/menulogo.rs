use crate::paths;
use azr_platform::icons::Bitmap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

static GEN: AtomicU32 = AtomicU32::new(0);
const STEM: &str = "menu_logo";
const EXTS: [&str; 9] = [
    "png", "jpg", "jpeg", "bmp", "ico", "gif", "webp", "tif", "tiff",
];

fn file_for(ext: &str) -> PathBuf {
    image_file(STEM, ext)
}

fn image_file(stem: &str, ext: &str) -> PathBuf {
    paths::config_dir().join(format!("{stem}.{ext}"))
}

pub fn path() -> Option<PathBuf> {
    EXTS.iter().map(|e| file_for(e)).find(|p| p.exists())
}

pub fn generation() -> u32 {
    GEN.load(Ordering::Relaxed)
}

fn clear() {
    for e in EXTS {
        let _ = std::fs::remove_file(file_for(e));
    }
}

pub fn set(src: &str) -> bool {
    set_image(STEM, src)
}

pub(crate) fn launcher_path() -> Option<PathBuf> {
    EXTS.iter()
        .map(|e| image_file("launchpad_icon", e))
        .find(|p| p.exists())
}

pub(crate) fn set_launcher(src: &str) -> bool {
    set_image("launchpad_icon", src)
}

fn set_image(stem: &str, src: &str) -> bool {
    use std::io::{Read, Write};
    const MAX_BYTES: u64 = 8 * 1024 * 1024;
    let ext = Path::new(src)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !EXTS.contains(&ext.as_str()) {
        return false;
    }
    let Ok(file) = std::fs::File::open(src) else {
        return false;
    };
    let Ok(meta) = file.metadata() else {
        return false;
    };
    if !meta.is_file() || meta.len() > MAX_BYTES {
        return false;
    }
    let mut bytes = Vec::new();
    if file.take(MAX_BYTES + 1).read_to_end(&mut bytes).is_err() || bytes.len() as u64 > MAX_BYTES {
        return false;
    }
    if std::fs::create_dir_all(paths::config_dir()).is_err() {
        return false;
    }
    let id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temp = paths::config_dir().join(format!("menu-logo-{}-{id}.{ext}", std::process::id()));
    let Ok(mut staged) = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
    else {
        return false;
    };
    if staged.write_all(&bytes).is_err() {
        drop(staged);
        let _ = std::fs::remove_file(&temp);
        return false;
    }
    drop(staged);
    let valid = azr_platform::wic::decode_logo_file_fit(&temp.to_string_lossy(), 32).is_some();
    if !valid || std::fs::rename(&temp, image_file(stem, &ext)).is_err() {
        let _ = std::fs::remove_file(temp);
        return false;
    }
    for old in EXTS {
        if old != ext {
            let _ = std::fs::remove_file(image_file(stem, old));
        }
    }
    GEN.fetch_add(1, Ordering::Relaxed);
    true
}

pub fn reset() {
    clear();
    GEN.fetch_add(1, Ordering::Relaxed);
}

pub fn bitmap(px: u32, rgb: [u8; 3]) -> Option<Bitmap> {
    let mode = crate::prefs::get().menu_logo;
    if mode == 0 {
        return None;
    }
    let bundled = std::env::current_exe()
        .ok()?
        .parent()?
        .join("brand/azr-liquid-glass.png");
    let source = if mode == 2 {
        path().unwrap_or(bundled)
    } else {
        bundled
    };
    let b = azr_platform::wic::decode_logo_file_fit(&source.to_string_lossy(), (px * 4).max(64))?;
    Some(fit(trim(template(b, rgb)), px, px))
}

pub fn trim(b: Bitmap) -> Bitmap {
    let (w, h) = (b.w as usize, b.h as usize);
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0usize, 0usize);
    for y in 0..h {
        for x in 0..w {
            if b.bgra[(y * w + x) * 4 + 3] > 8 {
                x0 = x0.min(x);
                x1 = x1.max(x);
                y0 = y0.min(y);
                y1 = y1.max(y);
            }
        }
    }
    if x0 > x1 {
        return b;
    }
    let (cw, ch) = (x1 - x0 + 1, y1 - y0 + 1);
    let mut out = vec![0u8; cw * ch * 4];
    for y in 0..ch {
        let s = ((y0 + y) * w + x0) * 4;
        out[y * cw * 4..(y + 1) * cw * 4].copy_from_slice(&b.bgra[s..s + cw * 4]);
    }
    Bitmap {
        w: cw as u32,
        h: ch as u32,
        bgra: out,
    }
}

pub fn fit(b: Bitmap, max_w: u32, max_h: u32) -> Bitmap {
    let k = (max_w as f32 / b.w.max(1) as f32)
        .min(max_h as f32 / b.h.max(1) as f32)
        .min(1.0);
    let (ow, oh) = (
        ((b.w as f32 * k).round() as u32).max(1),
        ((b.h as f32 * k).round() as u32).max(1),
    );
    if (ow, oh) == (b.w, b.h) {
        return b;
    }
    let mut out = vec![0u8; (ow * oh * 4) as usize];
    let (sx, sy) = (b.w as f32 / ow as f32, b.h as f32 / oh as f32);
    for y in 0..oh {
        let (ya, yb) = (
            (y as f32 * sy) as u32,
            (((y + 1) as f32 * sy).ceil() as u32)
                .min(b.h)
                .max((y as f32 * sy) as u32 + 1),
        );
        for x in 0..ow {
            let (xa, xb) = (
                (x as f32 * sx) as u32,
                (((x + 1) as f32 * sx).ceil() as u32)
                    .min(b.w)
                    .max((x as f32 * sx) as u32 + 1),
            );
            let mut acc = [0u32; 4];
            let mut n = 0u32;
            for yy in ya..yb {
                for xx in xa..xb {
                    let i = ((yy * b.w + xx) * 4) as usize;
                    for c in 0..4 {
                        acc[c] += b.bgra[i + c] as u32;
                    }
                    n += 1;
                }
            }
            let o = ((y * ow + x) * 4) as usize;
            for c in 0..4 {
                out[o + c] = (acc[c] / n.max(1)) as u8;
            }
        }
    }
    Bitmap {
        w: ow,
        h: oh,
        bgra: out,
    }
}

fn unmul(px: &[u8]) -> (u32, u32, u32, u32) {
    let a = px[3] as u32;
    if a == 0 {
        return (0, 0, 0, 0);
    }
    (
        px[2] as u32 * 255 / a,
        px[1] as u32 * 255 / a,
        px[0] as u32 * 255 / a,
        a,
    )
}

fn lum(r: u32, g: u32, b: u32) -> i32 {
    ((r * 54 + g * 183 + b * 19) / 256) as i32
}

pub fn template(b: Bitmap, rgb: [u8; 3]) -> Bitmap {
    let mut seen = 0u32;
    let mut gray = 0u32;
    let mut opaque = true;
    for p in b.bgra.chunks_exact(4) {
        let (r, g, bl, a) = unmul(p);
        if a < 250 {
            opaque = false;
        }
        if a > 16 {
            seen += 1;
            let hi = r.max(g).max(bl);
            let lo = r.min(g).min(bl);
            if hi - lo <= 24 {
                gray += 1;
            }
        }
    }
    if seen == 0 || gray * 100 < seen * 97 {
        return b;
    }
    let alpha: Vec<u32> = if opaque {
        let bg = {
            let (r, g, bl, _) = unmul(&b.bgra[0..4]);
            lum(r, g, bl)
        };
        let diffs: Vec<i32> = b
            .bgra
            .chunks_exact(4)
            .map(|p| {
                let (r, g, bl, _) = unmul(p);
                (lum(r, g, bl) - bg).abs()
            })
            .collect();
        let peak = diffs.iter().copied().max().unwrap_or(0).max(1);
        diffs
            .iter()
            .map(|d| (*d as u32 * 255 / peak as u32).min(255))
            .collect()
    } else {
        b.bgra.chunks_exact(4).map(|p| p[3] as u32).collect()
    };
    let mut out = vec![0u8; b.bgra.len()];
    for (i, a) in alpha.iter().enumerate() {
        let o = i * 4;
        out[o] = (rgb[2] as u32 * a / 255) as u8;
        out[o + 1] = (rgb[1] as u32 * a / 255) as u8;
        out[o + 2] = (rgb[0] as u32 * a / 255) as u8;
        out[o + 3] = *a as u8;
    }
    Bitmap {
        w: b.w,
        h: b.h,
        bgra: out,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn importing_current_logo_again_is_safe_and_failed_import_preserves_it() {
        use windows::Win32::System::Com::{
            CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED,
        };
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
        }
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("azr-logo-test-{}-{id}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let previous = std::env::var_os("AZR_CONFIG_DIR");
        std::env::set_var("AZR_CONFIG_DIR", &dir);
        let source =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/brand/azr-liquid-glass.png");
        let first = set(&source.to_string_lossy());
        let current = file_for("png");
        let second = set(&current.to_string_lossy());
        let before = std::fs::read(&current).ok();
        let bad = dir.join("bad.png");
        std::fs::write(&bad, b"not an image").unwrap();
        let rejected = !set(&bad.to_string_lossy());
        let after = std::fs::read(&current).ok();
        if let Some(value) = previous {
            std::env::set_var("AZR_CONFIG_DIR", value);
        } else {
            std::env::remove_var("AZR_CONFIG_DIR");
        }
        unsafe {
            CoUninitialize();
        }
        for name in ["menu_logo.png", "bad.png"] {
            let _ = std::fs::remove_file(dir.join(name));
        }
        let _ = std::fs::remove_dir(&dir);
        assert!(
            first && second,
            "Re-selecting the current logo must not delete it"
        );
        assert!(
            rejected && before.is_some() && before == after,
            "Invalid replacement must preserve the current logo"
        );
    }

    fn bmp(px: &[[u8; 4]]) -> Bitmap {
        Bitmap {
            w: px.len() as u32,
            h: 1,
            bgra: px.iter().flatten().copied().collect(),
        }
    }

    #[test]
    fn black_logo_on_transparent_takes_bar_color() {
        let out = template(
            bmp(&[[0, 0, 0, 255], [0, 0, 0, 0], [0, 0, 0, 128]]),
            [255, 255, 255],
        );
        assert_eq!(
            out.bgra,
            vec![255, 255, 255, 255, 0, 0, 0, 0, 128, 128, 128, 128]
        );
    }

    #[test]
    fn colorful_logo_is_left_alone() {
        let src = bmp(&[[0, 0, 255, 255], [0, 255, 0, 255], [255, 0, 0, 255]]);
        assert_eq!(template(src.clone(), [255, 255, 255]).bgra, src.bgra);
    }

    #[test]
    fn padding_is_trimmed_and_logo_fits_its_box() {
        let mut px = vec![[0u8; 4]; 100];
        for y in 3..7 {
            for x in 4..6 {
                px[y * 10 + x] = [255, 255, 255, 255];
            }
        }
        let b = Bitmap {
            w: 10,
            h: 10,
            bgra: px.iter().flatten().copied().collect(),
        };
        let t = trim(b);
        assert_eq!((t.w, t.h), (2, 4));
        let f = fit(t, 10, 2);
        assert_eq!((f.w, f.h), (1, 2));
        assert_eq!(&f.bgra[0..4], &[255, 255, 255, 255]);
    }

    #[test]
    fn opaque_black_on_white_becomes_a_mask() {
        let out = template(bmp(&[[255, 255, 255, 255], [0, 0, 0, 255]]), [10, 20, 30]);
        assert_eq!(&out.bgra[0..4], &[0, 0, 0, 0]);
        assert_eq!(&out.bgra[4..8], &[30, 20, 10, 255]);
    }
}
