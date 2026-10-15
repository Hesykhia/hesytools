//! Grabbing a small region of the screen.
//!
//! GDI `BitBlt` from the desktop is plenty fast for the small regions
//! Screen Watch reads (well under a millisecond for a 300×60 box) and works
//! for windowed and borderless-fullscreen games.

use serde::{Deserialize, Serialize};
use windows::Win32::Graphics::Gdi::*;

/// A screen rectangle in physical pixels.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

/// Top-down 32-bit BGRA pixels.
#[derive(Clone)]
pub struct Frame {
    pub w: u32,
    pub h: u32,
    pub bgra: Vec<u8>,
}

impl Frame {
    /// Cheap fingerprint used to skip OCR when nothing changed.
    pub fn fingerprint(&self) -> u64 {
        // FNV-1a over the RGB bytes (alpha is always 255).
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for px in self.bgra.chunks_exact(4) {
            for &b in &px[..3] {
                h ^= b as u64;
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
        }
        h
    }

    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 3]> {
        if x >= self.w || y >= self.h {
            return None;
        }
        let i = ((y * self.w + x) * 4) as usize;
        Some([self.bgra[i + 2], self.bgra[i + 1], self.bgra[i]])
    }

    /// Integer-factor bilinear upscale. OCR reads small game text far more
    /// reliably once glyphs are ~30px+ tall.
    pub fn upscale(&self, k: u32) -> Frame {
        if k <= 1 {
            return self.clone();
        }
        let (w, h) = (self.w * k, self.h * k);
        let mut out = vec![0u8; (w * h * 4) as usize];
        let src = |x: u32, y: u32, c: usize| self.bgra[((y.min(self.h - 1) * self.w + x.min(self.w - 1)) * 4) as usize + c] as f32;
        for y in 0..h {
            let fy = (y as f32 + 0.5) / k as f32 - 0.5;
            let (y0, ty) = (fy.max(0.0).floor() as u32, fy.max(0.0).fract());
            for x in 0..w {
                let fx = (x as f32 + 0.5) / k as f32 - 0.5;
                let (x0, tx) = (fx.max(0.0).floor() as u32, fx.max(0.0).fract());
                let o = ((y * w + x) * 4) as usize;
                for c in 0..3 {
                    let top = src(x0, y0, c) * (1.0 - tx) + src(x0 + 1, y0, c) * tx;
                    let bot = src(x0, y0 + 1, c) * (1.0 - tx) + src(x0 + 1, y0 + 1, c) * tx;
                    out[o + c] = (top * (1.0 - ty) + bot * ty) as u8;
                }
                out[o + 3] = 255;
            }
        }
        Frame { w, h, bgra: out }
    }

    /// Shrink to fit `max_w` wide (for the live preview), nearest-neighbour.
    pub fn thumbnail(&self, max_w: u32) -> Frame {
        if self.w <= max_w {
            return self.clone();
        }
        let w = max_w;
        let h = (self.h as u64 * max_w as u64 / self.w as u64).max(1) as u32;
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            let sy = y * self.h / h;
            for x in 0..w {
                let sx = x * self.w / w;
                let i = ((sy * self.w + sx) * 4) as usize;
                out.extend_from_slice(&self.bgra[i..i + 4]);
            }
        }
        Frame { w, h, bgra: out }
    }
}

pub fn capture(r: Region) -> Result<Frame, String> {
    if r.w == 0 || r.h == 0 || r.w > 8192 || r.h > 8192 {
        return Err("Region is empty or too large".into());
    }
    unsafe {
        let screen = GetDC(None);
        if screen.is_invalid() {
            return Err("Couldn't access the screen".into());
        }
        let mem = CreateCompatibleDC(Some(screen));
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: r.w as i32,
                biHeight: -(r.h as i32), // negative = top-down rows
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let result = match CreateDIBSection(Some(mem), &bmi, DIB_RGB_COLORS, &mut bits, None, 0) {
            Ok(dib) => {
                let old = SelectObject(mem, dib.into());
                let ok = BitBlt(mem, 0, 0, r.w as i32, r.h as i32, Some(screen), r.x, r.y, SRCCOPY | CAPTUREBLT).is_ok();
                let _ = GdiFlush();
                let out = if ok {
                    let len = (r.w * r.h * 4) as usize;
                    Ok(Frame { w: r.w, h: r.h, bgra: std::slice::from_raw_parts(bits as *const u8, len).to_vec() })
                } else {
                    Err("Screen capture failed".to_string())
                };
                SelectObject(mem, old);
                let _ = DeleteObject(dib.into());
                out
            }
            Err(e) => Err(format!("Couldn't allocate capture buffer: {e}")),
        };
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);
        result
    }
}

/// Grab region `r` (relative to the window's top-left corner) from just
/// this window, the way OBS's window capture does: whatever is on top of it
/// (other windows, taskbar previews) isn't in the picture. Returns the frame
/// and the region's current screen position.
pub fn capture_window(handle: isize, r: Region) -> Result<(Frame, (i32, i32)), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Storage::Xps::{PRINT_WINDOW_FLAGS, PrintWindow};
    let (wx, wy, ww, wh) = crate::window::rect(handle).ok_or("The window was closed")?;
    if crate::window::is_minimized(handle) {
        return Err("The window is minimized".into());
    }
    if r.w == 0 || r.h == 0 || r.x < 0 || r.y < 0 || r.x as u32 + r.w > ww || r.y as u32 + r.h > wh {
        return Err("The area isn't inside the window any more (was it resized?)".into());
    }
    unsafe {
        let screen = GetDC(None);
        if screen.is_invalid() {
            return Err("Couldn't access the screen".into());
        }
        let mem = CreateCompatibleDC(Some(screen));
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: ww as i32,
                biHeight: -(wh as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let result = match CreateDIBSection(Some(mem), &bmi, DIB_RGB_COLORS, &mut bits, None, 0) {
            Ok(dib) => {
                let old = SelectObject(mem, dib.into());
                // PW_RENDERFULLCONTENT: also works for GPU-drawn windows
                // (browsers, games in windowed/borderless mode).
                let ok = PrintWindow(HWND(handle as *mut _), mem, PRINT_WINDOW_FLAGS(2)).as_bool();
                let _ = GdiFlush();
                let out = if ok {
                    let all = std::slice::from_raw_parts(bits as *const u8, (ww * wh * 4) as usize);
                    let mut bgra = Vec::with_capacity((r.w * r.h * 4) as usize);
                    for row in r.y as u32..r.y as u32 + r.h {
                        let start = ((row * ww + r.x as u32) * 4) as usize;
                        bgra.extend_from_slice(&all[start..start + (r.w * 4) as usize]);
                    }
                    Ok((Frame { w: r.w, h: r.h, bgra }, (wx + r.x, wy + r.y)))
                } else {
                    Err("Windows couldn't capture that window".to_string())
                };
                SelectObject(mem, old);
                let _ = DeleteObject(dib.into());
                out
            }
            Err(e) => Err(format!("Couldn't allocate capture buffer: {e}")),
        };
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, rgb: [u8; 3]) -> Frame {
        let px = [rgb[2], rgb[1], rgb[0], 255];
        Frame { w, h, bgra: px.repeat((w * h) as usize) }
    }

    #[test]
    fn fingerprint_changes_with_content() {
        let a = solid(10, 10, [10, 20, 30]);
        let mut b = a.clone();
        assert_eq!(a.fingerprint(), b.fingerprint());
        b.bgra[44] ^= 1;
        assert_ne!(a.fingerprint(), b.fingerprint());
    }

    #[test]
    fn upscale_and_thumbnail_sizes() {
        let f = solid(30, 10, [200, 100, 50]);
        let up = f.upscale(3);
        assert_eq!((up.w, up.h), (90, 30));
        assert_eq!(up.pixel(45, 15), Some([200, 100, 50]));
        let t = up.thumbnail(45);
        assert_eq!((t.w, t.h), (45, 15));
    }

    #[test]
    fn captures_a_real_region() {
        let f = capture(Region { x: 0, y: 0, w: 64, h: 32 }).unwrap();
        assert_eq!(f.bgra.len(), 64 * 32 * 4);
    }
}
