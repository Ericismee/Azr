use crate::icons::Bitmap;
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_WICPixelFormat32bppPBGRA, IWICImagingFactory,
    WICBitmapDitherTypeNone, WICBitmapInterpolationModeFant, WICBitmapPaletteTypeCustom,
    WICDecodeMetadataCacheOnDemand,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::UI::Shell::SHCreateMemStream;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_artwork_preserves_landscape_and_portrait_dimensions() {
        use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok().unwrap() };
        for (width, height) in [(4u32, 2u32), (2, 4), (4, 4)] {
            let len = 54 + (width * 3).next_multiple_of(4) * height;
            let mut bmp = vec![0u8; len as usize];
            bmp[..2].copy_from_slice(b"BM");
            bmp[2..6].copy_from_slice(&len.to_le_bytes());
            bmp[10..14].copy_from_slice(&54u32.to_le_bytes());
            bmp[14..18].copy_from_slice(&40u32.to_le_bytes());
            bmp[18..22].copy_from_slice(&width.to_le_bytes());
            bmp[22..26].copy_from_slice(&height.to_le_bytes());
            bmp[26..28].copy_from_slice(&1u16.to_le_bytes());
            bmp[28..30].copy_from_slice(&24u16.to_le_bytes());
            let art = decode_artwork(&bmp, 128).unwrap();
            assert_eq!((art.w, art.h), (width * 32, height * 32));
            assert_eq!(art.bgra.len(), (art.w * art.h * 4) as usize);
        }
        unsafe { CoUninitialize() };
    }
}

pub fn decode_square(bytes: &[u8], px: u32) -> Option<Bitmap> {
    decode_memory(bytes, px, false)
}

pub fn decode_artwork(bytes: &[u8], px: u32) -> Option<Bitmap> {
    decode_memory(bytes, px, true)
}

fn decode_memory(bytes: &[u8], px: u32, preserve_aspect: bool) -> Option<Bitmap> {
    unsafe {
        let factory: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).ok()?;
        let stream = SHCreateMemStream(Some(bytes))?;
        let decoder = factory
            .CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnDemand)
            .ok()?;
        let frame = decoder.GetFrame(0).ok()?;
        let (w, h) = if preserve_aspect {
            let (mut w, mut h) = (0, 0);
            frame.GetSize(&mut w, &mut h).ok()?;
            fit_dimensions(w, h, px)?
        } else {
            (px, px)
        };
        let scaler = factory.CreateBitmapScaler().ok()?;
        scaler
            .Initialize(&frame, w, h, WICBitmapInterpolationModeFant)
            .ok()?;
        let conv = factory.CreateFormatConverter().ok()?;
        conv.Initialize(
            &scaler,
            &GUID_WICPixelFormat32bppPBGRA,
            WICBitmapDitherTypeNone,
            None,
            0.0,
            WICBitmapPaletteTypeCustom,
        )
        .ok()?;
        let mut buf = vec![0u8; (w * h * 4) as usize];
        conv.CopyPixels(std::ptr::null(), w * 4, &mut buf).ok()?;
        Some(Bitmap { w, h, bgra: buf })
    }
}

fn fit_dimensions(w: u32, h: u32, px: u32) -> Option<(u32, u32)> {
    if w == 0 || h == 0 || px == 0 {
        return None;
    }
    let scale = px as f64 / w.max(h) as f64;
    Some((
        ((w as f64 * scale).round() as u32).max(1),
        ((h as f64 * scale).round() as u32).max(1),
    ))
}

pub fn decode_file_fit(path: &str, px: u32) -> Option<Bitmap> {
    decode_file_fit_limited(path, px, u64::MAX)
}

pub fn decode_logo_file_fit(path: &str, px: u32) -> Option<Bitmap> {
    decode_file_fit_limited(path, px.min(1024), 16 * 1024 * 1024)
}

fn decode_file_fit_limited(path: &str, px: u32, max_pixels: u64) -> Option<Bitmap> {
    unsafe {
        let factory: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).ok()?;
        let decoder = factory
            .CreateDecoderFromFilename(
                &windows::core::HSTRING::from(path),
                None,
                windows::Win32::Foundation::GENERIC_READ,
                WICDecodeMetadataCacheOnDemand,
            )
            .ok()?;
        let frame = decoder.GetFrame(0).ok()?;
        let (mut w, mut h) = (0u32, 0u32);
        frame.GetSize(&mut w, &mut h).ok()?;
        if w as u64 * h as u64 > max_pixels {
            return None;
        }
        let (sw, sh) = fit_dimensions(w, h, px)?;
        let scaler = factory.CreateBitmapScaler().ok()?;
        scaler
            .Initialize(&frame, sw, sh, WICBitmapInterpolationModeFant)
            .ok()?;
        let conv = factory.CreateFormatConverter().ok()?;
        conv.Initialize(
            &scaler,
            &GUID_WICPixelFormat32bppPBGRA,
            WICBitmapDitherTypeNone,
            None,
            0.0,
            WICBitmapPaletteTypeCustom,
        )
        .ok()?;
        let mut small = vec![0u8; (sw * sh * 4) as usize];
        conv.CopyPixels(std::ptr::null(), sw * 4, &mut small).ok()?;
        let mut out = vec![0u8; (px * px * 4) as usize];
        let (ox, oy) = ((px - sw) / 2, (px - sh) / 2);
        for y in 0..sh {
            let d = (((oy + y) * px + ox) * 4) as usize;
            let s = (y * sw * 4) as usize;
            out[d..d + (sw * 4) as usize].copy_from_slice(&small[s..s + (sw * 4) as usize]);
        }
        Some(Bitmap {
            w: px,
            h: px,
            bgra: out,
        })
    }
}
