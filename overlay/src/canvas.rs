//! # Canvas Rendering and Blending Engine
//!
//! This module provides the core 2D rasterization and pixel-blending algorithms.
//! It features a raw-pointer backed [`OverlayCanvas`] surface that supports blitting imagery,
//! software-alpha pixel blending, primitive shape rendering, and text rasterization
//! via fixed-width font glyphs.

use std::slice;

use crate::image::common::*;
use crate::image::frames::ImageSource;
use font8x8::{BASIC_FONTS, UnicodeFonts};



pub trait Canvas {
    /// Returns the active pixel width of the canvas surface.
    fn width(&self) -> i32;
    /// Returns the active pixel height of the canvas surface.
    fn height(&self) -> i32;

    /// Clears the entire canvas back to a fully transparent state (`0x00000000`).
    fn clear(&mut self) { 
        self.frame_mut().fill(0);
    }

    /// Accesses the continuous underlying mutable frame memory buffer.
    fn frame_mut(&mut self) -> &mut [u32];

    /// Composes a single raw channel color tuple at specific `(x, y)` pixel coordinates.
    ///
    /// Automatically performs bounds checking. Alpha blending occurs automatically if
    /// the target color contains an alpha value less than 255.
    fn put_pixel(&mut self, x: i32, y: i32, color: Color) {
        let width = self.width();
        let height = self.height();
        if x < 0 || y < 0 || x >= width || y >= height {
            return;
        }

        let idx = (y as usize) * (width as usize) + (x as usize);
        let frame = self.frame_mut();
        let dst = &mut frame[idx];
        Self::blend_pixel(dst, rgba_premul(color));
    
    }


    /// Performs a high-accuracy, 32-bit software alpha blend overlay calculation over a single pixel location.
    /// Utilizes the rounding equation `(value * inv + 127) / 255`.
    fn blend_pixel(dst: &mut u32, src: u32) {
        let sa = (src >> 24) & 0xFF;
        if sa == 0 {
            return;
        }
        if sa == 255 {
            *dst = src;
            return;
        }

        let inv = 255 - sa;

        let sb = src & 0xFF;
        let sg = (src >> 8) & 0xFF;
        let sr = (src >> 16) & 0xFF;
        let da = (*dst >> 24) & 0xFF;
        let db = *dst & 0xFF;
        let dg = (*dst >> 8) & 0xFF;
        let dr = (*dst >> 16) & 0xFF;

        let out_b = sb + (db * inv + 127) / 255;
        let out_g = sg + (dg * inv + 127) / 255;
        let out_r = sr + (dr * inv + 127) / 255;
        let out_a = sa + (da * inv + 127) / 255;

        *dst = (out_a << 24) | (out_r << 16) | (out_g << 8) | out_b;
    }


    /// Writes a raw pixel color directly to the frame buffer at the specified sequential index.
    ///
    /// This function is designed for use in critical graphics rendering loops where performance
    /// is the highest priority. Thanks to the `#[inline(always)]` attribute, the compiler will
    /// attempt to embed this code directly at the call site, eliminating function call overhead.
    ///
    /// # Safety
    ///
    /// This method performs direct memory access and bypasses runtime bounds checking.
    /// It is the caller's responsibility to ensure that:
    /// * The index `idx` is strictly less than the total capacity of the frame buffer (`idx < self.len`).
    ///
    /// Passing an out-of-bounds index will result in **Undefined Behavior (UB)**, which can
    /// cause memory corruption, visual artifacts, or an immediate program crash.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let idx = (y * canvas.width + x) as usize;
    ///
    /// // Bounds check is performed once outside or before calling the raw pixel write
    /// if idx < canvas.len() {
    ///     canvas.put_raw_pixel(idx, 0xFF00FF00); // Writes a solid green pixel (ARGB)
    /// }
    /// ```
    #[inline(always)]
    unsafe fn put_raw_pixel(&mut self, idx: usize, color: u32) {
        unsafe {
            *self.frame_mut().get_unchecked_mut(idx) = color;
        }
    }


    /// Completely clears a structural sub-region rectangle back to transparent (`0x00000000`).
    ///
    /// Optimized via vectorized chunk slice filling (`fill(0)`).
    fn clear_rect(&mut self, x: i32, y: i32, w: i32, h: i32) {
        let width = self.width();
        let height = self.height();
        if w <= 0 || h <= 0 {
            return;
        }

        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w).min(width);
        let y1 = (y + h).min(height);

        if x0 >= x1 || y0 >= y1 {
            return;
        }

        let stride = width as usize;
        let frame = self.frame_mut();
        for yy in y0..y1 {
            let row = (yy as usize) * stride;
            let start = row + (x0 as usize);
            let end = row + (x1 as usize);

            frame[start..end].fill(0);
        }
    
    }


    /// Fills a bounded rectangular coordinates zone with a uniform color.
    ///
    /// Automatically branches into a vectorized `fill` block if alpha is opaque (255),
    /// fallback-routing into sequential pixel blending steps under fractional alpha bounds.
    fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Color) {
        let width = self.width();
        let height = self.height();
        let alpha = color.3; //(r,g,b,a)
        let color = rgba_premul(color);
        if w <= 0 || h <= 0 {
            return;
        }

        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w).min(width);
        let y1 = (y + h).min(height);

        if x0 >= x1 || y0 >= y1 {
            return;
        }
        
        let width = width as usize;
    
        let frame = self.frame_mut();
        if alpha == 255 {
            for yy in y0..y1 {
                let row = (yy as usize) * width;
                let start = row + (x0 as usize);
                let end = row + (x1 as usize);
                frame[start..end].fill(color);
            }
        } else {
            for yy in y0..y1 {
                let row = (yy as usize) * width;
                for xx in x0..x1 {
                    let idx = row + (xx as usize);
                    Self::blend_pixel(&mut frame[idx], color);
                }
            }
        }
    
    }

    /// Uniformly floods the entire canvas layout using a single solid color.
    fn fill(&mut self, color: Color) {
        
        self.frame_mut().fill(rgba_premul(color));
        
    }

    /// Draws a single character glyph using an 8x8 bitmap font structure.
    ///
    /// Drops back to drawing a `'?'` glyph if the required unicode char is missing from the table.
    ///
    /// # Arguments
    /// * `x`, `y` - Bounding top-left anchor start position.
    /// * `ch` - Target character.
    /// * `scale` - Nearest-neighbor magnification scalar value (clamped to a minimum of 1).
    /// * `rgba` - Color layout tuple applied over enabled bit blocks.
    fn draw_char(&mut self, x: i32, y: i32, ch: char, scale: i32, rgba: (u8, u8, u8, u8)) {
        let scale = scale.max(1);
        let glyph = BASIC_FONTS.get(ch).or_else(|| BASIC_FONTS.get('?'));
        let Some(glyph) = glyph else {
            return;
        };

        for (row, bits) in glyph.iter().enumerate() {
            let bits = *bits;
            for col in 0..8 {
                if (bits & (1 << col)) != 0 {
                    self.fill_rect(
                        x + (col) * scale,
                        y + (row as i32) * scale,
                        scale,
                        scale,
                        rgba,
                    );
                }
            }
        }
    }

    /// Draws multi-line text strings, automatically translating standard `\n` linebreaks.
    ///
    /// Coordinates line advances and carriage returns dynamically based on text scales.
    fn draw_text(&mut self, x: i32, y: i32, text: &str, scale: i32, rgba: (u8, u8, u8, u8)) {
        let scale = scale.max(1);
        let advance = 8 * scale + scale;
        let mut cx = x;
        let mut cy = y;

        for ch in text.chars() {
            match ch {
                '\n' => {
                    cx = x;
                    cy += 8 * scale + scale;
                }
                '\r' => {}
                _ => {
                    self.draw_char(cx, cy, ch, scale, rgba);
                    cx += advance;
                }
            }
        }
    }

    /// Blits an [`ImageSource`] container into a destination rectangle region, scaling it dynamically.
    ///
    /// Employs optimized fixed-point bit shifting arithmetic (`<< 16`) to achieve seamless scaling
    /// throughput speeds without relying on runtime floating-point hardware steps.
    #[inline]
    fn draw_image_scaled<T: ImageSource + ?Sized>(
        &mut self,
        img: &T,
        dst_x: i32,
        dst_y: i32,
        dst_w: i32,
        dst_h: i32,
    ) {
        if dst_w <= 0 || dst_h <= 0 || img.width() <= 0 || img.height() <= 0 {
            return;
        }

        let x0 = dst_x.max(0);
        let y0 = dst_y.max(0);
        let x1 = (dst_x + dst_w).min(self.width());
        let y1 = (dst_y + dst_h).min(self.height());

        if x0 >= x1 || y0 >= y1 {
            return;
        }

        let src_pixels = img.pixels();
        let src_origin = img.origin();
        let src_stride = img.stride();
        let src_w = img.width();
        let src_h = img.height();

        let dst_stride = self.width() as usize;

    
        let frame = self.frame_mut();

        let step_x = ((src_w as i64) << 16) / (dst_w as i64);
        let step_y = ((src_h as i64) << 16) / (dst_h as i64);

        let start_sy_fp = ((y0 - dst_y) as i64) * step_y;

        for dy in y0..y1 {
            let sy = ((start_sy_fp + ((dy - y0) as i64) * step_y) >> 16)
                .clamp(0, (src_h - 1) as i64) as usize;
            let src_row = src_origin + sy * src_stride;
            let dst_row = (dy as usize) * dst_stride;

            let mut sx_fp = ((x0 - dst_x) as i64) * step_x;
            for dx in x0..x1 {
                let sx = (sx_fp >> 16).clamp(0, (src_w - 1) as i64) as usize;
                let src = src_pixels[src_row + sx];
                let idx = dst_row + (dx as usize);

                let alpha = (src >> 24) as u8;
                if alpha == 255 {
                    frame[idx] = src;
                } else if alpha != 0 {
                    Self::blend_pixel(&mut frame[idx], src);
                }

                sx_fp += step_x;
            }
        }
    }
    
    /// Draws an asset on the canvas at raw native scale dimensions.
    #[inline]
    fn draw_image<T: ImageSource + ?Sized>(&mut self, img: &T, dst_x: i32, dst_y: i32) {
        self.draw_image_scaled(img, dst_x, dst_y, img.width(), img.height());
    }


    fn copy_from_slice(&mut self, slice: &[u32]) { 
        self.frame_mut().copy_from_slice(slice);
    }
    
}



/// A 2D flat bitmap surface for fast pixel rendering and composition.
///
/// Wraps a raw mutable pointer memory slice, tracking surface boundary dimensions.
/// Memory validation relies on the wrapper structure creating this surface safely.
pub struct OverlayCanvas {
    pub(super) bits: *mut u32,
    pub(super) len: usize,
    pub(super) width: i32,
    pub(super) height: i32,
}
impl Canvas for OverlayCanvas {
    /// Returns the active pixel width of the canvas surface.
    #[inline(always)]
    fn width(&self) -> i32 {
        self.width
    }

    /// Returns the active pixel height of the canvas surface.
    #[inline(always)]
    fn height(&self) -> i32 {
        self.height
    }
    
    /// Accesses the continuous underlying mutable frame memory buffer.
    fn frame_mut(&mut self) -> &mut [u32] {
        unsafe { slice::from_raw_parts_mut(self.bits, self.len) }
    }
}
