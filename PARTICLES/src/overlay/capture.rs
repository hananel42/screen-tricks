//! Complete image/capture API for the overlay project.
//!
//! Put this in something like `overlay/capture.rs` and adjust the `crate::overlay::canvas::Canvas`
//! path in the bottom section if your module layout differs.
//!
//! Dependencies:
//! ```toml
//! [dependencies]
//! windows-sys = { version = "0.59", features = [
//!     "Win32_Foundation",
//!     "Win32_Graphics_Gdi",
//!     "Win32_UI_WindowsAndMessaging",
//! ] }
//! image = "0.25"
//! ```

use std::{
    ffi::c_void,
    mem::{size_of, zeroed},
    path::Path,
    ptr::null_mut,
    slice,
};

use image::ImageReader;
use windows_sys::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC,
    SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS, HBITMAP,
    HDC, HGDIOBJ, SRCCOPY,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SetProcessDPIAware, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

// ------------------------------
// Helpers
// ------------------------------

#[inline]
pub const fn rgba_premul(r: u8, g: u8, b: u8, a: u8) -> u32 {
    let a32 = a as u32;
    let r32 = ((r as u32) * a32 + 127) / 255;
    let g32 = ((g as u32) * a32 + 127) / 255;
    let b32 = ((b as u32) * a32 + 127) / 255;
    (a32 << 24) | (r32 << 16) | (g32 << 8) | b32
}

#[inline]
fn premul_rgba_bytes_to_u32(px: &[u8]) -> u32 {
    let r = px[0] as u32;
    let g = px[1] as u32;
    let b = px[2] as u32;
    let a = px[3] as u32;
    let r = (r * a + 127) / 255;
    let g = (g * a + 127) / 255;
    let b = (b * a + 127) / 255;
    (a << 24) | (r << 16) | (g << 8) | b
}

#[inline]
fn u32_to_rgba_bytes(px: u32) -> [u8; 4] {
    [
        ((px >> 16) & 0xFF) as u8,
        ((px >> 8) & 0xFF) as u8,
        (px & 0xFF) as u8,
        ((px >> 24) & 0xFF) as u8,
    ]
}

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn virtual_screen() -> Option<Self> {
        unsafe {
            let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);

            if width <= 0 || height <= 0 {
                None
            } else {
                Some(Self { x, y, width, height })
            }
        }
    }
}

#[derive(Debug)]
pub enum ImageError {
    Io(std::io::Error),
    Decode(image::ImageError),
    InvalidDimensions,
    Empty,
}

impl std::fmt::Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::Decode(e) => write!(f, "image decode error: {e}"),
            Self::InvalidDimensions => write!(f, "invalid image dimensions"),
            Self::Empty => write!(f, "empty image"),
        }
    }
}

impl std::error::Error for ImageError {}

impl From<std::io::Error> for ImageError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<image::ImageError> for ImageError {
    fn from(e: image::ImageError) -> Self {
        Self::Decode(e)
    }
}

// ------------------------------
// Image model
// ------------------------------



/// Owned image, premultiplied ARGB in u32 buffer.
///
/// Memory layout is row-major and contiguous. Each pixel is `0xAARRGGBB` as u32,
/// which on little-endian Windows is the same in-memory byte order expected by the
/// old DIBSection path.




#[derive(Clone)]
pub struct FrameImage {
    pub width: i32,
    pub height: i32,
    pub stride: usize, // in pixels
    pub pixels: Box<[u32]>,
}

impl FrameImage {
    pub fn empty() -> Self {
        Self {
            width: 0,
            height: 0,
            stride: 0,
            pixels: Box::new([]),
        }
    }

    pub fn from_raw_premultiplied(width: i32, height: i32, pixels: Vec<u32>) -> Result<Self, ImageError> {
        if width <= 0 || height <= 0 {
            return Err(ImageError::InvalidDimensions);
        }
        let expected = (width as usize) * (height as usize);
        if pixels.len() != expected {
            return Err(ImageError::InvalidDimensions);
        }
        Ok(Self {
            width,
            height,
            stride: width as usize,
            pixels: pixels.into_boxed_slice(),
        })
    }

    pub fn from_bytes_rgba(width: i32, height: i32, rgba_bytes: &[u8]) -> Result<Self, ImageError> {
        if width <= 0 || height <= 0 {
            return Err(ImageError::InvalidDimensions);
        }
        let expected = (width as usize) * (height as usize) * 4;
        if rgba_bytes.len() != expected {
            return Err(ImageError::InvalidDimensions);
        }

        let mut out = vec![0u32; (width as usize) * (height as usize)];
        for (i, px) in rgba_bytes.chunks_exact(4).enumerate() {
            out[i] = premul_rgba_bytes_to_u32(px);
        }

        Ok(Self {
            width,
            height,
            stride: width as usize,
            pixels: out.into_boxed_slice(),
        })
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ImageError> {
        let img = image::load_from_memory(bytes)?;
        Self::from_dynamic_image(img)
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ImageError> {
        let img = ImageReader::open(path)?.with_guessed_format()?.decode()?;
        Self::from_dynamic_image(img)
    }

    pub fn from_dynamic_image(img: image::DynamicImage) -> Result<Self, ImageError> {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        if width == 0 || height == 0 {
            return Err(ImageError::Empty);
        }

        let mut out = Vec::with_capacity((width as usize) * (height as usize));
        for px in rgba.as_raw().chunks_exact(4) {
            out.push(premul_rgba_bytes_to_u32(px));
        }

        Ok(Self {
            width: width as i32,
            height: height as i32,
            stride: width as usize,
            pixels: out.into_boxed_slice(),
        })
    }

    #[inline]
    pub fn as_slice(&self) -> &[u32] {
        &self.pixels
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u32] {
        &mut self.pixels
    }

    pub fn view(&self) -> ImageView<'_> {
        ImageView {
            width: self.width,
            height: self.height,
            stride: self.stride,
            pixels: &self.pixels,
            origin: 0,
        }
    }
}



#[derive(Clone, Copy)]
pub struct ImageView<'a> {
    pub width: i32,
    pub height: i32,
    pub stride: usize, // in pixels, stride of the underlying source buffer
    pub pixels: &'a [u32],
    pub origin: usize, // pixel offset from pixels[0]
}

impl<'a> ImageView<'a> {
    pub fn crop(&self, x: i32, y: i32, w: i32, h: i32) -> Option<ImageView<'a>> {
        if x < 0 || y < 0 || w <= 0 || h <= 0 {
            return None;
        }
        if x + w > self.width || y + h > self.height {
            return None;
        }

        Some(ImageView {
            width: w,
            height: h,
            stride: self.stride,
            pixels: self.pixels,
            origin: self.origin + (y as usize) * self.stride + (x as usize),
        })
    }

    pub fn to_owned(&self) -> FrameImage {
        let mut out = vec![0u32; (self.width as usize) * (self.height as usize)];
        for y in 0..(self.height as usize) {
            let src_row = self.origin + y * self.stride;
            let dst_row = y * (self.width as usize);
            out[dst_row..dst_row + self.width as usize]
                .copy_from_slice(&self.pixels[src_row..src_row + self.width as usize]);
        }

        FrameImage {
            width: self.width,
            height: self.height,
            stride: self.width as usize,
            pixels: out.into_boxed_slice(),
        }
    }

    pub fn resize_nearest(&self, dst_w: i32, dst_h: i32) -> FrameImage {
        if self.width <= 0 || self.height <= 0 || dst_w <= 0 || dst_h <= 0 {
            return FrameImage::empty();
        }

        let mut out = vec![0u32; (dst_w as usize) * (dst_h as usize)];
        let step_x = ((self.width as i64) << 16) / (dst_w as i64);
        let step_y = ((self.height as i64) << 16) / (dst_h as i64);

        for dy in 0..dst_h {
            let sy = (((dy as i64) * step_y) >> 16).clamp(0, (self.height - 1) as i64) as usize;
            let src_row = self.origin + sy * self.stride;
            let dst_row = (dy as usize) * (dst_w as usize);

            let mut sx_fp = 0i64;
            for dx in 0..dst_w {
                let sx = (sx_fp >> 16).clamp(0, (self.width - 1) as i64) as usize;
                out[dst_row + (dx as usize)] = self.pixels[src_row + sx];
                sx_fp += step_x;
            }
        }

        FrameImage {
            width: dst_w,
            height: dst_h,
            stride: dst_w as usize,
            pixels: out.into_boxed_slice(),
        }
    }

    pub fn rotate_90_cw(&self) -> FrameImage {
        if self.width <= 0 || self.height <= 0 {
            return FrameImage::empty();
        }

        let dst_w = self.height;
        let dst_h = self.width;

        let mut out =
            vec![0u32; (dst_w as usize) * (dst_h as usize)];

        let sw = self.width as usize;
        let sh = self.height as usize;

        for y in 0..sh {
            for x in 0..sw {
                let src_px =
                    self.pixels[self.origin + y * self.stride + x];

                let dx = sh - 1 - y;
                let dy = x;

                out[dy * (dst_w as usize) + dx] = src_px;
            }
        }

        FrameImage {
            width: dst_w,
            height: dst_h,
            stride: dst_w as usize,
            pixels: out.into_boxed_slice(),
        }
    }

    pub fn rotate_90_ccw(&self) -> FrameImage {
        if self.width <= 0 || self.height <= 0 {
            return FrameImage::empty();
        }

        let dst_w = self.height;
        let dst_h = self.width;

        let mut out =
            vec![0u32; (dst_w as usize) * (dst_h as usize)];

        let sw = self.width as usize;
        let sh = self.height as usize;

        for y in 0..sh {
            for x in 0..sw {
                let src_px =
                    self.pixels[self.origin + y * self.stride + x];

                let dx = y;
                let dy = sw - 1 - x;

                out[dy * (dst_w as usize) + dx] = src_px;
            }
        }

        FrameImage {
            width: dst_w,
            height: dst_h,
            stride: dst_w as usize,
            pixels: out.into_boxed_slice(),
        }
    }

    pub fn rotate_degrees(&self, degrees: f32, background: u32) -> FrameImage {
        if self.width <= 0 || self.height <= 0 {
            return FrameImage::empty();
        }

        let rad = degrees.to_radians();
        let sin = rad.sin();
        let cos = rad.cos();

        let sw = self.width as f32;
        let sh = self.height as f32;
        let cx = (sw - 1.0) * 0.5;
        let cy = (sh - 1.0) * 0.5;

        // Rotate the four corners to determine bounds.
        let corners = [
            (-cx, -cy),
            (sw - 1.0 - cx, -cy),
            (-cx, sh - 1.0 - cy),
            (sw - 1.0 - cx, sh - 1.0 - cy),
        ];

        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for (x, y) in corners {
            let rx = x * cos - y * sin;
            let ry = x * sin + y * cos;
            min_x = min_x.min(rx);
            max_x = max_x.max(rx);
            min_y = min_y.min(ry);
            max_y = max_y.max(ry);
        }

        let dst_w = (max_x - min_x).ceil().max(1.0) as i32;
        let dst_h = (max_y - min_y).ceil().max(1.0) as i32;
        let mut out = vec![background; (dst_w as usize) * (dst_h as usize)];

        let dcx = (dst_w as f32 - 1.0) * 0.5;
        let dcy = (dst_h as f32 - 1.0) * 0.5;

        let sw_u = self.width as usize;
        let sh_u = self.height as usize;
        let _ = (sw_u, sh_u);

        for dy in 0..dst_h {
            for dx in 0..dst_w {
                let x = dx as f32 - dcx;
                let y = dy as f32 - dcy;

                // Map destination -> source (inverse rotation)
                let src_x = x * cos + y * sin + cx;
                let src_y = -x * sin + y * cos + cy;

                let sx = src_x.round() as i32;
                let sy = src_y.round() as i32;

                if sx >= 0 && sx < self.width && sy >= 0 && sy < self.height {
                    let src_px = self.pixels[self.origin + (sy as usize) * self.stride + (sx as usize)];
                    out[(dy as usize) * (dst_w as usize) + (dx as usize)] = src_px;
                }
            }
        }

        FrameImage {
            width: dst_w,
            height: dst_h,
            stride: dst_w as usize,
            pixels: out.into_boxed_slice(),
        }
    }

    pub fn flip_horizontal(&self) -> FrameImage {
        if self.width <= 0 || self.height <= 0 {
            return FrameImage::empty();
        }

        let w = self.width as usize;
        let h = self.height as usize;

        let mut out = vec![0u32; w * h];

        for y in 0..h {
            let src_row = self.origin + y * self.stride;
            let dst_row = y * w;

            for x in 0..w {
                out[dst_row + x] =
                    self.pixels[src_row + (w - 1 - x)];
            }
        }

        FrameImage {
            width: self.width,
            height: self.height,
            stride: w,
            pixels: out.into_boxed_slice(),
        }
    }

    pub fn flip_vertical(&self) -> FrameImage {
        if self.width <= 0 || self.height <= 0 {
            return FrameImage::empty();
        }

        let w = self.width as usize;
        let h = self.height as usize;

        let mut out = vec![0u32; w * h];

        for y in 0..h {
            let src_row = self.origin + y * self.stride;
            let dst_row = (h - 1 - y) * w;

            out[dst_row..dst_row + w]
                .copy_from_slice(
                    &self.pixels[src_row..src_row + w]
                );
        }

        FrameImage {
            width: self.width,
            height: self.height,
            stride: w,
            pixels: out.into_boxed_slice(),
        }
    }
}

// ------------------------------
// ImageSource trait
// ------------------------------

pub trait ImageSource {
    fn width(&self) -> i32;
    fn height(&self) -> i32;
    fn stride(&self) -> usize; // in pixels
    fn pixels(&self) -> &[u32];
    fn origin(&self) -> usize {
        0
    }

    #[inline]
    fn view(&self) -> ImageView<'_> {
        ImageView {
            width: self.width(),
            height: self.height(),
            stride: self.stride(),
            pixels: self.pixels(),
            origin: self.origin(),
        }
    }


    #[inline]
    fn frame(&self) -> FrameImage {
        self.view().to_owned()
    }

    #[inline]
    fn crop(&self, x: i32, y: i32, w: i32, h: i32) -> Option<ImageView<'_>> {
        self.view().crop(x, y, w, h)
    }

    #[inline]
    fn resize_nearest(&self, dst_w: i32, dst_h: i32) -> FrameImage {
        self.view().resize_nearest(dst_w, dst_h)
    }

    #[inline]
    fn rotate_90_cw(&self) -> FrameImage {
        self.view().rotate_90_cw()
    }

    #[inline]
    fn rotate_90_ccw(&self) -> FrameImage {
        self.view().rotate_90_ccw()
    }

    #[inline]
    fn rotate_degrees(&self, degrees: f32, background: u32) -> FrameImage {
        self.view().rotate_degrees(degrees, background)
    }

    #[inline]
    fn flip_horizontal(&self) -> FrameImage {
        self.view().flip_horizontal()
    }

    #[inline]
    fn flip_vertical(&self) -> FrameImage {
        self.view().flip_vertical()
    }
    #[inline]
    fn to_owned(&self) -> FrameImage {
        self.frame()
    }

}

impl ImageSource for FrameImage {
    fn width(&self) -> i32 { self.width }
    fn height(&self) -> i32 { self.height }
    fn stride(&self) -> usize { self.stride }
    fn pixels(&self) -> &[u32] { &self.pixels }
}

impl<'a> ImageSource for ImageView<'a> {
    fn width(&self) -> i32 { self.width }
    fn height(&self) -> i32 { self.height }
    fn stride(&self) -> usize { self.stride }
    fn pixels(&self) -> &[u32] { self.pixels }
    fn origin(&self) -> usize { self.origin }
}

// ------------------------------
// Capture backend
// ------------------------------

pub struct CaptureSession {
    rect: Rect,
    screen_dc: HDC,
    mem_dc: HDC,
    dib: HBITMAP,
    old_obj: HGDIOBJ,
    bits: *mut u32,
    include_layered_windows: bool,
}

impl Drop for CaptureSession {
    fn drop(&mut self) {
        unsafe {
            if !self.mem_dc.is_null() && !self.old_obj.is_null() {
                let _ = SelectObject(self.mem_dc, self.old_obj);
            }
            if !self.dib.is_null() {
                let _ = DeleteObject(self.dib as HGDIOBJ);
            }
            if !self.mem_dc.is_null() {
                let _ = DeleteDC(self.mem_dc);
            }
            if !self.screen_dc.is_null() {
                let _ = ReleaseDC(null_mut(), self.screen_dc);
            }
        }
    }
}

impl CaptureSession {
    pub fn new() -> Option<Self> {
        unsafe {SetProcessDPIAware();}
        Self::with_rect(Rect::virtual_screen()?, true)
    }

    pub fn with_rect(rect: Rect, include_layered_windows: bool) -> Option<Self> {
        unsafe {
            let _ = SetProcessDPIAware();

            let screen_dc = GetDC(null_mut());
            if screen_dc.is_null() {
                return None;
            }

            let mem_dc = CreateCompatibleDC(screen_dc);
            if mem_dc.is_null() {
                let _ = ReleaseDC(null_mut(), screen_dc);
                return None;
            }

            let mut bmi: BITMAPINFO = zeroed();
            bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = rect.width;
            bmi.bmiHeader.biHeight = -rect.height;
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = BI_RGB;

            let mut bits: *mut c_void = null_mut();
            let dib = CreateDIBSection(
                screen_dc,
                &bmi,
                DIB_RGB_COLORS,
                &mut bits,
                null_mut(),
                0,
            );

            if dib.is_null() || bits.is_null() {
                let _ = DeleteDC(mem_dc);
                let _ = ReleaseDC(null_mut(), screen_dc);
                return None;
            }

            let old_obj = SelectObject(mem_dc, dib as HGDIOBJ);
            if old_obj.is_null() {
                let _ = DeleteObject(dib as HGDIOBJ);
                let _ = DeleteDC(mem_dc);
                let _ = ReleaseDC(null_mut(), screen_dc);
                return None;
            }

            Some(Self {
                rect,
                screen_dc,
                mem_dc,
                dib,
                old_obj,
                bits: bits as *mut u32,
                include_layered_windows,
            })
        }
    }

    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Captures the session rect into the persistent DIB.
    /// No allocation happens here.
    pub fn capture(&mut self) -> Option<ImageView<'_>> {
        unsafe {
            let rop = if self.include_layered_windows {
                SRCCOPY | CAPTUREBLT
            } else {
                SRCCOPY
            };

            let ok = BitBlt(
                self.mem_dc,
                0,
                0,
                self.rect.width,
                self.rect.height,
                self.screen_dc,
                self.rect.x,
                self.rect.y,
                rop,
            ) != 0;

            if !ok {
                return None;
            }

            let len = (self.rect.width as usize) * (self.rect.height as usize);
            let pixels = slice::from_raw_parts(self.bits as *const u32, len);

            Some(ImageView {
                width: self.rect.width,
                height: self.rect.height,
                stride: self.rect.width as usize,
                pixels,
                origin:0
            })
        }
    }

    /// Convenience if you need ownership.
    pub fn capture_owned(&mut self) -> Option<FrameImage> {
        self.capture().map(|f| f.to_owned())
    }
}

// ------------------------------
// Transform helpers
// ------------------------------

fn resize_nearest_impl(
    pixels: &[u32],
    src_w: i32,
    src_h: i32,
    src_stride: usize,
    origin: usize,
    dst_w: i32,
    dst_h: i32,
) -> FrameImage {
    if src_w <= 0 || src_h <= 0 || dst_w <= 0 || dst_h <= 0 {
        return FrameImage::empty();
    }

    let mut out = vec![0u32; (dst_w as usize) * (dst_h as usize)];
    let step_x = ((src_w as i64) << 16) / (dst_w as i64);
    let step_y = ((src_h as i64) << 16) / (dst_h as i64);

    for dy in 0..dst_h {
        let sy = (((dy as i64) * step_y) >> 16).clamp(0, (src_h - 1) as i64) as usize;
        let src_row = origin + sy * src_stride;
        let dst_row = (dy as usize) * (dst_w as usize);

        let mut sx_fp = 0i64;
        for dx in 0..dst_w {
            let sx = (sx_fp >> 16).clamp(0, (src_w - 1) as i64) as usize;
            out[dst_row + (dx as usize)] = pixels[src_row + sx];
            sx_fp += step_x;
        }
    }

    FrameImage {
        width: dst_w,
        height: dst_h,
        stride: dst_w as usize,
        pixels: out.into_boxed_slice(),
    }
}





// ------------------------------
// Loading macros
// ------------------------------

#[macro_export]
macro_rules! load_image {
    ($path:expr) => {{
        $crate::overlay::FrameImage::from_path($path)
            .unwrap_or_else(|e| panic!("failed to load image {}: {}", $path, e))
    }};
}

#[macro_export]
macro_rules! include_image {
    ($path:expr) => {{
        $crate::overlay::capture::FrameImage::from_bytes(include_bytes!($path))
            .unwrap_or_else(|e| panic!("failed to decode embedded image {}: {}", $path, e))
    }};
}


