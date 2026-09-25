//! # Realistic Companion Sprite Sheet & Renderer
//!
//! Decodes and renders photographic companion assets (Oreo the Tuxedo Cat)
//! with alpha-channel chroma-keying, bilinear scaling, and GDI BGRA output
//! for native Windows desktop rendering.

#![allow(clippy::chunks_exact_to_as_chunks)]

use std::collections::HashMap;

/// Embedded photographic assets for the official realistic companion (Oreo).
const SIT_0_PNG: &[u8] = include_bytes!("../assets/realistic/sit_0.png");
const SLEEP_0_PNG: &[u8] = include_bytes!("../assets/realistic/sleep_0.png");
const STRETCH_0_PNG: &[u8] = include_bytes!("../assets/realistic/stretch_0.png");
const CURIOUS_0_PNG: &[u8] = include_bytes!("../assets/realistic/curious_0.png");
const ASK_0_PNG: &[u8] = include_bytes!("../assets/realistic/ask_0.png");
const IDLE_0_PNG: &[u8] = include_bytes!("../assets/realistic/idle_0.png");
const HIGH_FIVE_0_PNG: &[u8] = include_bytes!("../assets/realistic/high_five_0.png");

/// A decoded RGBA8 photographic animation frame.
#[derive(Clone, Debug)]
pub struct DecodedFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl DecodedFrame {
    /// Decodes an embedded PNG byte buffer into RGBA8 pixels.
    pub fn from_png_bytes(bytes: &[u8]) -> Self {
        let img = image::load_from_memory(bytes)
            .expect("embedded realistic companion PNG must be valid PNG format");
        let rgba = img.to_rgba8();
        let (width, height) = (rgba.width(), rgba.height());
        Self {
            width,
            height,
            rgba: rgba.into_raw(),
        }
    }
}

/// Photographic companion sprite collection for Oreo (Bıyıklı Smokin).
#[derive(Clone, Debug)]
pub struct RealisticCompanionSheet {
    pub frames: HashMap<String, DecodedFrame>,
}

impl Default for RealisticCompanionSheet {
    fn default() -> Self {
        Self::new()
    }
}

impl RealisticCompanionSheet {
    /// Initializes and decodes all embedded photographic assets for Oreo.
    pub fn new() -> Self {
        let mut frames = HashMap::new();

        frames.insert("sit_0".to_string(), DecodedFrame::from_png_bytes(SIT_0_PNG));
        frames.insert(
            "idle_0".to_string(),
            DecodedFrame::from_png_bytes(IDLE_0_PNG),
        );
        frames.insert(
            "high_five_0".to_string(),
            DecodedFrame::from_png_bytes(HIGH_FIVE_0_PNG),
        );
        frames.insert(
            "sleep_0".to_string(),
            DecodedFrame::from_png_bytes(SLEEP_0_PNG),
        );
        frames.insert(
            "stretch_0".to_string(),
            DecodedFrame::from_png_bytes(STRETCH_0_PNG),
        );
        frames.insert(
            "curious_0".to_string(),
            DecodedFrame::from_png_bytes(CURIOUS_0_PNG),
        );
        frames.insert("ask_0".to_string(), DecodedFrame::from_png_bytes(ASK_0_PNG));

        Self { frames }
    }

    /// Resolves an animation frame by key or semantic behavior name, falling back to "sit_0".
    pub fn get_frame(&self, name: &str) -> &DecodedFrame {
        if let Some(frame) = self.frames.get(name) {
            return frame;
        }

        let key = match name {
            s if s.starts_with("sleep") || s.contains("loaf") => "sleep_0",
            s if s.starts_with("stretch") => "stretch_0",
            s if s.starts_with("curious") => "curious_0",
            s if s.starts_with("ask") || s.starts_with("chat") => "ask_0",
            s if s.starts_with("high_five") => "high_five_0",
            s if s.starts_with("idle") => "idle_0",
            _ => "sit_0",
        };

        self.frames
            .get(key)
            .or_else(|| self.frames.get("sit_0"))
            .expect("sit_0 frame must always exist")
    }

    /// Renders a scaled 32-bit BGRA buffer suitable for Windows GDI rendering.
    ///
    /// The image is aspect-ratio preserved and bottom-aligned / horizontally centered
    /// within `target_w` x `target_h`. Transparent pixels are filled or blended with
    /// `chroma_key` if specified.
    #[allow(clippy::too_many_lines)]
    pub fn render_frame_bgra(
        &self,
        frame: &str,
        target_w: usize,
        target_h: usize,
        chroma_key: Option<u32>,
    ) -> (Vec<u8>, usize, usize) {
        if target_w == 0 || target_h == 0 {
            return (Vec::new(), target_w, target_h);
        }

        let src = self.get_frame(frame);
        let src_w = src.width as usize;
        let src_h = src.height as usize;
        if src_w == 0 || src_h == 0 {
            return (vec![0u8; target_w * target_h * 4], target_w, target_h);
        }

        // Calculate aspect-ratio fit
        let scale = (target_w as f64 / src_w as f64).min(target_h as f64 / src_h as f64);
        let draw_w = ((src_w as f64 * scale).round() as usize).clamp(1, target_w);
        let draw_h = ((src_h as f64 * scale).round() as usize).clamp(1, target_h);

        let offset_x = (target_w - draw_w) / 2;
        let offset_y = target_h - draw_h; // Bottom-aligned for cat on surface

        let (chroma_b, chroma_g, chroma_r) = match chroma_key {
            Some(rgb) => (
                (rgb & 0xFF) as u8,
                ((rgb >> 8) & 0xFF) as u8,
                ((rgb >> 16) & 0xFF) as u8,
            ),
            None => (0, 0, 0),
        };

        let mut out = vec![0u8; target_w * target_h * 4];

        // Fill background with chroma key if supplied
        if chroma_key.is_some() {
            for chunk in out.chunks_exact_mut(4) {
                chunk[0] = chroma_b;
                chunk[1] = chroma_g;
                chunk[2] = chroma_r;
                chunk[3] = 255;
            }
        }

        let is_gdi_colorkey = chroma_key == Some(0x00FF00FF);

        for dy in 0..draw_h {
            let gy = (dy as f64 + 0.5) * (src_h as f64 / draw_h as f64) - 0.5;
            let gyi = gy.floor() as isize;
            let fy = (gy - gy.floor()) as f32;

            for dx in 0..draw_w {
                let gx = (dx as f64 + 0.5) * (src_w as f64 / draw_w as f64) - 0.5;
                let gxi = gx.floor() as isize;
                let fx = (gx - gx.floor()) as f32;

                // Bilinear sample 4 neighbors
                let sample_px = |x: isize, y: isize| -> [u8; 4] {
                    let cx = x.clamp(0, src_w as isize - 1) as usize;
                    let cy = y.clamp(0, src_h as isize - 1) as usize;
                    let idx = (cy * src_w + cx) * 4;
                    [
                        src.rgba[idx],
                        src.rgba[idx + 1],
                        src.rgba[idx + 2],
                        src.rgba[idx + 3],
                    ]
                };

                let p00 = sample_px(gxi, gyi);
                let p10 = sample_px(gxi + 1, gyi);
                let p01 = sample_px(gxi, gyi + 1);
                let p11 = sample_px(gxi + 1, gyi + 1);

                let w00 = (1.0 - fx) * (1.0 - fy);
                let w10 = fx * (1.0 - fy);
                let w01 = (1.0 - fx) * fy;
                let w11 = fx * fy;

                let r = (p00[0] as f32 * w00
                    + p10[0] as f32 * w10
                    + p01[0] as f32 * w01
                    + p11[0] as f32 * w11)
                    .round() as u8;
                let g = (p00[1] as f32 * w00
                    + p10[1] as f32 * w10
                    + p01[1] as f32 * w01
                    + p11[1] as f32 * w11)
                    .round() as u8;
                let b = (p00[2] as f32 * w00
                    + p10[2] as f32 * w10
                    + p01[2] as f32 * w01
                    + p11[2] as f32 * w11)
                    .round() as u8;
                let a = (p00[3] as f32 * w00
                    + p10[3] as f32 * w10
                    + p01[3] as f32 * w01
                    + p11[3] as f32 * w11)
                    .round() as u8;

                let out_x = offset_x + dx;
                let out_y = offset_y + dy;
                let out_idx = (out_y * target_w + out_x) * 4;

                if is_gdi_colorkey {
                    if a >= 128 {
                        // Prevent accidental chroma key keying inside sprite fur
                        let safe_b = if r == 255 && g == 0 && b == 255 {
                            254
                        } else {
                            b
                        };
                        out[out_idx] = safe_b;
                        out[out_idx + 1] = g;
                        out[out_idx + 2] = r;
                        out[out_idx + 3] = 255;
                    }
                } else if chroma_key.is_some() {
                    if a > 0 {
                        let alpha_factor = a as u32;
                        let inv_alpha = 255 - alpha_factor;
                        out[out_idx] =
                            ((b as u32 * alpha_factor + chroma_b as u32 * inv_alpha) / 255) as u8;
                        out[out_idx + 1] =
                            ((g as u32 * alpha_factor + chroma_g as u32 * inv_alpha) / 255) as u8;
                        out[out_idx + 2] =
                            ((r as u32 * alpha_factor + chroma_r as u32 * inv_alpha) / 255) as u8;
                        out[out_idx + 3] = 255;
                    }
                } else {
                    out[out_idx] = b;
                    out[out_idx + 1] = g;
                    out[out_idx + 2] = r;
                    out[out_idx + 3] = a;
                }
            }
        }

        (out, target_w, target_h)
    }

    /// Determines whether the pixel at `(x, y)` in the scaled `target_w` x `target_h` box is opaque.
    /// Used by `WM_NCHITTEST` to pass through transparent clicks (`HTTRANSPARENT`).
    pub fn is_pixel_opaque(
        &self,
        frame: &str,
        target_w: usize,
        target_h: usize,
        x: usize,
        y: usize,
    ) -> bool {
        if x >= target_w || y >= target_h {
            return false;
        }

        let src = self.get_frame(frame);
        let src_w = src.width as usize;
        let src_h = src.height as usize;
        if src_w == 0 || src_h == 0 {
            return false;
        }

        let scale = (target_w as f64 / src_w as f64).min(target_h as f64 / src_h as f64);
        let draw_w = ((src_w as f64 * scale).round() as usize).clamp(1, target_w);
        let draw_h = ((src_h as f64 * scale).round() as usize).clamp(1, target_h);

        let offset_x = (target_w - draw_w) / 2;
        let offset_y = target_h - draw_h;

        if x < offset_x || x >= offset_x + draw_w || y < offset_y || y >= offset_y + draw_h {
            return false;
        }

        let dx = x - offset_x;
        let dy = y - offset_y;
        let sx = (dx * src_w / draw_w).min(src_w - 1);
        let sy = (dy * src_h / draw_h).min(src_h - 1);

        let idx = (sy * src_w + sx) * 4;
        src.rgba[idx + 3] >= 128
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_realistic_sheet_loading_and_dimensions() {
        let sheet = RealisticCompanionSheet::new();
        let expected = [
            "sit_0",
            "idle_0",
            "high_five_0",
            "sleep_0",
            "stretch_0",
            "curious_0",
            "ask_0",
        ];

        for name in &expected {
            let frame = sheet.get_frame(name);
            assert!(frame.width > 0, "Frame {} width must be > 0", name);
            assert!(frame.height > 0, "Frame {} height must be > 0", name);
            assert_eq!(
                frame.rgba.len(),
                (frame.width * frame.height * 4) as usize,
                "Frame {} byte size mismatch",
                name
            );

            // Verify non-empty alpha content
            let has_opaque = frame.rgba.chunks_exact(4).any(|px| px[3] > 0);
            assert!(
                has_opaque,
                "Frame {} must contain non-transparent pixels",
                name
            );
        }
    }

    #[test]
    fn test_render_frame_bgra_scaled() {
        let sheet = RealisticCompanionSheet::new();
        let (bgra, w, h) = sheet.render_frame_bgra("sit_0", 128, 128, Some(0x00FF00FF));
        assert_eq!(w, 128);
        assert_eq!(h, 128);
        assert_eq!(bgra.len(), 128 * 128 * 4);

        // Top-left corner outside the cat body must be the chroma key (Magenta in BGRA = B:255, G:0, R:255)
        let (b, g, r) = (bgra[0], bgra[1], bgra[2]);
        assert_eq!(b, 255);
        assert_eq!(g, 0);
        assert_eq!(r, 255);
    }

    #[test]
    fn test_is_pixel_opaque_hit_testing() {
        let sheet = RealisticCompanionSheet::new();
        // Corner (0, 0) is transparent
        assert!(!sheet.is_pixel_opaque("sit_0", 128, 128, 0, 0));

        // Center bottom area where Oreo sits should be opaque
        let is_center_opaque = sheet.is_pixel_opaque("sit_0", 128, 128, 64, 100);
        assert!(
            is_center_opaque,
            "Cat center-bottom area must be opaque for hit testing"
        );
    }

    #[test]
    fn test_zero_dimensions_graceful() {
        let sheet = RealisticCompanionSheet::new();
        let (buf, w, h) = sheet.render_frame_bgra("sit_0", 0, 0, None);
        assert_eq!(w, 0);
        assert_eq!(h, 0);
        assert!(buf.is_empty());
        assert!(!sheet.is_pixel_opaque("sit_0", 0, 0, 0, 0));
    }
}
