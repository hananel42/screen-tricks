

use crate::image::common::{premul_rgba_bytes_to_u32, rgba_premul, Color};

/// Represents a frame image with width, height, and pixel data.
///
/// This struct encapsulates a 2D image frame with dimensions and pixel data.
///
/// Fields:
/// * `width`: The width of the image in pixels (i32).
/// * `height`: The height of the image in pixels (i32).
/// * `stride`: The number of pixels per row in the pixel buffer, which may differ from the width due to padding (usize).
/// * `pixels`: A boxed slice of 32-bit unsigned integers representing the pixel data. Each pixel value is typically interpreted as a 32-bit color value (RGBA).
///
/// Note: The stride field allows for non-standard row alignment, which is useful for optimized memory access or when working with formats that require padding.
///
///
/// This struct is designed to be cloned, enabling efficient copying of image data in memory-intensive operations.
#[derive(Clone)]
pub struct FrameImage {
    pub(super) width: i32,
    pub(super) height: i32,
    pub(super) stride: usize, // in pixels
    pub(super) pixels: Box<[u32]>,
}

impl FrameImage {
    /// Creates a new empty image with zero dimensions and no pixels.
    ///
    /// This function returns an image instance with width, height, and stride all set to 0,
    /// and an empty pixel buffer. The resulting image has no pixels and is effectively
    /// an empty canvas.
    ///
    /// # Returns
    ///
    /// A new `Self` instance representing an empty image.
    pub fn empty() -> Self {
        Self {
            width: 0,
            height: 0,
            stride: 0,
            pixels: Box::new([]),
        }
    }
    /// Creates a new filled image with the specified width, height, and color.
    ///
    /// # Arguments
    ///
    /// * `width` - The width of the image in pixels. Must be positive.
    /// * `height` - The height of the image in pixels. Must be positive.
    /// * `color` - The color to fill the image with. The color is automatically converted to premultiplied alpha (premul) format.
    ///
    /// # Returns
    ///
    /// A `Result` containing the newly created image if dimensions are valid, otherwise an `ImageError`.
    ///
    /// # Errors
    ///
    /// Returns `ImageError::InvalidDimensions` if either `width` or `height` is less than or equal to zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use overlay::capture::FrameImage;
    /// let image = FrameImage::filled(100, 100, (0,0,0,255));
    /// assert!(image.is_ok());
    /// ```
    ///
    /// # Notes
    ///
    /// - The image's pixel data is stored in row-major order.
    /// - The stride is equal to the width in bytes, ensuring proper row alignment.
    /// - The color is converted to premultiplied alpha format internally to ensure correct blending behavior.
    pub fn filled(width: i32, height: i32, color: Color) -> Option<Self> {
        if width <= 0 || height <= 0 {
            return None
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
    /// Creates a new image from raw premultiplied pixels.
    ///
    /// This function constructs an image from a vector of raw 32-bit unsigned integer pixels,
    /// where each pixel is assumed to be premultiplied (i.e., alpha is already combined with RGB).
    ///
    /// # Arguments
    ///
    /// * `width` - The width of the image in pixels. Must be positive.
    /// * `height` - The height of the image in pixels. Must be positive.
    /// * `pixels` - A vector of `u32` values representing the raw pixel data.
    ///
    /// # Returns
    ///
    /// A `Result<Self, ImageError>` where:
    /// * `Ok(image)` if the dimensions and pixel count match the expected size.
    /// * `Err(ImageError::InvalidDimensions)` if the width or height is non-positive,
    ///   or if the number of pixels does not match `width * height`.
    ///
    /// # Examples
    ///
    /// ```rust
    ///
    /// use overlay::capture::FrameImage;
    /// let pixels = vec![0xFF0000FF, 0xFF00FF00, 0xFFFF0000, 0xFFFF0000];
    /// let image = FrameImage::from_raw_premultiplied(2, 2, pixels);
    /// assert!(image.is_ok());
    /// ```
    pub fn from_raw_premultiplied(
        width: i32,
        height: i32,
        pixels: Vec<u32>,
    ) -> Option<Self> {
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

    /// Creates a new `Self` instance from a slice of RGBA bytes.
    ///
    /// This function interprets a byte array containing RGBA pixel data and converts it into a
    /// pixel buffer with premultiplied alpha. The input must be exactly `width * height * 4` bytes
    /// long, with each pixel represented as 4 bytes (R, G, B, A).
    ///
    /// # Arguments
    ///
    /// * `width` - The width of the image in pixels. Must be positive.
    /// * `height` - The height of the image in pixels. Must be positive.
    /// * `rgba_bytes` - A slice of bytes containing RGBA pixel data in row-major order.
    ///
    /// # Returns
    ///
    /// A `Result` containing the newly created image instance, or an `ImageError` if:
    /// - `width` or `height` is non-positive.
    /// - The length of `rgba_bytes` does not match `width * height * 4`.
    ///
    /// # Notes
    ///
    /// - The input byte array is assumed to be in row-major order, with each pixel represented
    ///   as `[R, G, B, A]` (each component 8 bits).
    /// - The resulting pixel values are stored as premultiplied RGBA (`premul_rgba`) in a 32-bit
    ///   integer format (i.e., `R * A / 255`, etc.).
    /// - The output image has a stride equal to the width in pixels, and the pixel data is stored
    ///   in a boxed slice for efficient memory access.
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

    /// Returns a slice referencing the underlying array of pixel values as `u32`.
    ///
    /// This method provides direct access to the raw pixel data stored in the structure.
    /// The returned slice has the same lifetime as the current instance.
    ///
    /// # Examples
    ///
    /// ```rust
    ///use overlay::capture::FrameImage;
    /// let image = FrameImage::from_raw_premultiplied(2,2,(&[1,2,3,4]).to_vec()).unwrap();
    /// let slice = image.as_slice();
    /// assert_eq!(slice[2], 3);
    /// ```
    #[inline]
    pub fn as_slice(&self) -> &[u32] {
        &self.pixels
    }

    /// Returns a mutable reference to the underlying array of `u32` values that
    /// represents the pixel data.
    ///
    /// This method allows direct mutation of the pixel data through a slice
    /// reference, enabling efficient operations such as pixel manipulation or
    /// image processing.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use overlay::capture::{FrameImage, Color};
    /// let mut image = FrameImage::filled(100,100,(0,0,255,255)).expect("Cannot create image");
    /// let pixels = image.as_mut_slice();
    /// pixels[0] = 255; // Set the first pixel to white
    /// ```
    ///
    /// # Safety
    ///
    /// The returned slice refers to the internal `pixels` field of the struct.
    /// It is valid to use this reference only if the struct instance is not
    /// being moved or mutated in a way that would invalidate the reference.
    ///
    /// # Notes
    ///
    /// This method is marked `#[inline]` to optimize performance by avoiding
    /// function call overhead. It is intended to be used frequently in pixel
    /// manipulation scenarios.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u32] {
        &mut self.pixels
    }

    /// Creates an `ImageView` that provides a shared view into the pixel data of this image.
    ///
    /// This method returns a new `ImageView` instance that wraps a slice of the original pixel data,
    /// allowing safe and efficient access to the image's pixels without copying.
    ///
    /// The returned `ImageView` has the same dimensions and stride as the original image,
    /// and the pixel data is accessed through a reference to the original buffer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the original image data is not dropped or modified while
    /// the `ImageView` is in use, as it references the original pixel buffer.
    ///
    /// # Example
    ///
    /// ```rust
    /// use overlay::capture::{FrameImage,ImageView,ImageSource};
    /// let image = FrameImage::filled(100,100,(42,42,42,42)).unwrap();
    /// let view = image.view();
    /// assert_eq!(view.width(), 100);
    /// assert_eq!(view.height(), 100);
    /// ```
    ///
    /// # Panics
    ///
    /// This method does not panic. It will always return a valid `ImageView`.
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

/// A view into a buffer of 32-bit unsigned integers representing image pixels.
///
/// This struct provides a safe and efficient way to access pixel data from a source buffer,
/// with support for arbitrary strides and origin offsets. It is designed to be used with
/// image data that may not be stored in a row-major format or may start at a non-zero offset.
#[derive(Clone, Copy)]
pub struct ImageView<'a> {
    pub(super) width: i32,
    pub(super) height: i32,
    pub(super) stride: usize, // in pixels, stride of the underlying source buffer
    pub(super) pixels: &'a [u32],
    pub(super) origin: usize, // pixel offset from pixels[0]
}

impl<'a> ImageView<'a> {
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

// ------------------------------
// ImageSource trait
// ------------------------------

/// `ImageSource` is a trait that defines the core interface for any image data source.
///
/// It provides methods to access the image's dimensions, pixel data, and various image transformations.
/// All operations are performed on a view of the raw pixel data, with methods that return new `FrameImage`
/// instances to ensure the original image remains unmodified.
///
/// # Key Features
///
/// - `width()` and `height()` return the image dimensions in pixels.
/// - `stride()` returns the byte stride of the pixel buffer (used for alignment or padding).
/// - `pixels()` returns a slice to the raw pixel data, where each pixel is represented as a 32-bit unsigned integer.
/// - `origin()` returns the starting index of the image data in the buffer (default is 0).
///
/// # Transformations
///
/// The trait supports common image operations:
/// - `view()`: Returns an `ImageView` that provides a safe, immutable view into the pixel data.
/// - `frame()`: Returns a fully owned `FrameImage` copy of the current image.
/// - `crop()`: Creates a new `ImageView` representing a cropped region of the image (returns `None` if bounds are invalid).
/// - `resize_nearest()`: Resizes the image using nearest-neighbor interpolation.
/// - `rotate_90_cw()` and `rotate_90_ccw()`: Rotate the image 90 degrees clockwise or counterclockwise.
/// - `rotate_degrees()`: Rotates the image by a specified number of degrees around its center, with optional background filling.
/// - `flip_horizontal()` and `flip_vertical()`: Flip the image along the horizontal or vertical axis.
///
/// # Safety and Ownership
///
/// All transformations return new `FrameImage` instances. The original image is not modified.
/// The `pixels()` method returns a slice, so it is valid only as long as the source data remains valid.
///
/// # Example Usage
///
/// ```rust
/// use overlay::capture::{FrameImage,ImageSource};
/// let image = FrameImage::filled(100,42,(255,255,0,24)).unwrap();
/// let view = image.view();
/// let rotated = image.rotate_90_cw();
/// let cropped = image.crop(10, 10, 50, 50);
/// ```
///
/// # Notes
///
/// - The pixel format is assumed to be 32-bit unsigned integers (u32), typically representing RGBA values.
/// - All rotation and flip operations are performed around the image's center, with background filling where appropriate.
/// - The `Color` type is assumed to be u32 representing an RGBA value.
pub trait ImageSource {
    /// Returns the width of the object as an i32 value.
    fn width(&self) -> i32;
    /// Returns the height of the object as an i32 value.
    fn height(&self) -> i32;
    /// Returns the stride of the buffer.
    fn stride(&self) -> usize;

    /// Returns a slice referencing the raw pixel data of the image.
    fn pixels(&self) -> &[u32];
    /// Returns the origin index of the image (used for ImageView).
    fn origin(&self) -> usize {
        0
    }

    /// Creates an `ImageView` that provides a view into the pixel data of this image,
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

    /// Returns a copy of the current image as a `FrameImage`.
    #[inline]
    fn frame(&self) -> FrameImage {
        self.view().to_owned()
    }

    /// Creates a new `ImageView` that represents a cropped region of the original image.
    #[inline]
    fn crop(&self, x: i32, y: i32, w: i32, h: i32) -> Option<ImageView<'_>> {
        self.view().crop(x, y, w, h)
    }

    /// Resizes the image using nearest-neighbor interpolation.
    #[inline]
    fn resize_nearest(&self, dst_w: i32, dst_h: i32) -> FrameImage {
        self.view().resize_nearest(dst_w, dst_h)
    }

    /// Rotates the frame image 90 degrees clockwise.
    ///
    /// This method returns a new `FrameImage` that represents the original image rotated
    /// 90 degrees clockwise. The rotation is performed on the view of the current frame,
    /// and the original image remains unmodified.
    ///
    /// # Returns
    ///
    /// A new `FrameImage` object with the 90-degree clockwise rotation applied.
    #[inline]
    fn rotate_90_cw(&self) -> FrameImage {
        self.view().rotate_90_cw()
    }

    /// Rotates the image 90 degrees counterclockwise (CCW) around its center.
    ///
    /// This method returns a new `FrameImage` that represents the original image rotated
    /// 90 degrees counterclockwise. The rotation is performed around the image's center,
    /// preserving the original image and returning a separate instance of the rotated image.
    ///
    /// # Returns
    ///
    /// A new `FrameImage` object containing the rotated image.
    #[inline]
    fn rotate_90_ccw(&self) -> FrameImage {
        self.view().rotate_90_ccw()
    }

    /// Rotates the frame image by a specified number of degrees around its center,
    /// with a background color filled behind the rotated image.
    ///
    /// This function applies a rotation transformation to the current frame image,
    /// using the specified degrees to rotate the image counterclockwise around its center.
    /// The background color is used to fill the area outside the rotated image.
    ///
    /// # Arguments
    ///
    /// * `degrees` - The number of degrees to rotate the image. Positive values rotate
    ///               counterclockwise, negative values rotate clockwise.
    /// * `background` - The background color (as a 32-bit unsigned integer) used to fill
    ///                  the area outside the rotated image.
    ///
    /// # Returns
    ///
    /// A new `FrameImage` object representing the rotated image with the specified background.
    #[inline]
    fn rotate_degrees(&self, degrees: f32, background: Color) -> FrameImage {

        self.view().rotate_degrees(degrees, background)
    }

    /// Flips the image horizontally using the current view.
    ///
    /// This method creates a new `FrameImage` by flipping the current image along the horizontal axis.
    /// The original image is not modified; a new instance is returned with the flipped content.
    ///
    /// # Returns
    ///
    /// A new `FrameImage` with the content flipped horizontally.
    #[inline]
    fn flip_horizontal(&self) -> FrameImage {
        self.view().flip_horizontal()
    }

    /// Flips the image vertically (top to bottom) using the current view.
    ///
    /// This method creates a new `FrameImage` by flipping the current image along the vertical axis,
    /// effectively reversing the order of rows from top to bottom.
    ///
    /// # Returns
    ///
    /// A new `FrameImage` with the vertical flip applied.
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


