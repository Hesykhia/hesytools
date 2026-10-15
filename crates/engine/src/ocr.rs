//! Text recognition with Windows' built-in OCR engine (Windows.Media.Ocr).
//!
//! It ships with Windows, needs no model download, and reads a small region
//! in a few milliseconds, versus the old EasyOCR/PyTorch setup that pinned
//! the CPU and added ~200MB to the exe.

use crate::capture::Frame;
use windows::Globalization::Language;
use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Security::Cryptography::CryptographicBuffer;
use windows::core::HSTRING;

/// Glyphs should be at least this tall for reliable recognition.
const TARGET_HEIGHT: u32 = 48;

pub struct Ocr {
    engine: OcrEngine,
    max_dim: u32,
}

impl Ocr {
    pub fn new() -> Result<Self, String> {
        let engine = OcrEngine::TryCreateFromUserProfileLanguages()
            .or_else(|_| OcrEngine::TryCreateFromLanguage(&Language::CreateLanguage(&HSTRING::from("en-US"))?))
            .map_err(|e| format!("Windows OCR isn't available: {e}. Install an OCR language pack in Settings → Time & language."))?;
        let max_dim = OcrEngine::MaxImageDimension().unwrap_or(10_000);
        Ok(Self { engine, max_dim })
    }

    /// Recognised text, lines joined with newlines.
    pub fn read(&self, frame: &Frame) -> Result<String, String> {
        // Small regions are upscaled so the text is big enough to read.
        let k = (TARGET_HEIGHT / frame.h.max(1)).clamp(1, 4);
        let k = k.min(self.max_dim / frame.w.max(frame.h).max(1)).max(1);
        let f = frame.upscale(k);
        let run = || -> windows::core::Result<String> {
            let buf = CryptographicBuffer::CreateFromByteArray(&f.bgra)?;
            let bmp = SoftwareBitmap::CreateCopyFromBuffer(&buf, BitmapPixelFormat::Bgra8, f.w as i32, f.h as i32)?;
            let result = self.engine.RecognizeAsync(&bmp)?.join()?;
            let lines = result.Lines()?;
            let mut out = Vec::new();
            for line in lines {
                out.push(line.Text()?.to_string());
            }
            Ok(out.join("\n"))
        };
        run().map_err(|e| format!("OCR failed: {e}"))
    }
}
