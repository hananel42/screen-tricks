use std::slice;

use crate::overlay::capture::ImageSource;
use font8x8::{UnicodeFonts, BASIC_FONTS};

pub(super) const fn rgba_premul(r: u8, g: u8, b: u8, a: u8) -> u32 {
    let a32 = a as u32;
    let r32 = ((r as u32) * a32 + 127) / 255;
    let g32 = ((g as u32) * a32 + 127) / 255;
    let b32 = ((b as u32) * a32 + 127) / 255;
    (a32 << 24) | (r32 << 16) | (g32 << 8) | b32
}

pub struct Canvas {
    pub(super) bits: *mut u32,
    pub(super) len: usize,
    pub(crate) width: i32,
    pub(crate) height: i32,
}

impl Canvas {
    unsafe fn frame_mut(&mut self) -> &mut [u32] {
        slice::from_raw_parts_mut(self.bits, self.len)
    }

    pub fn clear(&mut self) {
        unsafe {
            self.frame_mut().fill(0);
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

    pub fn put_pixel(&mut self, x: i32, y: i32, (r,g,b,a): (u8,u8,u8,u8)) {
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
    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, (r,g,b,a): (u8,u8,u8,u8)) {
        let color = rgba_premul(r,g,b,a);
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

    pub fn draw_rect_outline(&mut self, x: i32, y: i32, w: i32, h: i32, rgba: (u8, u8, u8, u8), thickness: i32) {
        if thickness <= 0 {
            return;
        }

        self.fill_rect(x, y, w, thickness, rgba);
        self.fill_rect(x, y + h - thickness, w, thickness, rgba);
        self.fill_rect(x, y, thickness, h, rgba);
        self.fill_rect(x + w - thickness, y, thickness, h, rgba);
    }

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
                    self.draw_char(cx, cy, ch.to_ascii_uppercase(), scale, rgba);
                    cx += advance;
                }
            }
        }
    }

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
                let sy = (((start_sy_fp + ((dy - y0) as i64) * step_y) >> 16)
                    .clamp(0, (src_h - 1) as i64)) as usize;
                let src_row = src_origin + sy * src_stride;
                let dst_row = (dy as usize) * dst_stride;

                let mut sx_fp = ((x0 - dst_x) as i64) * step_x;
                for dx in x0..x1 {
                    let sx = ((sx_fp >> 16).clamp(0, (src_w - 1) as i64)) as usize;
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

    #[inline]
    pub fn draw_image<T: ImageSource + ?Sized>(&mut self, img: &T, dst_x: i32, dst_y: i32) {
        self.draw_image_scaled(img, dst_x, dst_y, img.width(), img.height());
    }


}