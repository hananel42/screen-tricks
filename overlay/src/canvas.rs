//! # Canvas Rendering and Blending Engine
//!
//! This module provides the core 2D rasterization and pixel-blending algorithms. 
//! It features a raw-pointer backed [`Canvas`] surface that supports blitting imagery, 
//! software-alpha pixel blending, primitive shape rendering, and text rasterization 
//! via fixed-width font glyphs.

use std::slice;

use crate::image::frames::ImageSource;
use crate::image::common::*;
use font8x8::{BASIC_FONTS, UnicodeFonts};

/// A 2D flat bitmap surface for fast pixel rendering and composition.
///
/// Wraps a raw mutable pointer memory slice, tracking surface boundary dimensions.
/// Memory validation relies on the wrapper structure creating this surface safely.
pub struct Canvas {
    pub(super) bits: *mut u32,
    pub(super) len: usize,
    pub(super) width: i32,
    pub(super) height: i32,
}

impl Canvas {
    /// Returns the active pixel width of the canvas surface.
    #[inline(always)]
    pub fn width(&self) -> i32 {
        self.width
    }

    /// Returns the active pixel height of the canvas surface.
    #[inline(always)]
    pub fn height(&self) -> i32 {
        self.height
    }

    /// Clears the entire canvas back to a fully transparent state (`0x00000000`).
    pub fn clear(&mut self) {
        unsafe {
            self.frame_mut().fill(0);
        }
    }

    /// Composes a single raw channel color tuple at specific `(x, y)` pixel coordinates.
    ///
    /// Automatically performs bounds checking. Alpha blending occurs automatically if 
    /// the target color contains an alpha value less than 255.
    pub fn put_pixel(&mut self, x: i32, y: i32, color: Color) {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return;
        }

        let idx = (y as usize) * (self.width as usize) + (x as usize);
        unsafe {
            let frame = self.frame_mut();
            let dst = &mut frame[idx];
            Self::blend_pixel(dst, rgba_premul(color));
        }
    }

    /// Completely clears a structural sub-region rectangle back to transparent (`0x00000000`).
    ///
    /// Optimized via vectorized chunk slice filling (`fill(0)`).
    pub fn clear_rect(&mut self, x: i32, y: i32, w: i32, h: i32) {
        if w <= 0 || h <= 0 {
            return;
        }

        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w).min(self.width);
        let y1 = (y + h).min(self.height);

        if x0 >= x1 || y0 >= y1 {
            return;
        }

        let stride = self.width as usize;

        unsafe {
            let frame = self.frame_mut();

            for yy in y0..y1 {
                let row = (yy as usize) * stride;
                let start = row + (x0 as usize);
                let end = row + (x1 as usize);

                frame[start..end].fill(0);
            }
        }
    }

    /// Fills a bounded rectangular coordinates zone with a uniform color.
    ///
    /// Automatically branches into a vectorized `fill` block if alpha is opaque (255), 
    /// fallback-routing into sequential pixel blending steps under fractional alpha bounds.
    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Color) {
        let color = rgba_premul(color);
        if w <= 0 || h <= 0 {
            return;
        }

        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w).min(self.width);
        let y1 = (y + h).min(self.height);

        if x0 >= x1 || y0 >= y1 {
            return;
        }

        let alpha = (color >> 24) & 0xFF;
        let width = self.width as usize;
        unsafe {
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
    }

    /// Uniformly floods the entire canvas layout using a single solid color.
    pub fn fill(&mut self, color: Color) {
        unsafe {
            self.frame_mut().fill(rgba_premul(color));
        }
    }

    /// Renders a hollow wireframe boundary outline representing a targeted rectangle shape.
    pub fn draw_rect_outline(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        rgba: (u8, u8, u8, u8),
        thickness: i32,
    ) {
        if thickness <= 0 {
            return;
        }

        self.fill_rect(x, y, w, thickness, rgba);
        self.fill_rect(x, y + h - thickness, w, thickness, rgba);
        self.fill_rect(x, y, thickness, h, rgba);
        self.fill_rect(x + w - thickness, y, thickness, h, rgba);
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
    pub fn draw_char(&mut self, x: i32, y: i32, ch: char, scale: i32, rgba: (u8, u8, u8, u8)) {
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
    pub fn draw_text(&mut self, x: i32, y: i32, text: &str, scale: i32, rgba: (u8, u8, u8, u8)) {
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
    pub fn draw_image_scaled<T: ImageSource + ?Sized>(
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
        let x1 = (dst_x + dst_w).min(self.width);
        let y1 = (dst_y + dst_h).min(self.height);

        if x0 >= x1 || y0 >= y1 {
            return;
        }

        let src_pixels = img.pixels();
        let src_origin = img.origin();
        let src_stride = img.stride();
        let src_w = img.width();
        let src_h = img.height();

        let dst_stride = self.width as usize;

        unsafe {
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
    }

    /// Renders an affine-transformed imagery source backing arbitrary scale tracking, 
    /// rotative radian displacements, and specialized source pivot anchors.
    ///
    /// # Arguments
    /// * `img` - Reference container implementing [`ImageSource`].
    /// * `dst_x`, `dst_y` - Target global screen layout placement location for the pivot point.
    /// * `scale_x`, `scale_y` - Custom dimensional axis multipliers.
    /// * `rotation` - Angular orientation **measured in radians** (positive values shift orientation clockwise).
    /// * `pivot_x`, `pivot_y` - Local inner source asset coordinates designated as the transformation center.
    #[inline]
    pub fn draw_image_transformed<T: ImageSource + ?Sized>(
        &mut self,
        img: &T,
        dst_x: f32,
        dst_y: f32,
        scale_x: f32,
        scale_y: f32,
        rotation: f32,
        pivot_x: f32,
        pivot_y: f32,
    ) {
        let src_w = img.width();
        let src_h = img.height();

        if src_w <= 0 || src_h <= 0 {
            return;
        }

        if scale_x.abs() <= f32::EPSILON || scale_y.abs() <= f32::EPSILON {
            return;
        }

        let src_pixels = img.pixels();
        let src_stride = img.stride();
        let src_origin = img.origin();

        let cos_r = rotation.cos();
        let sin_r = rotation.sin();

        let corners = [
            (0.0f32, 0.0f32),
            (src_w as f32, 0.0f32),
            (src_w as f32, src_h as f32),
            (0.0f32, src_h as f32),
        ];

        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for (cx, cy) in corners {
            let lx = (cx - pivot_x) * scale_x;
            let ly = (cy - pivot_y) * scale_y;

            let rx = lx * cos_r - ly * sin_r;
            let ry = lx * sin_r + ly * cos_r;

            let sx = dst_x + rx;
            let sy = dst_y + ry;

            min_x = min_x.min(sx);
            min_y = min_y.min(sy);
            max_x = max_x.max(sx);
            max_y = max_y.max(sy);
        }

        let x0 = (min_x.floor() as i32).max(0);
        let y0 = (min_y.floor() as i32).max(0);
        let x1 = (max_x.ceil() as i32).min(self.width);
        let y1 = (max_y.ceil() as i32).min(self.height);

        if x0 >= x1 || y0 >= y1 {
            return;
        }

        let inv_scale_x = 1.0 / scale_x;
        let inv_scale_y = 1.0 / scale_y;

        unsafe {
            let dst_stride = self.width as usize;
            let frame = self.frame_mut();

            for dy in y0..y1 {
                let dst_row = (dy as usize) * dst_stride;

                for dx in x0..x1 {
                    let tx = dx as f32 - dst_x;
                    let ty = dy as f32 - dst_y;

                    let rx = tx * cos_r + ty * sin_r;
                    let ry = -tx * sin_r + ty * cos_r;

                    let sx = rx * inv_scale_x + pivot_x;
                    let sy = ry * inv_scale_y + pivot_y;

                    if sx < 0.0 || sy < 0.0 || sx >= src_w as f32 || sy >= src_h as f32 {
                        continue;
                    }

                    let src_x = sx as usize;
                    let src_y = sy as usize;

                    let src = src_pixels[src_origin + src_y * src_stride + src_x];

                    let alpha = (src >> 24) as u8;
                    if alpha == 0 {
                        continue;
                    }

                    let idx = dst_row + (dx as usize);

                    if alpha == 255 {
                        frame[idx] = src;
                    } else {
                        Self::blend_pixel(&mut frame[idx], src);
                    }
                }
            }
        }
    }

    /// Draws an asset on the canvas at raw native scale dimensions.
    #[inline]
    pub fn draw_image<T: ImageSource + ?Sized>(&mut self, img: &T, dst_x: i32, dst_y: i32) {
        self.draw_image_scaled(img, dst_x, dst_y, img.width(), img.height());
    }

    /// Accesses the continuous underlying mutable frame memory buffer.
    unsafe fn frame_mut(&mut self) -> &mut [u32] {
        unsafe {
            slice::from_raw_parts_mut(self.bits, self.len)
        }
    }

    /// Performs a high-accuracy, 32-bit software alpha blend overlay calculation over a single pixel location.
    /// Utilizes the rounding equation `(value * inv + 127) / 255`.
    #[inline(always)]
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
}