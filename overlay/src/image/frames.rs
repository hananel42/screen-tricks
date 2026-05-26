//! # Image Processing and Manipulation Module
//!
//! This module provides facilities for representing, viewing, and transforming 2D image buffers.
//! It is built around efficient pixel manipulation using 32-bit unsigned integers (`u32`)
//! representing colors in **premultiplied RGBA** format.
//!
//! ## Core Components
//!
//! * [`FrameImage`]: An owned, heap-allocated 2D image buffer.
//! * [`ImageView`]: A lightweight, non-owning structural view into a sub-region or an entire pixel buffer, supporting zero-copy operations.
//! * [`ImageSource`]: A unified trait implemented by both `FrameImage` and `ImageView` exposing common geometric transformation interfaces.

use crate::image::common::{Color, premul_rgba_bytes_to_u32, rgba_premul};

/// Represents an owned frame image with width, height, and pixel data.
///
/// This struct encapsulates a contiguous 2D image frame allocated on the heap.
/// The pixel data is stored in row-major order.
///
/// # Fields
/// * `width`: The width of the image in pixels.
/// * `height`: The height of the image in pixels.
/// * `stride`: The number of pixels per row in the pixel buffer, which may include padding.
/// * `pixels`: A boxed slice of 32-bit unsigned integers containing the premultiplied RGBA pixels.
#[derive(Clone)]
pub struct FrameImage {
    pub(super) width: i32,
    pub(super) height: i32,
    pub(super) stride: usize, // in pixels
    pub(super) pixels: Box<[u32]>,
}

impl FrameImage {
    /// Creates a new empty image with zero dimensions and an empty pixel buffer.
    ///
    /// # Returns
    ///
    /// A new `Self` instance representing an empty canvas.
    pub fn empty() -> Self {
        Self {
            width: 0,
            height: 0,
            stride: 0,
            pixels: Box::new([]),
        }
    }

    /// Creates a new filled image with the specified width, height, and background color.
    ///
    /// # Arguments
    ///
    /// * `width` - The width of the image in pixels. Must be greater than zero.
    /// * `height` - The height of the image in pixels. Must be greater than zero.
    /// * `color` - The background color to fill the image with. Internally converted to premultiplied alpha format.
    ///
    /// # Returns
    ///
    /// * `Some(FrameImage)` - If the dimensions are valid.
    /// * `None` - If either `width` or `height` is less than or equal to zero.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use overlay::image::FrameImage;
    /// let image = FrameImage::filled(100, 100, (0, 0, 0, 255));
    /// assert!(image.is_some());
    /// ```
    pub fn filled(width: i32, height: i32, color: Color) -> Option<Self> {
        if width <= 0 || height <= 0 {
            return None;
        }

        let color = rgba_premul(color);
        let len = (width as usize) * (height as usize);

        Some(Self {
            width,
            height,
            stride: width as usize,
            pixels: vec![color; len].into_boxed_slice(),
        })
    }

    /// Creates a new image from an existing vector of raw premultiplied pixels.
    ///
    /// # Arguments
    ///
    /// * `width` - The width of the image in pixels. Must be greater than zero.
    /// * `height` - The height of the image in pixels. Must be greater than zero.
    /// * `pixels` - A vector of `u32` raw values representing premultiplied pixel data.
    ///
    /// # Returns
    ///
    /// * `Some(FrameImage)` - If dimensions are valid and match the provided pixel vector length exactly.
    /// * `None` - If dimensions are non-positive, or if `width * height != pixels.len()`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use overlay::image::FrameImage;
    /// let pixels = vec![0xFF0000FF, 0xFF00FF00, 0xFFFF0000, 0xFFFF0000];
    /// let image = FrameImage::from_raw_premultiplied(2, 2, pixels);
    /// assert!(image.is_some());
    /// ```
    pub fn from_raw_premultiplied(width: i32, height: i32, pixels: Vec<u32>) -> Option<Self> {
        if width <= 0 || height <= 0 {
            return None;
        }
        let expected = (width as usize) * (height as usize);
        if pixels.len() != expected {
            return None;
        }
        Some(Self {
            width,
            height,
            stride: width as usize,
            pixels: pixels.into_boxed_slice(),
        })
    }

    /// Creates a new image from a slice of raw, straight RGBA bytes, converting them to premultiplied alpha.
    ///
    /// # Arguments
    ///
    /// * `width` - The width of the image in pixels. Must be greater than zero.
    /// * `height` - The height of the image in pixels. Must be greater than zero.
    /// * `rgba_bytes` - A contiguous byte slice containing flat `[R, G, B, A]` layout data.
    ///
    /// # Returns
    ///
    /// * `Some(FrameImage)` - If dimensions are valid and `rgba_bytes.len() == width * height * 4`.
    /// * `None` - Otherwise.
    ///
    /// # Notes
    ///
    /// The input bytes are parsed sequentially into chunks of 4. Each chunk is processed into a
    /// single `u32` with premultiplied alpha channels for performance optimizations during rendering.
    pub fn from_bytes_rgba(width: i32, height: i32, rgba_bytes: &[u8]) -> Option<Self> {
        if width <= 0 || height <= 0 {
            return None;
        }
        let expected = (width as usize) * (height as usize) * 4;
        if rgba_bytes.len() != expected {
            return None;
        }

        let mut out = vec![0u32; (width as usize) * (height as usize)];
        for (i, px) in rgba_bytes.chunks_exact(4).enumerate() {
            out[i] = premul_rgba_bytes_to_u32(px);
        }

        Some(Self {
            width,
            height,
            stride: width as usize,
            pixels: out.into_boxed_slice(),
        })
    }

    /// Returns a shared slice reference to the underlying flat array of pixel values.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use overlay::image::FrameImage;
    /// let image = FrameImage::from_raw_premultiplied(2, 2, vec![1, 2, 3, 4]).unwrap();
    /// let slice = image.as_slice();
    /// assert_eq!(slice[2], 3);
    /// ```
    #[inline]
    pub fn as_slice(&self) -> &[u32] {
        &self.pixels
    }

    /// Returns a mutable slice reference to the underlying flat array of pixel values.
    ///
    /// This allows direct in-place mutation of pixel buffers for high-performance filters.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use overlay::image::FrameImage;
    /// let mut image = FrameImage::filled(10, 10, (0, 0, 255, 255)).unwrap();
    /// let pixels = image.as_mut_slice();
    /// pixels[0] = 0xFFFFFFFF; // Mutate first pixel in-place
    /// ```
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u32] {
        &mut self.pixels
    }

    /// Creates a lightweight, structural [`ImageView`] referencing this image's pixel data.
    ///
    /// Allows invoking zero-copy cropping or transformations over shared views.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use overlay::image::{FrameImage, ImageSource};
    /// let image = FrameImage::filled(100, 100, (42, 42, 42, 42)).unwrap();
    /// let view = image.view();
    /// assert_eq!(view.width(), 100);
    /// ```
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

/// A lightweight, non-owning reference view into an image buffer.
///
/// `ImageView` enables zero-copy image manipulations such as sub-region slicing (cropping)
/// by keeping track of a structural `origin` offset and a row `stride` across a borrowed data buffer.
#[derive(Clone, Copy)]
pub struct ImageView<'a> {
    pub(super) width: i32,
    pub(super) height: i32,
    pub(super) stride: usize, // in pixels, stride of the underlying source buffer
    pub(super) pixels: &'a [u32],
    pub(super) origin: usize, // pixel offset from pixels[0]
}

impl<'a> ImageView<'a> {
    /// Internal crop implementation. Returns a subview bounding box within the current view constraints.
    fn crop(&self, x: i32, y: i32, w: i32, h: i32) -> Option<ImageView<'a>> {
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

    /// Allocates and constructs an owned fully deep-copied [`FrameImage`] out of this view.
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

    /// Resizes the structural view into a new owned [`FrameImage`] via standard nearest-neighbor scaling.
    fn resize_nearest(&self, dst_w: i32, dst_h: i32) -> FrameImage {
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

    /// Rotates the view 90 degrees clockwise, producing a new owned [`FrameImage`].
    fn rotate_90_cw(&self) -> FrameImage {
        if self.width <= 0 || self.height <= 0 {
            return FrameImage::empty();
        }

        let dst_w = self.height;
        let dst_h = self.width;
        let mut out = vec![0u32; (dst_w as usize) * (dst_h as usize)];

        let sw = self.width as usize;
        let sh = self.height as usize;

        for y in 0..sh {
            for x in 0..sw {
                let src_px = self.pixels[self.origin + y * self.stride + x];
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

    /// Rotates the view 90 degrees counterclockwise, producing a new owned [`FrameImage`].
    fn rotate_90_ccw(&self) -> FrameImage {
        if self.width <= 0 || self.height <= 0 {
            return FrameImage::empty();
        }

        let dst_w = self.height;
        let dst_h = self.width;
        let mut out = vec![0u32; (dst_w as usize) * (dst_h as usize)];

        let sw = self.width as usize;
        let sh = self.height as usize;

        for y in 0..sh {
            for x in 0..sw {
                let src_px = self.pixels[self.origin + y * self.stride + x];
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

    /// Rotates the structural view arbitrarily by specified degrees, returning a new bounding-box sized [`FrameImage`].
    fn rotate_degrees(&self, degrees: f32, background: Color) -> FrameImage {
        if self.width <= 0 || self.height <= 0 {
            return FrameImage::empty();
        }
        let background = rgba_premul(background);
        let rad = degrees.to_radians();
        let sin = rad.sin();
        let cos = rad.cos();

        let sw = self.width as f32;
        let sh = self.height as f32;
        let cx = (sw - 1.0) * 0.5;
        let cy = (sh - 1.0) * 0.5;

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

        for dy in 0..dst_h {
            for dx in 0..dst_w {
                let x = dx as f32 - dcx;
                let y = dy as f32 - dcy;

                let src_x = x * cos + y * sin + cx;
                let src_y = -x * sin + y * cos + cy;

                let sx = src_x.round() as i32;
                let sy = src_y.round() as i32;

                if sx >= 0 && sx < self.width && sy >= 0 && sy < self.height {
                    let src_px =
                        self.pixels[self.origin + (sy as usize) * self.stride + (sx as usize)];
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

    /// Flips the frame horizontally, returning a newly allocated owned [`FrameImage`].
    fn flip_horizontal(&self) -> FrameImage {
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
                out[dst_row + x] = self.pixels[src_row + (w - 1 - x)];
            }
        }

        FrameImage {
            width: self.width,
            height: self.height,
            stride: w,
            pixels: out.into_boxed_slice(),
        }
    }

    /// Flips the frame vertically, returning a newly allocated owned [`FrameImage`].
    fn flip_vertical(&self) -> FrameImage {
        if self.width <= 0 || self.height <= 0 {
            return FrameImage::empty();
        }

        let w = self.width as usize;
        let h = self.height as usize;
        let mut out = vec![0u32; w * h];

        for y in 0..h {
            let src_row = self.origin + y * self.stride;
            let dst_row = (h - 1 - y) * w;

            out[dst_row..dst_row + w].copy_from_slice(&self.pixels[src_row..src_row + w]);
        }

        FrameImage {
            width: self.width,
            height: self.height,
            stride: w,
            pixels: out.into_boxed_slice(),
        }
    }
}

/// A trait specifying the core reading interface and transformations for 2D image abstractions.
///
/// Implementors can safely expose dimensions, continuous raw pixels slice bounds, structural subviews,
/// or allocate geometric variations across buffers.
pub trait ImageSource {
    /// Returns the width of the image in pixels.
    fn width(&self) -> i32;

    /// Returns the height of the image in pixels.
    fn height(&self) -> i32;

    /// Returns the line stride of the buffer **measured in pixels** (not raw bytes).
    fn stride(&self) -> usize;

    /// Returns a shared slice reference referencing the raw flat u32 pixel colors buffer.
    fn pixels(&self) -> &[u32];

    /// Returns the data container entry offset index (primarily utilized by non-zero structural [`ImageView`] slices).
    fn origin(&self) -> usize {
        0
    }

    /// Instantiates an immutable zero-copy structural [`ImageView`] representation over this data source.
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

    /// Deep-copies data out of the current layout reference, creating a fully allocated owned [`FrameImage`].
    #[inline]
    fn frame(&self) -> FrameImage {
        self.view().to_owned()
    }

    /// Creates a scoped rectangular structural subview bounded inside the current geometry.
    ///
    /// # Returns
    /// * `Some(ImageView)` - If the bounded box dimensions are correctly aligned.
    /// * `None` - If coordinates escape container boundaries.
    #[inline]
    fn crop(&self, x: i32, y: i32, w: i32, h: i32) -> Option<ImageView<'_>> {
        self.view().crop(x, y, w, h)
    }

    /// Scales the image dimensions using a fast nearest-neighbor algorithm, outputting an owned [`FrameImage`].
    #[inline]
    fn resize_nearest(&self, dst_w: i32, dst_h: i32) -> FrameImage {
        self.view().resize_nearest(dst_w, dst_h)
    }

    /// Rotates the layout view 90 degrees clockwise, outputting a separate owned [`FrameImage`].
    #[inline]
    fn rotate_90_cw(&self) -> FrameImage {
        self.view().rotate_90_cw()
    }

    /// Rotates the layout view 90 degrees counterclockwise, outputting a separate owned [`FrameImage`].
    #[inline]
    fn rotate_90_ccw(&self) -> FrameImage {
        self.view().rotate_90_ccw()
    }

    /// Rotates the frame image layout by an arbitrary angle value around its bounding box center.
    /// Empty padding areas are filled using the provided `background` parameter.
    ///
    /// # Arguments
    /// * `degrees` - Angle displacement value. Positive values trigger counterclockwise rotation.
    /// * `background` - Background placeholder color filled into vacant border frames.
    #[inline]
    fn rotate_degrees(&self, degrees: f32, background: Color) -> FrameImage {
        self.view().rotate_degrees(degrees, background)
    }

    /// Inverts the image orientation along its vertical axis line, returning a clean owned [`FrameImage`].
    #[inline]
    fn flip_horizontal(&self) -> FrameImage {
        self.view().flip_horizontal()
    }

    /// Inverts the image orientation along its horizontal axis line, returning a clean owned [`FrameImage`].
    #[inline]
    fn flip_vertical(&self) -> FrameImage {
        self.view().flip_vertical()
    }
}

impl ImageSource for FrameImage {
    fn width(&self) -> i32 {
        self.width
    }
    fn height(&self) -> i32 {
        self.height
    }
    fn stride(&self) -> usize {
        self.stride
    }
    fn pixels(&self) -> &[u32] {
        &self.pixels
    }
}

impl<'a> ImageSource for ImageView<'a> {
    fn width(&self) -> i32 {
        self.width
    }
    fn height(&self) -> i32 {
        self.height
    }
    fn stride(&self) -> usize {
        self.stride
    }
    fn pixels(&self) -> &[u32] {
        self.pixels
    }
    fn origin(&self) -> usize {
        self.origin
    }
}
