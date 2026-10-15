//! Finding a small template image inside a captured region.
//!
//! Coarse-to-fine: compare at reduced size over every position to find a few
//! candidates, then refine those at full resolution. That keeps a typical
//! check (400×300 area, 60×40 icon) to a couple of milliseconds.

use crate::capture::Frame;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use serde::{Deserialize, Serialize};

/// The picture to look for, stored inline as base64 RGBA.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Template {
    pub w: u32,
    pub h: u32,
    /// Base64 of w*h RGBA pixels.
    pub data: String,
}

impl Template {
    pub fn from_frame(f: &Frame) -> Template {
        let mut rgba = Vec::with_capacity(f.bgra.len());
        for px in f.bgra.chunks_exact(4) {
            rgba.extend_from_slice(&[px[2], px[1], px[0], 255]);
        }
        Template { w: f.w, h: f.h, data: B64.encode(rgba) }
    }

    pub fn gray(&self) -> Result<Gray, String> {
        let rgba = B64.decode(&self.data).map_err(|e| format!("Bad image data: {e}"))?;
        if rgba.len() != (self.w * self.h * 4) as usize || self.w == 0 || self.h == 0 {
            return Err("Image data doesn't match its size".into());
        }
        let px = rgba.chunks_exact(4).map(|p| luma(p[0], p[1], p[2])).collect();
        Ok(Gray { w: self.w, h: self.h, px })
    }
}

#[derive(Clone)]
pub struct Gray {
    pub w: u32,
    pub h: u32,
    pub px: Vec<u8>,
}

impl Gray {
    pub fn from_frame(f: &Frame) -> Gray {
        let px = f.bgra.chunks_exact(4).map(|p| luma(p[2], p[1], p[0])).collect();
        Gray { w: f.w, h: f.h, px }
    }

    /// Box-filter downscale by an integer factor.
    fn shrink(&self, k: u32) -> Gray {
        if k <= 1 {
            return self.clone();
        }
        let (w, h) = (self.w / k, self.h / k);
        let mut px = Vec::with_capacity((w * h) as usize);
        for y in 0..h {
            for x in 0..w {
                let mut sum = 0u32;
                for dy in 0..k {
                    let row = ((y * k + dy) * self.w + x * k) as usize;
                    sum += self.px[row..row + k as usize].iter().map(|&v| v as u32).sum::<u32>();
                }
                px.push((sum / (k * k)) as u8);
            }
        }
        Gray { w, h, px }
    }
}

fn luma(r: u8, g: u8, b: u8) -> u8 {
    ((r as u32 * 77 + g as u32 * 150 + b as u32 * 29) >> 8) as u8
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
pub struct Found {
    pub x: u32,
    pub y: u32,
    /// 1.0 = identical, 0.0 = completely different.
    pub score: f64,
}

/// Sum of absolute differences of `t` placed at (ox, oy) in `hay`, giving
/// up early once it exceeds `limit`.
fn sad(hay: &Gray, t: &Gray, ox: u32, oy: u32, limit: u64) -> u64 {
    let mut total = 0u64;
    for y in 0..t.h {
        let hr = ((oy + y) * hay.w + ox) as usize;
        let tr = (y * t.w) as usize;
        let row: u64 = hay.px[hr..hr + t.w as usize]
            .iter()
            .zip(&t.px[tr..tr + t.w as usize])
            .map(|(a, b)| a.abs_diff(*b) as u64)
            .sum();
        total += row;
        if total > limit {
            return total;
        }
    }
    total
}

/// Best place the template appears in `hay`, if it fits at all.
pub fn find(hay: &Gray, tpl: &Gray) -> Option<Found> {
    if tpl.w > hay.w || tpl.h > hay.h {
        return None;
    }
    // Shrink so the template is still >= ~8px on its short side.
    let k = (tpl.w.min(tpl.h) / 8).clamp(1, 4);
    let (sh, st) = (hay.shrink(k), tpl.shrink(k));
    const KEEP: usize = 6;
    let mut best: Vec<(u64, u32, u32)> = Vec::with_capacity(KEEP + 1);
    for y in 0..=(sh.h - st.h) {
        for x in 0..=(sh.w - st.w) {
            let limit = if best.len() < KEEP { u64::MAX } else { best[KEEP - 1].0 };
            let d = sad(&sh, &st, x, y, limit);
            if d < limit {
                best.push((d, x, y));
                best.sort_by_key(|b| b.0);
                best.truncate(KEEP);
            }
        }
    }
    // Refine each candidate at full resolution in a small window.
    let mut winner: Option<(u64, u32, u32)> = None;
    for &(_, cx, cy) in &best {
        let (x0, y0) = ((cx * k).saturating_sub(k), (cy * k).saturating_sub(k));
        let x1 = (cx * k + k).min(hay.w - tpl.w);
        let y1 = (cy * k + k).min(hay.h - tpl.h);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let limit = winner.map_or(u64::MAX, |w| w.0);
                let d = sad(hay, tpl, x, y, limit);
                if d < limit {
                    winner = Some((d, x, y));
                }
            }
        }
    }
    let (d, x, y) = winner?;
    let score = 1.0 - d as f64 / (tpl.w as f64 * tpl.h as f64 * 255.0);
    Some(Found { x, y, score })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pattern(w: u32, h: u32, seed: u32) -> Gray {
        let px = (0..w * h)
            .map(|i| {
                let (x, y) = (i % w, i / w);
                ((x * 7 + y * 13 + seed * 31) ^ (x * y + seed)).wrapping_mul(2654435761) as u8
            })
            .collect();
        Gray { w, h, px }
    }

    fn crop(g: &Gray, x0: u32, y0: u32, w: u32, h: u32) -> Gray {
        let mut px = Vec::new();
        for y in y0..y0 + h {
            px.extend_from_slice(&g.px[(y * g.w + x0) as usize..(y * g.w + x0 + w) as usize]);
        }
        Gray { w, h, px }
    }

    #[test]
    fn finds_an_exact_crop() {
        let hay = pattern(400, 300, 1);
        let tpl = crop(&hay, 213, 97, 60, 40);
        let f = find(&hay, &tpl).unwrap();
        assert_eq!((f.x, f.y), (213, 97));
        assert!(f.score > 0.999, "{f:?}");
    }

    #[test]
    fn absent_image_scores_low() {
        let hay = pattern(300, 200, 1);
        let tpl = crop(&pattern(100, 100, 9), 10, 10, 40, 30);
        assert!(find(&hay, &tpl).unwrap().score < 0.9);
    }

    #[test]
    fn template_round_trips_through_base64() {
        let f = Frame { w: 2, h: 1, bgra: vec![0, 0, 255, 255, 255, 0, 0, 255] };
        let g = Template::from_frame(&f).gray().unwrap();
        assert_eq!(g.px, vec![luma(255, 0, 0), luma(0, 0, 255)]);
    }
}
