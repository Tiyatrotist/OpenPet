//! # Mimi the Cat Sprite Generator & Renderer
//!
//! Procedural pixel-art generation for the official starter companion Mimi the Cat.
//! Produces 64x64 frames across all behaviors (idle, walk, sit, sleep, play, pet, drag)
//! with transparent backgrounds and scaling support for Windows desktop rendering.

use std::collections::HashMap;

/// Canvas size for each individual animation frame
pub const FRAME_WIDTH: usize = 64;
pub const FRAME_HEIGHT: usize = 64;
pub const FRAME_PIXELS: usize = FRAME_WIDTH * FRAME_HEIGHT;
pub const FRAME_BYTES: usize = FRAME_PIXELS * 4;

/// Atlas sheet dimensions
pub const ATLAS_WIDTH: usize = 256;
pub const ATLAS_HEIGHT: usize = 512;

/// Color palette for Mimi the Cat
pub mod palette {
    pub const TRANSPARENT: [u8; 4] = [0, 0, 0, 0];
    pub const ORANGE_FUR: [u8; 4] = [245, 140, 50, 255];
    pub const DARK_ORANGE: [u8; 4] = [200, 100, 30, 255];
    pub const CREAM_FUR: [u8; 4] = [255, 235, 205, 255];
    pub const WHITE_FUR: [u8; 4] = [255, 255, 255, 255];
    pub const PINK_EAR: [u8; 4] = [255, 175, 190, 255];
    pub const PINK_NOSE: [u8; 4] = [255, 105, 140, 255];
    pub const EYE_GREEN: [u8; 4] = [35, 185, 120, 255];
    pub const EYE_PUPIL: [u8; 4] = [20, 25, 35, 255];
    pub const EYE_GLINT: [u8; 4] = [255, 255, 255, 255];
    pub const OUTLINE: [u8; 4] = [45, 30, 25, 255];
    pub const HEART_RED: [u8; 4] = [255, 45, 95, 255];
    pub const ZZZ_BLUE: [u8; 4] = [80, 190, 255, 255];
    pub const STAR_GOLD: [u8; 4] = [255, 215, 0, 255];
    pub const FISH_BLUE: [u8; 4] = [70, 160, 240, 255];
    pub const FISH_LIGHT: [u8; 4] = [130, 200, 255, 255];
}

/// A 64x64 RGBA pixel buffer representing one animation frame.
#[derive(Clone)]
pub struct FrameBuffer {
    pub pixels: [u8; FRAME_BYTES],
}

impl Default for FrameBuffer {
    fn default() -> Self {
        Self {
            pixels: [0u8; FRAME_BYTES],
        }
    }
}

impl FrameBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn set_pixel(&mut self, x: usize, y: usize, color: [u8; 4]) {
        if x < FRAME_WIDTH && y < FRAME_HEIGHT {
            let idx = (y * FRAME_WIDTH + x) * 4;
            self.pixels[idx] = color[0];
            self.pixels[idx + 1] = color[1];
            self.pixels[idx + 2] = color[2];
            self.pixels[idx + 3] = color[3];
        }
    }

    #[inline]
    pub fn get_pixel(&self, x: usize, y: usize) -> [u8; 4] {
        if x < FRAME_WIDTH && y < FRAME_HEIGHT {
            let idx = (y * FRAME_WIDTH + x) * 4;
            [
                self.pixels[idx],
                self.pixels[idx + 1],
                self.pixels[idx + 2],
                self.pixels[idx + 3],
            ]
        } else {
            palette::TRANSPARENT
        }
    }

    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: [u8; 4]) {
        for dy in 0..h {
            for dx in 0..w {
                self.set_pixel(x + dx, y + dy, color);
            }
        }
    }

    pub fn draw_ellipse(
        &mut self,
        cx: isize,
        cy: isize,
        rx: isize,
        ry: isize,
        color: [u8; 4],
        outline: Option<[u8; 4]>,
    ) {
        if rx <= 0 || ry <= 0 {
            return;
        }
        let rx2 = rx * rx;
        let ry2 = ry * ry;
        for dy in -ry..=ry {
            for dx in -rx..=rx {
                let d = dx * dx * ry2 + dy * dy * rx2;
                let px = cx + dx;
                let py = cy + dy;
                if px >= 0
                    && px < FRAME_WIDTH as isize
                    && py >= 0
                    && py < FRAME_HEIGHT as isize
                    && d <= rx2 * ry2
                {
                    if let Some(out_color) = outline {
                        let border_d = (dx.abs() * 10 / rx) * (dx.abs() * 10 / rx)
                            + (dy.abs() * 10 / ry) * (dy.abs() * 10 / ry);
                        if border_d >= 75 {
                            self.set_pixel(px as usize, py as usize, out_color);
                            continue;
                        }
                    }
                    self.set_pixel(px as usize, py as usize, color);
                }
            }
        }
    }

    pub fn draw_heart(&mut self, cx: usize, cy: usize, size: usize) {
        if size == 0 {
            return;
        }
        let s = size as isize;
        for dy in -s..=s {
            for dx in -s..=s {
                let x = dx as f32 / s as f32;
                let y = -dy as f32 / s as f32; // Invert y
                                               // Standard algebraic heart curve: (x^2 + y^2 - 1)^3 - x^2 * y^3 <= 0
                let a = x * x + y * y - 0.7;
                if a * a * a - x * x * y * y * y <= 0.0 {
                    let px = cx as isize + dx;
                    let py = cy as isize + dy;
                    if px >= 0 && px < FRAME_WIDTH as isize && py >= 0 && py < FRAME_HEIGHT as isize
                    {
                        self.set_pixel(px as usize, py as usize, palette::HEART_RED);
                    }
                }
            }
        }
    }

    pub fn draw_zzz(&mut self, cx: usize, cy: usize, size: usize) {
        if size == 0 {
            return;
        }
        let s = size;
        let color = palette::ZZZ_BLUE;
        for i in 0..s {
            self.set_pixel(cx + i, cy, color);
            self.set_pixel(cx + s - 1 - i, cy + i, color);
            self.set_pixel(cx + i, cy + s - 1, color);
        }
    }

    pub fn draw_sparkle(&mut self, cx: usize, cy: usize) {
        let color = palette::STAR_GOLD;
        self.set_pixel(cx, cy, color);
        self.set_pixel(cx.saturating_add(1), cy, color);
        if cx > 0 {
            self.set_pixel(cx - 1, cy, color);
        }
        self.set_pixel(cx, cy.saturating_add(1), color);
        if cy > 0 {
            self.set_pixel(cx, cy - 1, color);
        }
        self.set_pixel(cx.saturating_add(2), cy, color);
        if cx > 1 {
            self.set_pixel(cx - 2, cy, color);
        }
        self.set_pixel(cx, cy.saturating_add(2), color);
        if cy > 1 {
            self.set_pixel(cx, cy - 2, color);
        }
    }

    pub fn draw_fish(&mut self, cx: usize, cy: usize, size: usize) {
        if size == 0 {
            return;
        }
        let s = size as isize;
        // Body: horizontal ellipse
        self.draw_ellipse(
            cx as isize,
            cy as isize,
            s,
            (s / 2).max(2),
            palette::FISH_BLUE,
            Some(palette::OUTLINE),
        );
        self.draw_ellipse(
            cx as isize,
            cy as isize,
            (s - 2).max(1),
            ((s / 2).max(2) - 1).max(1),
            palette::FISH_LIGHT,
            None,
        );

        // Tail: triangle pointing away (left side)
        let tail_x = (cx as isize) - s - 1;
        for dy in -(s / 2)..=(s / 2) {
            let len = ((s / 2) - dy.abs()).max(1);
            for dx in 0..=len {
                let px = tail_x - dx;
                let py = (cy as isize) + dy;
                if px >= 0 && px < FRAME_WIDTH as isize && py >= 0 && py < FRAME_HEIGHT as isize {
                    self.set_pixel(px as usize, py as usize, palette::FISH_BLUE);
                }
            }
        }
        // Little fish eye (pupil with glint)
        let eye_x = (cx as isize) + s / 2;
        let eye_y = (cy as isize) - 1;
        if eye_x >= 0 && eye_x < FRAME_WIDTH as isize && eye_y >= 0 && eye_y < FRAME_HEIGHT as isize
        {
            self.set_pixel(eye_x as usize, eye_y as usize, palette::EYE_PUPIL);
            if eye_y > 0 {
                self.set_pixel(eye_x as usize, (eye_y - 1) as usize, palette::EYE_GLINT);
            }
        }
    }

    pub fn draw_crumbs(&mut self, cx: usize, cy: usize) {
        let color = palette::DARK_ORANGE;
        self.set_pixel(cx, cy, color);
        self.set_pixel(cx.saturating_add(3), cy.saturating_add(2), color);
        self.set_pixel(cx.saturating_sub(2), cy.saturating_add(3), color);
        self.set_pixel(cx.saturating_add(2), cy.saturating_sub(2), color);
        self.set_pixel(cx.saturating_sub(3), cy.saturating_sub(1), color);
    }

    pub fn draw_exclamation(&mut self, cx: usize, cy: usize) {
        let color = palette::HEART_RED;
        for dy in 0..6 {
            self.set_pixel(cx, cy.saturating_add(dy), color);
            self.set_pixel(cx.saturating_add(1), cy.saturating_add(dy), color);
        }
        self.set_pixel(cx, cy.saturating_add(8), color);
        self.set_pixel(cx.saturating_add(1), cy.saturating_add(8), color);
    }
}

/// Cat breed color palettes supporting realistic feline coats and patterns.
#[derive(Debug, Clone, Copy)]
pub struct CatColorPalette {
    pub body_fur: [u8; 4],
    pub secondary_fur: [u8; 4],
    pub chest_fur: [u8; 4],
    pub eye_iris: [u8; 4],
    pub inner_ear: [u8; 4],
    pub nose: [u8; 4],
}

impl CatColorPalette {
    pub fn for_breed(breed: openpet_types::CatBreed) -> Self {
        match breed {
            openpet_types::CatBreed::Tabby => Self {
                body_fur: [165, 150, 135, 255],   // Warm grey-brown coat
                secondary_fur: [80, 70, 60, 255], // Dark tabby stripes
                chest_fur: [240, 235, 225, 255],  // Cream bib
                eye_iris: [45, 180, 110, 255],    // Greenish hazel
                inner_ear: [255, 180, 195, 255],
                nose: [240, 120, 140, 255],
            },
            openpet_types::CatBreed::Tuxedo => Self {
                body_fur: [35, 35, 42, 255],      // Midnight black
                secondary_fur: [22, 22, 28, 255], // Deep charcoal
                chest_fur: [255, 255, 255, 255],  // Pure white bib
                eye_iris: [35, 200, 120, 255],    // Emerald green
                inner_ear: [255, 175, 190, 255],
                nose: [255, 105, 140, 255],
            },
            openpet_types::CatBreed::Calico => Self {
                body_fur: [255, 250, 245, 255],     // Cream white base
                secondary_fur: [235, 125, 45, 255], // Ginger patches
                chest_fur: [45, 45, 52, 255],       // Charcoal black patch
                eye_iris: [235, 175, 45, 255],      // Golden amber
                inner_ear: [255, 180, 190, 255],
                nose: [255, 110, 145, 255],
            },
            openpet_types::CatBreed::Ginger => Self {
                body_fur: [245, 140, 50, 255],      // Marmalade orange
                secondary_fur: [200, 100, 30, 255], // Dark orange stripes
                chest_fur: [255, 235, 205, 255],    // Warm cream
                eye_iris: [220, 160, 40, 255],      // Honey amber
                inner_ear: [255, 175, 190, 255],
                nose: [255, 105, 140, 255],
            },
            openpet_types::CatBreed::Siamese => Self {
                body_fur: [250, 240, 225, 255],   // Pale cream body
                secondary_fur: [70, 50, 45, 255], // Dark seal points
                chest_fur: [255, 248, 238, 255],
                eye_iris: [45, 130, 245, 255], // Deep sapphire blue
                inner_ear: [120, 85, 75, 255],
                nose: [75, 55, 50, 255],
            },
            openpet_types::CatBreed::Black => Self {
                body_fur: [30, 30, 35, 255], // Silky midnight black
                secondary_fur: [20, 20, 25, 255],
                chest_fur: [42, 42, 50, 255],
                eye_iris: [255, 215, 0, 255], // Glowing amber gold
                inner_ear: [70, 70, 80, 255],
                nose: [40, 40, 45, 255],
            },
            openpet_types::CatBreed::White => Self {
                body_fur: [255, 255, 255, 255], // Pure snow white
                secondary_fur: [240, 240, 245, 255],
                chest_fur: [255, 255, 255, 255],
                eye_iris: [70, 175, 255, 255], // Sky blue
                inner_ear: [255, 190, 205, 255],
                nose: [255, 130, 160, 255],
            },
        }
    }
}

/// Mimi Cat Sprite Sheet containing all animation frames.
pub struct MimiSpriteSheet {
    pub frames: HashMap<String, FrameBuffer>,
}

impl MimiSpriteSheet {
    /// Generates base procedural pixel-art frames for Mimi the Cat.
    pub fn generate_base() -> Self {
        let mut frames = HashMap::new();

        frames.insert("idle_0".to_string(), render_mimi_idle(0));
        frames.insert("idle_1".to_string(), render_mimi_idle(1));
        frames.insert("walk_0".to_string(), render_mimi_walk(0));
        frames.insert("walk_1".to_string(), render_mimi_walk(1));
        frames.insert("walk_2".to_string(), render_mimi_walk(2));
        frames.insert("sit_0".to_string(), render_mimi_sit());
        frames.insert("sleep_0".to_string(), render_mimi_sleep(0));
        frames.insert("sleep_1".to_string(), render_mimi_sleep(1));
        frames.insert("play_0".to_string(), render_mimi_play(0));
        frames.insert("play_1".to_string(), render_mimi_play(1));
        frames.insert("drag_0".to_string(), render_mimi_drag());
        frames.insert("pet_0".to_string(), render_mimi_pet());

        // Enriched animations
        frames.insert("eat_0".to_string(), render_mimi_eat(0));
        frames.insert("eat_1".to_string(), render_mimi_eat(1));
        frames.insert("stretch_0".to_string(), render_mimi_stretch(0));
        frames.insert("stretch_1".to_string(), render_mimi_stretch(1));
        frames.insert("curious_0".to_string(), render_mimi_curious(0));
        frames.insert("curious_1".to_string(), render_mimi_curious(1));
        frames.insert("jump_0".to_string(), render_mimi_jump(0));
        frames.insert("jump_1".to_string(), render_mimi_jump(1));
        frames.insert("groom_0".to_string(), render_mimi_groom(0));
        frames.insert("groom_1".to_string(), render_mimi_groom(1));
        frames.insert("surprised_0".to_string(), render_mimi_surprised());

        // Real Cat behavior frames
        frames.insert("purr_0".to_string(), render_mimi_purr(0));
        frames.insert("purr_1".to_string(), render_mimi_purr(1));
        frames.insert("knead_0".to_string(), render_mimi_knead(0));
        frames.insert("knead_1".to_string(), render_mimi_knead(1));
        frames.insert("zoomies_0".to_string(), render_mimi_zoomies(0));
        frames.insert("zoomies_1".to_string(), render_mimi_zoomies(1));
        frames.insert("loaf_0".to_string(), render_mimi_loaf());
        frames.insert("drink_0".to_string(), render_mimi_drink(0));
        frames.insert("drink_1".to_string(), render_mimi_drink(1));

        Self { frames }
    }

    /// Recolors all frames in place to match a specific cat breed palette.
    #[allow(clippy::chunks_exact_to_as_chunks)]
    pub fn recolor_for_breed(&mut self, breed: openpet_types::CatBreed) {
        if breed == openpet_types::CatBreed::Ginger {
            return; // Ginger is base palette
        }
        let pal = CatColorPalette::for_breed(breed);
        for frame in self.frames.values_mut() {
            for chunk in frame.pixels.chunks_exact_mut(4) {
                if chunk[3] == 0 {
                    continue;
                }
                let px = [chunk[0], chunk[1], chunk[2], chunk[3]];
                if px == palette::ORANGE_FUR {
                    chunk.copy_from_slice(&pal.body_fur);
                } else if px == palette::DARK_ORANGE {
                    chunk.copy_from_slice(&pal.secondary_fur);
                } else if px == palette::CREAM_FUR {
                    chunk.copy_from_slice(&pal.chest_fur);
                } else if px == palette::EYE_GREEN {
                    chunk.copy_from_slice(&pal.eye_iris);
                } else if px == palette::PINK_EAR {
                    chunk.copy_from_slice(&pal.inner_ear);
                } else if px == palette::PINK_NOSE {
                    chunk.copy_from_slice(&pal.nose);
                }
            }
        }
    }

    /// Generates sprite sheet with colors customized for a specific cat breed.
    pub fn generate_for_breed(breed: openpet_types::CatBreed) -> Self {
        let mut sheet = Self::generate_base();
        sheet.recolor_for_breed(breed);
        sheet
    }

    /// Generates all procedural pixel-art frames for Mimi the Cat (classic Ginger / default).
    pub fn generate() -> Self {
        Self::generate_for_breed(openpet_types::CatBreed::Ginger)
    }

    /// Retrieves an animation frame by name, falling back to "idle_0".
    pub fn get_frame(&self, name: &str) -> &FrameBuffer {
        self.frames
            .get(name)
            .or_else(|| self.frames.get("idle_0"))
            .expect("idle_0 frame must always exist")
    }

    /// Converts a 64x64 frame into a scaled 32-bit BGRA buffer suitable for Windows GDI rendering.
    /// If `chroma_key_rgb` is provided (e.g. 0x00FF00FF for Magenta), transparent pixels are painted
    /// with that color for `SetLayeredWindowAttributes(hwnd, colorkey, 0, LWA_COLORKEY)`.
    pub fn render_frame_bgra_scaled(
        &self,
        name: &str,
        scale: usize,
        chroma_key_rgb: Option<u32>,
    ) -> (Vec<u8>, usize, usize) {
        let frame = self.get_frame(name);
        let out_w = FRAME_WIDTH * scale;
        let out_h = FRAME_HEIGHT * scale;
        let mut out = vec![0u8; out_w * out_h * 4];

        let (chroma_b, chroma_g, chroma_r) = match chroma_key_rgb {
            Some(rgb) => (
                (rgb & 0xFF) as u8,
                ((rgb >> 8) & 0xFF) as u8,
                ((rgb >> 16) & 0xFF) as u8,
            ),
            None => (0, 0, 0),
        };

        for y in 0..FRAME_HEIGHT {
            for x in 0..FRAME_WIDTH {
                let px = frame.get_pixel(x, y);
                let (b, g, r, a) = if px[3] > 0 {
                    (px[2], px[1], px[0], px[3])
                } else if chroma_key_rgb.is_some() {
                    (chroma_b, chroma_g, chroma_r, 255)
                } else {
                    (0, 0, 0, 0)
                };

                // Nearest neighbor scaling
                for sy in 0..scale {
                    for sx in 0..scale {
                        let out_x = x * scale + sx;
                        let out_y = y * scale + sy;
                        let out_idx = (out_y * out_w + out_x) * 4;
                        out[out_idx] = b;
                        out[out_idx + 1] = g;
                        out[out_idx + 2] = r;
                        out[out_idx + 3] = a;
                    }
                }
            }
        }

        (out, out_w, out_h)
    }
}

// ---------------------------------------------------------------------------
// Procedural Frame Artists
// ---------------------------------------------------------------------------

fn draw_cat_ears(fb: &mut FrameBuffer, cy: isize) {
    // Left ear
    for y in 0..12 {
        let w = (12 - y) * 8 / 12;
        for x in 0..w {
            let px = 20 - x;
            let py = (cy - 12 + y as isize) as usize;
            fb.set_pixel(px, py, palette::ORANGE_FUR);
            if x < w / 2 && y > 3 {
                fb.set_pixel(px, py, palette::PINK_EAR);
            }
        }
    }
    // Right ear
    for y in 0..12 {
        let w = (12 - y) * 8 / 12;
        for x in 0..w {
            let px = 43 + x;
            let py = (cy - 12 + y as isize) as usize;
            fb.set_pixel(px, py, palette::ORANGE_FUR);
            if x < w / 2 && y > 3 {
                fb.set_pixel(px, py, palette::PINK_EAR);
            }
        }
    }
}

fn draw_cat_face(fb: &mut FrameBuffer, cx: isize, cy: isize, eyes_open: bool, is_happy: bool) {
    // Head shape
    fb.draw_ellipse(cx, cy, 15, 12, palette::CREAM_FUR, Some(palette::OUTLINE));

    // Orange forehead patch
    fb.draw_ellipse(cx, cy - 6, 9, 5, palette::ORANGE_FUR, None);

    // Eyes
    if eyes_open {
        // Left eye
        fb.draw_ellipse(cx - 6, cy, 3, 4, palette::EYE_GREEN, Some(palette::OUTLINE));
        fb.set_pixel((cx - 6) as usize, cy as usize, palette::EYE_PUPIL);
        fb.set_pixel((cx - 7) as usize, (cy - 1) as usize, palette::EYE_GLINT);

        // Right eye
        fb.draw_ellipse(cx + 6, cy, 3, 4, palette::EYE_GREEN, Some(palette::OUTLINE));
        fb.set_pixel((cx + 6) as usize, cy as usize, palette::EYE_PUPIL);
        fb.set_pixel((cx + 5) as usize, (cy - 1) as usize, palette::EYE_GLINT);
    } else if is_happy {
        // Happy closed smiling eyes ^ ^
        let lx = (cx - 7) as usize;
        let rx = (cx + 5) as usize;
        let ey = cy as usize;
        fb.set_pixel(lx, ey, palette::OUTLINE);
        fb.set_pixel(lx + 1, ey - 1, palette::OUTLINE);
        fb.set_pixel(lx + 2, ey - 1, palette::OUTLINE);
        fb.set_pixel(lx + 3, ey, palette::OUTLINE);

        fb.set_pixel(rx, ey, palette::OUTLINE);
        fb.set_pixel(rx + 1, ey - 1, palette::OUTLINE);
        fb.set_pixel(rx + 2, ey - 1, palette::OUTLINE);
        fb.set_pixel(rx + 3, ey, palette::OUTLINE);
    } else {
        // Sleepy line eyes - -
        for i in 0..4 {
            fb.set_pixel((cx - 8) as usize + i, cy as usize, palette::OUTLINE);
            fb.set_pixel((cx + 5) as usize + i, cy as usize, palette::OUTLINE);
        }
    }

    // Pink nose
    fb.set_pixel(cx as usize, (cy + 3) as usize, palette::PINK_NOSE);
    fb.set_pixel((cx - 1) as usize, (cy + 3) as usize, palette::PINK_NOSE);

    // Mouth :3
    fb.set_pixel((cx - 2) as usize, (cy + 5) as usize, palette::OUTLINE);
    fb.set_pixel((cx - 1) as usize, (cy + 6) as usize, palette::OUTLINE);
    fb.set_pixel(cx as usize, (cy + 5) as usize, palette::OUTLINE);
    fb.set_pixel((cx + 1) as usize, (cy + 6) as usize, palette::OUTLINE);
    fb.set_pixel((cx + 2) as usize, (cy + 5) as usize, palette::OUTLINE);

    // Whiskers
    for i in 0..4 {
        fb.set_pixel((cx - 14) as usize + i, (cy + 3) as usize, palette::OUTLINE);
        fb.set_pixel((cx - 13) as usize + i, (cy + 5) as usize, palette::OUTLINE);
        fb.set_pixel((cx + 11) as usize + i, (cy + 3) as usize, palette::OUTLINE);
        fb.set_pixel((cx + 10) as usize + i, (cy + 5) as usize, palette::OUTLINE);
    }
}

fn draw_cat_body(fb: &mut FrameBuffer, cx: isize, cy: isize) {
    // Main body
    fb.draw_ellipse(cx, cy, 13, 14, palette::ORANGE_FUR, Some(palette::OUTLINE));

    // White chest patch
    fb.draw_ellipse(cx, cy - 2, 7, 9, palette::WHITE_FUR, None);
}

fn draw_cat_tail(fb: &mut FrameBuffer, start_x: usize, start_y: usize, curl_dir: isize) {
    for i in 0..12 {
        let tx = (start_x as isize + curl_dir * (i as isize * 10 / 12)) as usize;
        let ty = (start_y as isize - (i as isize * 8 / 12)) as usize;
        let color = if i > 8 {
            palette::WHITE_FUR
        } else {
            palette::ORANGE_FUR
        };
        fb.set_pixel(tx, ty, color);
        fb.set_pixel(tx + 1, ty, color);
    }
}

fn render_mimi_idle(frame_idx: usize) -> FrameBuffer {
    let mut fb = FrameBuffer::new();
    let cy_head: isize = if frame_idx == 0 { 24 } else { 25 };
    let cy_body: isize = if frame_idx == 0 { 42 } else { 43 };

    draw_cat_tail(&mut fb, 22, 48, -1);
    draw_cat_ears(&mut fb, cy_head);
    draw_cat_body(&mut fb, 32, cy_body);
    draw_cat_face(&mut fb, 32, cy_head, frame_idx == 0, frame_idx == 1);

    // Front paws
    fb.draw_ellipse(26, 54, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
    fb.draw_ellipse(38, 54, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));

    fb
}

fn render_mimi_walk(step: usize) -> FrameBuffer {
    let mut fb = FrameBuffer::new();
    let cy_head: isize = 24;
    let cy_body: isize = 42;

    let tail_dir = match step {
        0 => -1,
        1 => 0,
        _ => 1,
    };
    draw_cat_tail(&mut fb, 22, 48, tail_dir);
    draw_cat_ears(&mut fb, cy_head);
    draw_cat_body(&mut fb, 32, cy_body);
    draw_cat_face(&mut fb, 32, cy_head, true, false);

    // Step paws
    match step {
        0 => {
            fb.draw_ellipse(24, 56, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
            fb.draw_ellipse(40, 52, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
        }
        1 => {
            fb.draw_ellipse(27, 54, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
            fb.draw_ellipse(37, 54, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
        }
        _ => {
            fb.draw_ellipse(24, 52, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
            fb.draw_ellipse(40, 56, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
        }
    }

    fb
}

fn render_mimi_sit() -> FrameBuffer {
    let mut fb = FrameBuffer::new();
    let cy_head: isize = 22;
    let cy_body: isize = 41;

    draw_cat_tail(&mut fb, 20, 50, -1);
    draw_cat_ears(&mut fb, cy_head);
    draw_cat_body(&mut fb, 32, cy_body);
    draw_cat_face(&mut fb, 32, cy_head, true, false);

    // Neatly tucked paws
    fb.draw_ellipse(29, 53, 3, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
    fb.draw_ellipse(35, 53, 3, 3, palette::WHITE_FUR, Some(palette::OUTLINE));

    fb
}

fn render_mimi_sleep(frame_idx: usize) -> FrameBuffer {
    let mut fb = FrameBuffer::new();
    let rad: isize = if frame_idx == 0 { 16 } else { 17 };

    // Curled ball
    fb.draw_ellipse(
        32,
        38,
        rad,
        rad - 2,
        palette::ORANGE_FUR,
        Some(palette::OUTLINE),
    );
    fb.draw_ellipse(32, 40, rad - 6, rad - 7, palette::WHITE_FUR, None);

    // Curled ears
    fb.draw_ellipse(22, 28, 4, 4, palette::PINK_EAR, Some(palette::OUTLINE));
    fb.draw_ellipse(42, 28, 4, 4, palette::PINK_EAR, Some(palette::OUTLINE));

    // Closed sleepy eyes - -
    for i in 0..4 {
        fb.set_pixel(26 + i, 36, palette::OUTLINE);
        fb.set_pixel(34 + i, 36, palette::OUTLINE);
    }

    // Tiny pink nose
    fb.set_pixel(32, 38, palette::PINK_NOSE);

    // Floating Zzz
    if frame_idx == 0 {
        fb.draw_zzz(46, 16, 5);
    } else {
        fb.draw_zzz(46, 12, 6);
        fb.draw_zzz(53, 6, 4);
    }

    fb
}

fn render_mimi_play(frame_idx: usize) -> FrameBuffer {
    let mut fb = FrameBuffer::new();
    if frame_idx == 0 {
        // Crouching to pounce
        let cy_head: isize = 28;
        let cy_body: isize = 44;
        draw_cat_tail(&mut fb, 22, 44, -1);
        draw_cat_ears(&mut fb, cy_head);
        draw_cat_body(&mut fb, 32, cy_body);
        draw_cat_face(&mut fb, 32, cy_head, true, false);

        fb.draw_ellipse(24, 54, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
        fb.draw_ellipse(40, 54, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
    } else {
        // Bouncing up with sparkles!
        let cy_head: isize = 20;
        let cy_body: isize = 36;
        draw_cat_tail(&mut fb, 22, 40, -1);
        draw_cat_ears(&mut fb, cy_head);
        draw_cat_body(&mut fb, 32, cy_body);
        draw_cat_face(&mut fb, 32, cy_head, true, true);

        // Paws up in the air
        fb.draw_ellipse(20, 26, 4, 4, palette::WHITE_FUR, Some(palette::OUTLINE));
        fb.draw_ellipse(44, 26, 4, 4, palette::WHITE_FUR, Some(palette::OUTLINE));

        fb.draw_sparkle(52, 12);
        fb.draw_sparkle(12, 14);
    }

    fb
}

fn render_mimi_drag() -> FrameBuffer {
    let mut fb = FrameBuffer::new();
    // Dangling kitten held by mouse
    let cy_head: isize = 20;
    let cy_body: isize = 38;

    draw_cat_ears(&mut fb, cy_head);
    // Stretched body
    fb.draw_ellipse(
        32,
        cy_body,
        10,
        16,
        palette::ORANGE_FUR,
        Some(palette::OUTLINE),
    );
    fb.draw_ellipse(32, cy_body, 5, 10, palette::WHITE_FUR, None);
    draw_cat_face(&mut fb, 32, cy_head, true, false);

    // Dangling paws
    fb.draw_ellipse(25, 42, 3, 5, palette::WHITE_FUR, Some(palette::OUTLINE));
    fb.draw_ellipse(39, 42, 3, 5, palette::WHITE_FUR, Some(palette::OUTLINE));
    fb.draw_ellipse(27, 56, 3, 4, palette::WHITE_FUR, Some(palette::OUTLINE));
    fb.draw_ellipse(37, 56, 3, 4, palette::WHITE_FUR, Some(palette::OUTLINE));

    fb
}

fn render_mimi_pet() -> FrameBuffer {
    let mut fb = FrameBuffer::new();
    let cy_head: isize = 24;
    let cy_body: isize = 42;

    draw_cat_tail(&mut fb, 22, 48, 1);
    draw_cat_ears(&mut fb, cy_head);
    draw_cat_body(&mut fb, 32, cy_body);
    draw_cat_face(&mut fb, 32, cy_head, false, true);

    // Front paws together
    fb.draw_ellipse(27, 54, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
    fb.draw_ellipse(37, 54, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));

    // Big heart floating above head
    fb.draw_heart(48, 10, 6);

    fb
}

fn render_mimi_eat(step: usize) -> FrameBuffer {
    let mut fb = FrameBuffer::new();
    let cy_head: isize = 25;
    let cy_body: isize = 43;

    draw_cat_tail(&mut fb, 22, 48, -1);
    draw_cat_ears(&mut fb, cy_head);
    draw_cat_body(&mut fb, 32, cy_body);

    if step == 0 {
        // Looking at fish with delight
        draw_cat_face(&mut fb, 32, cy_head, true, false);
        // Left paw resting, right paw touching fish
        fb.draw_ellipse(26, 54, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
        fb.draw_ellipse(38, 52, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
        // Fish treat on the right side
        fb.draw_fish(48, 52, 5);
    } else {
        // Chewing happily with crumbs and a floating heart
        draw_cat_face(&mut fb, 32, cy_head, false, true);
        fb.draw_ellipse(27, 54, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
        fb.draw_ellipse(37, 54, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
        fb.draw_crumbs(44, 50);
        fb.draw_heart(48, 12, 4);
    }

    fb
}

fn render_mimi_stretch(step: usize) -> FrameBuffer {
    let mut fb = FrameBuffer::new();
    if step == 0 {
        // Front paw reach: head down, paws extended far out, back arched down
        let cy_head: isize = 32;
        let cy_body: isize = 38;

        draw_cat_tail(&mut fb, 18, 36, -2);
        draw_cat_ears(&mut fb, cy_head);
        // Elongated body
        fb.draw_ellipse(
            30,
            cy_body,
            15,
            10,
            palette::ORANGE_FUR,
            Some(palette::OUTLINE),
        );
        fb.draw_ellipse(30, cy_body + 1, 9, 6, palette::WHITE_FUR, None);
        draw_cat_face(&mut fb, 40, cy_head, false, true);

        // Extended front paws
        fb.draw_ellipse(50, 52, 5, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
        fb.draw_ellipse(46, 54, 5, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
        // Back paws tucked
        fb.draw_ellipse(18, 48, 4, 4, palette::WHITE_FUR, Some(palette::OUTLINE));
    } else {
        // Arching back high up, tail high in the air
        let cy_head: isize = 26;
        let cy_body: isize = 34;

        // Tail straight up
        for y in 0..14 {
            let ty = (36 - y) as usize;
            fb.set_pixel(20, ty, palette::ORANGE_FUR);
            fb.set_pixel(21, ty, palette::ORANGE_FUR);
            if y > 10 {
                fb.set_pixel(20, ty, palette::WHITE_FUR);
                fb.set_pixel(21, ty, palette::WHITE_FUR);
            }
        }

        draw_cat_ears(&mut fb, cy_head);
        // Tall arched body
        fb.draw_ellipse(
            32,
            cy_body,
            12,
            16,
            palette::ORANGE_FUR,
            Some(palette::OUTLINE),
        );
        fb.draw_ellipse(32, cy_body, 6, 10, palette::WHITE_FUR, None);
        draw_cat_face(&mut fb, 34, cy_head, false, true);

        // Paws planted on ground
        fb.draw_ellipse(24, 54, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
        fb.draw_ellipse(40, 54, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
    }
    fb
}

fn render_mimi_curious(step: usize) -> FrameBuffer {
    let mut fb = FrameBuffer::new();
    let (cx_head, cy_head): (isize, isize) = if step == 0 { (30, 23) } else { (34, 23) };
    let cy_body: isize = 42;

    draw_cat_tail(&mut fb, 22, 48, if step == 0 { -1 } else { 1 });
    // Tilted ears
    if step == 0 {
        // Left ear slightly lower, right ear upright
        for y in 0..12 {
            let w = (12 - y) * 8 / 12;
            for x in 0..w {
                let px = 18 - x;
                let py = (cy_head - 10 + y as isize) as usize;
                fb.set_pixel(px, py, palette::ORANGE_FUR);
                if x < w / 2 && y > 3 {
                    fb.set_pixel(px, py, palette::PINK_EAR);
                }
            }
        }
        for y in 0..12 {
            let w = (12 - y) * 8 / 12;
            for x in 0..w {
                let px = 41 + x;
                let py = (cy_head - 14 + y as isize) as usize;
                fb.set_pixel(px, py, palette::ORANGE_FUR);
                if x < w / 2 && y > 3 {
                    fb.set_pixel(px, py, palette::PINK_EAR);
                }
            }
        }
    } else {
        draw_cat_ears(&mut fb, cy_head);
    }

    draw_cat_body(&mut fb, 32, cy_body);
    draw_cat_face(&mut fb, cx_head, cy_head, true, false);

    // Extra glint in eyes for curiosity
    fb.set_pixel(
        (cx_head - 5) as usize,
        (cy_head + 1) as usize,
        palette::EYE_GLINT,
    );
    fb.set_pixel(
        (cx_head + 7) as usize,
        (cy_head + 1) as usize,
        palette::EYE_GLINT,
    );

    // Sitting paws
    fb.draw_ellipse(28, 54, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
    fb.draw_ellipse(36, 54, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));

    fb
}

fn render_mimi_jump(step: usize) -> FrameBuffer {
    let mut fb = FrameBuffer::new();
    if step == 0 {
        // Crouch low, preparing vertical leap
        let cy_head: isize = 30;
        let cy_body: isize = 45;

        draw_cat_tail(&mut fb, 20, 48, -2);
        draw_cat_ears(&mut fb, cy_head);
        draw_cat_body(&mut fb, 32, cy_body);
        draw_cat_face(&mut fb, 32, cy_head, true, false);

        fb.draw_ellipse(24, 56, 5, 2, palette::WHITE_FUR, Some(palette::OUTLINE));
        fb.draw_ellipse(40, 56, 5, 2, palette::WHITE_FUR, Some(palette::OUTLINE));
    } else {
        // High vertical bounce with sparkles!
        let cy_head: isize = 16;
        let cy_body: isize = 32;

        draw_cat_tail(&mut fb, 22, 38, 0);
        draw_cat_ears(&mut fb, cy_head);
        draw_cat_body(&mut fb, 32, cy_body);
        draw_cat_face(&mut fb, 32, cy_head, true, true);

        // Extended jumping paws
        fb.draw_ellipse(18, 22, 4, 4, palette::WHITE_FUR, Some(palette::OUTLINE));
        fb.draw_ellipse(46, 22, 4, 4, palette::WHITE_FUR, Some(palette::OUTLINE));
        fb.draw_ellipse(22, 44, 4, 4, palette::WHITE_FUR, Some(palette::OUTLINE));
        fb.draw_ellipse(42, 44, 4, 4, palette::WHITE_FUR, Some(palette::OUTLINE));

        // Multiple golden sparkles
        fb.draw_sparkle(10, 14);
        fb.draw_sparkle(54, 10);
        fb.draw_sparkle(52, 32);
        fb.draw_sparkle(12, 34);
    }
    fb
}

fn render_mimi_groom(step: usize) -> FrameBuffer {
    let mut fb = FrameBuffer::new();
    let cy_head: isize = 24;
    let cy_body: isize = 42;

    draw_cat_tail(&mut fb, 22, 48, -1);
    draw_cat_ears(&mut fb, cy_head);
    draw_cat_body(&mut fb, 32, cy_body);

    if step == 0 {
        // Raising paw up towards face
        draw_cat_face(&mut fb, 32, cy_head, true, false);
        // Right paw on ground, left paw raised to whiskers
        fb.draw_ellipse(38, 54, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
        fb.draw_ellipse(24, 34, 4, 4, palette::WHITE_FUR, Some(palette::OUTLINE));
    } else {
        // Rubbing face/ear, happy closed eyes
        draw_cat_face(&mut fb, 32, cy_head, false, true);
        fb.draw_ellipse(38, 54, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
        fb.draw_ellipse(22, 24, 4, 4, palette::WHITE_FUR, Some(palette::OUTLINE));
        // Little clean sparkle
        fb.draw_sparkle(14, 20);
    }

    fb
}

fn render_mimi_surprised() -> FrameBuffer {
    let mut fb = FrameBuffer::new();
    let cy_head: isize = 20;
    let cy_body: isize = 36;

    // Poofy wide tail
    for i in 0..14 {
        let tx = (22_isize - (i as isize * 10 / 14)) as usize;
        let ty = (38_isize - (i as isize * 10 / 14)) as usize;
        let color = if i > 10 {
            palette::WHITE_FUR
        } else {
            palette::ORANGE_FUR
        };
        fb.set_pixel(tx, ty, color);
        fb.set_pixel(tx + 1, ty, color);
        fb.set_pixel(tx, ty + 1, palette::DARK_ORANGE);
        if tx > 0 {
            fb.set_pixel(tx - 1, ty, palette::OUTLINE);
        }
    }

    draw_cat_ears(&mut fb, cy_head);
    draw_cat_body(&mut fb, 32, cy_body);

    // Face with big wide eyes and exclamation
    // Head shape
    fb.draw_ellipse(
        32,
        cy_head,
        15,
        12,
        palette::CREAM_FUR,
        Some(palette::OUTLINE),
    );
    // Orange forehead patch
    fb.draw_ellipse(32, cy_head - 6, 9, 5, palette::ORANGE_FUR, None);

    // Large surprised eyes (big white/green with tiny pupils)
    fb.draw_ellipse(
        26,
        cy_head,
        4,
        5,
        palette::WHITE_FUR,
        Some(palette::OUTLINE),
    );
    fb.draw_ellipse(26, cy_head, 3, 4, palette::EYE_GREEN, None);
    fb.set_pixel(26, cy_head as usize, palette::EYE_PUPIL);
    fb.set_pixel(25, (cy_head - 1) as usize, palette::EYE_GLINT);

    fb.draw_ellipse(
        38,
        cy_head,
        4,
        5,
        palette::WHITE_FUR,
        Some(palette::OUTLINE),
    );
    fb.draw_ellipse(38, cy_head, 3, 4, palette::EYE_GREEN, None);
    fb.set_pixel(38, cy_head as usize, palette::EYE_PUPIL);
    fb.set_pixel(37, (cy_head - 1) as usize, palette::EYE_GLINT);

    // Pink nose
    fb.set_pixel(32, (cy_head + 3) as usize, palette::PINK_NOSE);
    // Small open mouth 'o'
    fb.draw_ellipse(32, cy_head + 6, 2, 2, palette::OUTLINE, None);

    // Whiskers
    for i in 0..4 {
        fb.set_pixel(18 + i, (cy_head + 3) as usize, palette::OUTLINE);
        fb.set_pixel(19 + i, (cy_head + 5) as usize, palette::OUTLINE);
        fb.set_pixel(43 + i, (cy_head + 3) as usize, palette::OUTLINE);
        fb.set_pixel(42 + i, (cy_head + 5) as usize, palette::OUTLINE);
    }

    // Paws poised in surprise
    fb.draw_ellipse(24, 48, 4, 4, palette::WHITE_FUR, Some(palette::OUTLINE));
    fb.draw_ellipse(40, 48, 4, 4, palette::WHITE_FUR, Some(palette::OUTLINE));

    // Exclamation mark above head
    fb.draw_exclamation(32, 2);

    fb
}

fn render_mimi_purr(step: usize) -> FrameBuffer {
    let mut fb = FrameBuffer::new();
    let cy_head: isize = if step == 0 { 21 } else { 20 };
    let cy_body: isize = 37;

    draw_cat_tail(&mut fb, 22, 48, -1);
    draw_cat_ears(&mut fb, cy_head);
    draw_cat_body(&mut fb, 32, cy_body);
    draw_cat_face(&mut fb, 32, cy_head, false, true);

    // Front paws tucked comfortably
    fb.draw_ellipse(28, 54, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
    fb.draw_ellipse(36, 54, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));

    // Vibrating purr musical notes / heart
    if step == 0 {
        fb.draw_heart(48, 14, 4);
    } else {
        fb.draw_sparkle(48, 12);
        fb.draw_sparkle(16, 16);
    }
    fb
}

fn render_mimi_knead(step: usize) -> FrameBuffer {
    let mut fb = FrameBuffer::new();
    let cy_head: isize = 21;
    let cy_body: isize = 37;

    draw_cat_tail(&mut fb, 22, 48, -1);
    draw_cat_ears(&mut fb, cy_head);
    draw_cat_body(&mut fb, 32, cy_body);
    draw_cat_face(&mut fb, 32, cy_head, false, true);

    // Alternating kneading paws
    if step == 0 {
        // Left paw pressing down, right paw slightly lifted
        fb.draw_ellipse(27, 55, 5, 4, palette::WHITE_FUR, Some(palette::OUTLINE));
        fb.draw_ellipse(37, 51, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
    } else {
        // Right paw pressing down, left paw slightly lifted
        fb.draw_ellipse(27, 51, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
        fb.draw_ellipse(37, 55, 5, 4, palette::WHITE_FUR, Some(palette::OUTLINE));
    }

    fb.draw_heart(46, 16, 3);
    fb
}

fn render_mimi_zoomies(step: usize) -> FrameBuffer {
    let mut fb = FrameBuffer::new();
    let cy_head: isize = 22;
    let cy_body: isize = 34;

    // Tail high in the air with frantic angle
    for i in 0..12 {
        let tx = (18_isize - (i as isize * 8 / 12)) as usize;
        let ty = (32_isize - (i as isize * 14 / 12)) as usize;
        fb.set_pixel(tx, ty, palette::ORANGE_FUR);
        fb.set_pixel(tx + 1, ty, palette::DARK_ORANGE);
    }

    draw_cat_ears(&mut fb, cy_head);
    // Stretched running body
    fb.draw_ellipse(
        32,
        cy_body,
        17,
        9,
        palette::ORANGE_FUR,
        Some(palette::OUTLINE),
    );
    fb.draw_ellipse(34, cy_body + 1, 10, 5, palette::CREAM_FUR, None);

    // Wide excited eyes
    draw_cat_face(&mut fb, 36, cy_head, true, false);

    // Frantic zoomies running legs
    if step == 0 {
        fb.draw_ellipse(18, 48, 5, 2, palette::WHITE_FUR, Some(palette::OUTLINE));
        fb.draw_ellipse(46, 44, 5, 2, palette::WHITE_FUR, Some(palette::OUTLINE));
    } else {
        fb.draw_ellipse(22, 44, 5, 2, palette::WHITE_FUR, Some(palette::OUTLINE));
        fb.draw_ellipse(42, 48, 5, 2, palette::WHITE_FUR, Some(palette::OUTLINE));
    }

    // Motion streaks
    fb.draw_sparkle(10, 36);
    fb.draw_sparkle(8, 44);
    fb
}

fn render_mimi_loaf() -> FrameBuffer {
    let mut fb = FrameBuffer::new();
    let cy_head: isize = 22;
    let cy_body: isize = 38;

    // Loaf posture: paws and tail fully tucked into an adorable warm bread loaf
    fb.draw_ellipse(
        32,
        cy_body,
        18,
        12,
        palette::ORANGE_FUR,
        Some(palette::OUTLINE),
    );
    // White / cream chest under chin
    fb.draw_ellipse(32, cy_body - 4, 11, 7, palette::CREAM_FUR, None);
    // Tail curled neatly along the side
    for i in 0..8 {
        fb.set_pixel(14 + i, 46, palette::DARK_ORANGE);
        fb.set_pixel(14 + i, 47, palette::OUTLINE);
    }

    draw_cat_ears(&mut fb, cy_head);
    draw_cat_face(&mut fb, 32, cy_head, false, true);

    fb
}

fn render_mimi_drink(step: usize) -> FrameBuffer {
    let mut fb = FrameBuffer::new();
    let cy_head: isize = if step == 0 { 24 } else { 22 };
    let cy_body: isize = 38;

    draw_cat_tail(&mut fb, 22, 48, -1);
    draw_cat_ears(&mut fb, cy_head);
    draw_cat_body(&mut fb, 32, cy_body);
    draw_cat_face(&mut fb, 32, cy_head, false, true);

    // Front paws on ground next to bowl
    fb.draw_ellipse(26, 54, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));
    fb.draw_ellipse(38, 54, 4, 3, palette::WHITE_FUR, Some(palette::OUTLINE));

    // Water bowl in front of paws
    fb.draw_ellipse(32, 57, 10, 4, palette::FISH_BLUE, Some(palette::OUTLINE));
    fb.draw_ellipse(32, 56, 8, 3, palette::FISH_LIGHT, None);

    // Little pink tongue lapping water or splash
    if step == 0 {
        fb.set_pixel(32, 40, palette::PINK_NOSE);
        fb.set_pixel(32, 41, palette::PINK_NOSE);
    } else {
        fb.set_pixel(32, 53, palette::WHITE_FUR);
        fb.draw_sparkle(32, 52);
    }

    fb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mimi_sprite_sheet_generation() {
        let sheet = MimiSpriteSheet::generate();
        let expected_frames = [
            "idle_0",
            "idle_1",
            "walk_0",
            "walk_1",
            "walk_2",
            "sit_0",
            "sleep_0",
            "sleep_1",
            "play_0",
            "play_1",
            "drag_0",
            "pet_0",
            // Enriched animations
            "eat_0",
            "eat_1",
            "stretch_0",
            "stretch_1",
            "curious_0",
            "curious_1",
            "jump_0",
            "jump_1",
            "groom_0",
            "groom_1",
            "surprised_0",
            // Real Cat behavior frames
            "purr_0",
            "purr_1",
            "knead_0",
            "knead_1",
            "zoomies_0",
            "zoomies_1",
            "loaf_0",
            "drink_0",
            "drink_1",
        ];

        for frame_name in &expected_frames {
            assert!(
                sheet.frames.contains_key(*frame_name),
                "Missing animation frame: {}",
                frame_name
            );
            let frame = sheet.get_frame(frame_name);
            assert_eq!(frame.pixels.len(), FRAME_BYTES);

            let has_content = frame.pixels.chunks(4).any(|px| px[3] > 0);
            assert!(
                has_content,
                "Frame {} must have non-empty pixel content",
                frame_name
            );
        }
    }

    #[test]
    fn test_mimi_breed_recoloring() {
        let tabby_sheet = MimiSpriteSheet::generate_for_breed(openpet_types::CatBreed::Tabby);
        let tuxedo_sheet = MimiSpriteSheet::generate_for_breed(openpet_types::CatBreed::Tuxedo);
        let calico_sheet = MimiSpriteSheet::generate_for_breed(openpet_types::CatBreed::Calico);

        assert!(tabby_sheet.frames.contains_key("idle_0"));
        assert!(tuxedo_sheet.frames.contains_key("purr_0"));
        assert!(calico_sheet.frames.contains_key("loaf_0"));
    }

    #[test]
    fn test_mimi_bgra_scaled_rendering() {
        let sheet = MimiSpriteSheet::generate();
        let (bgra, w, h) = sheet.render_frame_bgra_scaled("idle_0", 2, Some(0x00FF00FF));
        assert_eq!(w, 128);
        assert_eq!(h, 128);
        assert_eq!(bgra.len(), 128 * 128 * 4);

        // Verify chroma key translation for transparent pixels:
        // Magenta = R: 255, G: 0, B: 255 -> in BGRA: B=255, G=0, R=255
        let (corner_b, corner_g, corner_r) = (bgra[0], bgra[1], bgra[2]);
        assert_eq!(corner_b, 255);
        assert_eq!(corner_g, 0);
        assert_eq!(corner_r, 255);
    }

    #[test]
    fn test_mimi_zero_dimensions_graceful() {
        let mut fb = FrameBuffer::new();
        // Should not panic with division by zero
        fb.draw_ellipse(32, 32, 0, 10, palette::CREAM_FUR, Some(palette::OUTLINE));
        fb.draw_ellipse(32, 32, 10, 0, palette::CREAM_FUR, Some(palette::OUTLINE));
        fb.draw_ellipse(32, 32, 0, 0, palette::CREAM_FUR, Some(palette::OUTLINE));
        fb.draw_heart(32, 32, 0);
        fb.draw_zzz(32, 32, 0);
        fb.draw_fish(32, 32, 0);
        fb.draw_fish(0, 0, 5);
        fb.draw_sparkle(0, 0);
        fb.draw_crumbs(0, 0);
        fb.draw_crumbs(32, 32);
        fb.draw_exclamation(32, 32);
        fb.draw_crumbs(usize::MAX, usize::MAX);
        fb.draw_sparkle(usize::MAX, usize::MAX);
        fb.draw_exclamation(usize::MAX, usize::MAX);
        fb.draw_fish(usize::MAX, usize::MAX, 5);
    }
}
