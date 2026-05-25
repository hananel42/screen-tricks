use std::slice;

use crate::capture::ImageSource;
use font8x8::{BASIC_FONTS, UnicodeFonts};

pub(super) const fn rgba_premul(r: u8, g: u8, b: u8, a: u8) -> u32 {
    let a32 = a as u32;
    let r32 = ((r as u32) * a32 + 127) / 255;
    let g32 = ((g as u32) * a32 + 127) / 255;
    let b32 = ((b as u32) * a32 + 127) / 255;
    (a32 << 24) | (r32 << 16) | (g32 << 8) | b32
}

/// A `Canvas` represents a 2D bitmap surface composed of pixels stored as a flat array of 32-bit integers.
pub struct Canvas {
    pub(super) bits: *mut u32,
    pub(super) len: usize,
    pub(super) width: i32,
    pub(super) height: i32,
}

impl Canvas {
    /// Returns the width of the canvas as an `i32`.
    #[inline(always)]
    pub fn width(&self) -> i32 {
        self.width
    }
    /// Returns the height of the canvas as an `i32`.
    #[inline(always)]
    pub fn height(&self) -> i32 {
        self.height
    }


    /// fills the canvas with (0,0,0,0).
    pub fn clear(&mut self) {
        unsafe {
            self.frame_mut().fill(0);
        }
    }

    /// Writes a premultiplied RGBA pixel at (x, y) iff at bounds.
    pub fn put_pixel(&mut self, x: i32, y: i32, (r, g, b, a): (u8, u8, u8, u8)) {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return;
        }

        let idx = (y as usize) * (self.width as usize) + (x as usize);
        unsafe {
            let frame = self.frame_mut();
            let dst = &mut frame[idx];
            Self::blend_pixel(dst, rgba_premul(r, g, b, a));
        }
    }


    /// Fills the given rect with (0,0,0,0).
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

    /// Fills the given rect with the given color.
    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, (r, g, b, a): (u8, u8, u8, u8)) {
        let color = rgba_premul(r, g, b, a);
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

    /// Fills the canvas with specific color.
    pub fn fill(&mut self, (r, g, b, a): (u8, u8, u8, u8)) {
        unsafe {
            self.frame_mut().fill(rgba_premul(r,g,b,a));
        }

    }

    /// Draws an outline around a given rectangle.
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

    /// Draws a character at the specified position with a given scale and color.
    ///
    /// This function renders a character from a predefined font set onto the canvas.
    /// The character is drawn with the specified scale (each pixel is expanded by this factor),
    /// and filled with the provided RGBA color.
    ///
    /// # Arguments
    ///
    /// * `x` - The x-coordinate (in pixels) where the character should be drawn.
    /// * `y` - The y-coordinate (in pixels) where the character should be drawn.
    /// * `ch` - The character to draw. If not found in the font, the '?' character is used as a fallback.
    /// * `scale` - The scaling factor for each pixel of the character. Must be at least 1.
    /// * `rgba` - A tuple of four `u8` values representing the red, green, blue, and alpha components
    ///            of the color to use for drawing the character.
    ///
    /// # Behavior
    ///
    /// - If the character is not found in the built-in font set, the '?' character is used instead.
    /// - The character is drawn as a block of pixels, with each bit in the font data representing a pixel.
    /// - The drawing is performed using `fill_rect`, which fills a rectangle with the specified color.
    /// - The scale is clamped to a minimum of 1 to avoid invalid rendering.
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

    /// Draws text at a specified position with a given scale and color.
    ///
    /// This function renders a string of text at the specified (x, y) coordinates, with each character drawn at the appropriate position based on the character's width and the provided scale.
    ///
    /// - `x`: The horizontal position (in pixels) where the text should start.
    /// - `y`: The vertical position (in pixels) where the text should start.
    /// - `text`: The string of characters to be rendered.
    /// - `scale`: The scale factor for the font size. Must be at least 1. A higher value increases the size of the text.
    /// - `rgba`: A 4-tuple representing the red, green, blue, and alpha components of the text color (each 0-255).
    ///
    ///
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

    /// Draws an image scaled to fit within a destination rectangle.
    ///
    /// This function scales an image source (`img`) and draws it onto the destination surface at the specified coordinates
    /// (`dst_x`, `dst_y`) with the given dimensions (`dst_w`, `dst_h`). The scaling is performed using integer
    /// fixed-point arithmetic to avoid expensive floating-point divisions per pixel.
    ///
    /// # Arguments
    ///
    /// * `img` - A reference to an image source that implements `ImageSource`.
    /// * `dst_x` - The x-coordinate (in pixels) of the top-left corner of the destination rectangle.
    /// * `dst_y` - The y-coordinate (in pixels) of the top-left corner of the destination rectangle.
    /// * `dst_w` - The width (in pixels) of the destination rectangle.
    /// * `dst_h` - The height (in pixels) of the destination rectangle.
    ///
    /// # Behavior
    ///
    /// - If any dimension of the destination or source image is non-positive, the function returns early.
    /// - The destination rectangle is clamped to the bounds of the target surface.
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

            // Integer fixed-point scaling is noticeably cheaper than a division per pixel.
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


    /// Draws an image transformed by scaling, rotation, and pivoting, using nearest-neighbor sampling.
    ///
    /// This function applies a 2D transformation to an image source (scaling, rotation around a pivot point),
    /// then renders the transformed image onto a destination buffer (e.g., a frame). The transformation is
    /// applied relative to a specified pivot point in the source image, and the final output is clipped
    /// to the bounds of the destination.
    ///
    /// # Arguments
    ///
    /// * `img` - A reference to the image source (must implement `ImageSource`).
    /// * `dst_x`, `dst_y` - The screen coordinates of the pivot point for the transformed image.
    /// * `scale_x`, `scale_y` - The scaling factors in the x and y directions.
    /// * `rotation` - The rotation angle in radians (positive is counterclockwise).
    /// * `pivot_x`, `pivot_y` - The coordinates within the source image at which to pivot the transformation.
    ///
    /// # Behavior
    ///
    /// - If the source image has zero or negative dimensions, or if either scale factor is near zero,
    ///   the function returns early without drawing.
    /// - The corners of the source image are transformed using the specified pivot, scale, and rotation.
    /// - The bounding box of the transformed image is computed and clipped to the destination bounds.
    /// - The destination image is updated using nearest-neighbor sampling with alpha blending.
    #[inline]
    pub fn draw_image_transformed<T: ImageSource + ?Sized>(&mut self, img: &T, dst_x: f32,
        dst_y: f32, scale_x: f32, scale_y: f32, rotation: f32, pivot_x: f32, pivot_y: f32,
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

        //
        // transformed corners -> screen bounding box
        //

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

        //
        // inverse transform
        //

        let inv_scale_x = 1.0 / scale_x;
        let inv_scale_y = 1.0 / scale_y;

        unsafe {
            let dst_stride = self.width as usize;
            let frame = self.frame_mut();

            for dy in y0..y1 {
                let dst_row = (dy as usize) * dst_stride;

                for dx in x0..x1 {
                    //
                    // screen -> local transformed
                    //

                    let tx = dx as f32 - dst_x;
                    let ty = dy as f32 - dst_y;

                    //
                    // inverse rotation
                    //

                    let rx = tx * cos_r + ty * sin_r;
                    let ry = -tx * sin_r + ty * cos_r;

                    //
                    // inverse scale
                    //

                    let sx = rx * inv_scale_x + pivot_x;
                    let sy = ry * inv_scale_y + pivot_y;

                    //
                    // clip
                    //

                    if sx < 0.0 || sy < 0.0 || sx >= src_w as f32 || sy >= src_h as f32 {
                        continue;
                    }

                    //
                    // nearest-neighbor sample
                    //

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



    /// Draws an image as is at a given position.
    #[inline]
    pub fn draw_image<T: ImageSource + ?Sized>(&mut self, img: &T, dst_x: i32, dst_y: i32) {
        self.draw_image_scaled(img, dst_x, dst_y, img.width(), img.height());
    }

    unsafe fn frame_mut(&mut self) -> &mut [u32] {
        unsafe {
            slice::from_raw_parts_mut(self.bits, self.len)
        }
    }

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
