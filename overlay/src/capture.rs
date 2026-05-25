use std::{
    ffi::c_void,
    mem::{size_of, zeroed},
    path::Path,
    ptr::null_mut,
    slice,
};

use image::ImageReader;
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, CreateCompatibleDC, CreateDIBSection,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, HBITMAP, HDC, HGDIOBJ, ReleaseDC, SRCCOPY,
    SelectObject,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
    SetProcessDPIAware,
};

// ------------------------------
// Helpers
// ------------------------------
pub type Color = (u8, u8, u8, u8); // (r,g,b,a)
#[inline]
const fn rgba_premul(color: Color) -> u32 {
    let (r, g, b, a) = color;
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


/// A 2D rectangle defined by its top-left corner (x, y) and dimensions (width, height).
///
/// This struct represents a rectangle in a 2D coordinate system where the origin (0, 0) is typically
/// in the top-left corner. The rectangle is defined by its position and size, with the
/// top-left corner at (x, y) and extending right and down by width and height respectively.
///
/// # Fields
///
/// * `x` - The x-coordinate of the top-left corner of the rectangle.
/// * `y` - The y-coordinate of the top-left corner of the rectangle.
/// * `width` - The width of the rectangle (horizontal size).
/// * `height` - The height of the rectangle (vertical size).
///
/// # Examples
///
/// ```
/// use overlay::capture::Rect;
/// let rect = Rect { x: 10, y: 20, width: 100, height: 50 };
/// ```
///
/// This creates a rectangle positioned at (10, 20) with a width of 100 and a height of 5.0.
///
/// # Notes
///
/// All fields are of type `i32`, allowing for large coordinate values. The rectangle does not
/// perform bounds checking or validation on construction. It is the caller's responsibility to
/// ensure valid dimensions and positions.
#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    fn virtual_screen() -> Option<Self> {
        unsafe {
            let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);

            if width <= 0 || height <= 0 {
                None
            } else {
                Some(Self {
                    x,
                    y,
                    width,
                    height,
                })
            }
        }
    }
}

/// Error type representing various failure modes when handling images.
///
/// This enum encapsulates errors that can occur during image operations such as
/// reading, decoding, or validating image data. It provides a structured way to
/// handle different types of errors with clear error sources.
///
/// # Variants
///
/// * `Io(std::io::Error)`: An I/O error occurred while reading or writing image data.
/// * `Decode(image::ImageError)`: A decoding error occurred when parsing image data using the `image` crate.
/// * `InvalidDimensions`: The image dimensions are invalid (e.g., negative width/height).
/// * `Empty`: The image data is empty or has zero size.
///
/// # Example
///
/// ```rust,no_run,ignore
/// use overlay::capture::{ImageError,FrameImage};
/// use overlay::load_image;
///
/// match load_image!("path/to/image.png") {
///     Ok(img) => println!("Image loaded successfully"),
///     Err(ImageError::Io(e)) => eprintln!("I/O error: {}", e),
///     Err(ImageError::Decode(e)) => eprintln!("Decode error: {}", e),
///     Err(ImageError::InvalidDimensions) => eprintln!("Invalid image dimensions"),
///     Err(ImageError::Empty) => eprintln!("Image is empty"),
/// }
/// ```
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
    width: i32,
    height: i32,
    stride: usize, // in pixels
    pixels: Box<[u32]>,
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
    pub fn filled(width: i32, height: i32, color: Color) -> Result<Self, ImageError> {
        if width <= 0 || height <= 0 {
            return Err(ImageError::InvalidDimensions);
        }

        let color = rgba_premul(color);

        let len = (width as usize) * (height as usize);

        Ok(Self {
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
    ) -> Result<Self, ImageError> {
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

    /// Creates a `FrameImage` instance from a byte slice representing an image.
    ///
    /// This function attempts to load an image from the provided byte slice using the `image` crate,
    /// and then converts the loaded image into a `FrameImage` instance using `from_dynamic_image`.
    ///
    /// # Arguments
    ///
    /// * `bytes` - A slice of bytes containing the image data (e.g., PNG, JPEG, etc.).
    ///
    /// # Returns
    ///
    /// A `Result` containing the newly created `FrameImage` instance on success,
    /// or an `ImageError` if the image cannot be loaded or converted.
    ///
    /// # Errors
    ///
    /// - `ImageError` if the byte slice does not contain valid image data,
    ///   or if the image format is not supported, or if conversion fails.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ImageError> {
        let img = image::load_from_memory(bytes)?;
        Self::from_dynamic_image(img)
    }

    /// Creates a new `FrameImage` instance from an image file path.
    ///
    /// This function attempts to open an image file at the given path, automatically
    /// guess the image format, and decode the image into a dynamic image format.
    /// It then converts the decoded image into the type `FrameImage` using `from_dynamic_image`.
    ///
    /// # Arguments
    ///
    /// * `path` - A reference to a path (e.g., `&str`, `&Path`) that points to the image file.
    ///
    /// # Returns
    ///
    /// A `Result` containing the newly created `FrameImage` instance on success, or an `ImageError`
    /// if the file cannot be opened, the format cannot be guessed, or the image cannot be decoded.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use overlay::capture::FrameImage;
    /// let img = FrameImage::from_path("example.jpg");
    /// ```
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ImageError> {
        let img = ImageReader::open(path)?.with_guessed_format()?.decode()?;
        Self::from_dynamic_image(img)
    }

    /// Converts a `image::DynamicImage` into a `FrameImage` type representing a premultiplied RGBA image.
    ///
    /// This function takes a dynamic image from the `image` crate and converts it into an internal
    /// representation with premultiplied alpha (premul) RGBA pixel data. The resulting format is
    /// stored as a vector of 32-bit unsigned integers, where each integer encodes one RGBA pixel
    /// with alpha premultiplied.
    ///
    /// # Arguments
    ///
    /// * `img` - A reference to a `image::DynamicImage` to convert.
    ///
    /// # Returns
    ///
    /// A `Result<FrameImage, ImageError>`:
    /// - `Ok(FrameImage)` if the image is valid and non-empty.
    /// - `Err(ImageError::Empty)` if the image has zero width or height.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use overlay::capture::FrameImage;
    /// use crate::overlay::capture::ImageSource;
    /// use image::open;
    /// let img = image::open("example.png").unwrap();
    /// let result = FrameImage::from_dynamic_image(img);
    /// match result {
    ///     Ok(image) => println!("Image dimensions: {}x{}", image.width(), image.height()),
    ///     Err(e) => eprintln!("Error: {:?}", e),
    /// }
    /// ```
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
    width: i32,
    height: i32,
    stride: usize, // in pixels, stride of the underlying source buffer
    pixels: &'a [u32],
    origin: usize, // pixel offset from pixels[0]
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

    pub fn rotate_90_ccw(&self) -> FrameImage {
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
        unsafe {
            SetProcessDPIAware();
        }
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
            let dib = CreateDIBSection(screen_dc, &bmi, DIB_RGB_COLORS, &mut bits, null_mut(), 0);

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
                origin: 0,
            })
        }
    }

    /// Convenience if you need ownership.
    pub fn capture_owned(&mut self) -> Option<FrameImage> {
        self.capture().map(|f| f.to_owned())
    }
}


// ------------------------------
// Loading macros
// ------------------------------

/// Loads an image from a file path using the `overlay::FrameImage` type.
///
/// This macro takes a string literal or expression representing a file path and attempts to load
/// the image using `FrameImage::from_path`. If the image fails to load, a panic occurs with a
/// message indicating the path and the error that occurred.
///
/// # Arguments
///
/// * `path` - A string literal or expression representing the file path to the image.
///
/// # Returns
///
/// A `FrameImage` instance representing the loaded image.
///
/// # Panics
///
/// If the image fails to load, this macro will panic with a message in the format:
/// `"failed to load image <path>: <error>"`.
///
/// # Example
///
/// ```rust,ignore
/// use overlay::load_image;
/// let image = load_image!("assets/logo.png");
/// ```
///
#[macro_export]
macro_rules! load_image {
    ($path:expr) => {{
        $crate::overlay::FrameImage::from_path($path)
            .unwrap_or_else(|e| panic!("failed to load image {}: {}", $path, e))
    }};
}

/// Creates a `FrameImage` from an embedded image file using the provided path.
///
/// This macro loads a binary image file from the current crate's resources using `include_bytes!`,
/// then attempts to decode it into a `FrameImage`. If decoding fails, it panics with an error message
/// containing the path and the specific error.
///
/// # Arguments
///
/// * `$path:expr` - A string literal or expression representing the path to the embedded image file.
///   The path is relative to the crate's resources directory and must point to a valid binary image file.
///
/// # Returns
///
/// A `FrameImage` instance decoded from the embedded image data.
///
/// # Panics
///
/// If the image file cannot be loaded or decoded, this macro will panic with a message
/// indicating the path and the underlying error.
///
/// # Example
///
/// ```rust,ignore
/// use overlay::include_image;
/// include_image!("assets/example.png")
/// ```
///
/// # Note
///
/// The image must be embedded in the crate's resources and accessible via the `include_bytes!` macro.
#[macro_export]
macro_rules! include_image {
    ($path:expr) => {{
        $crate::overlay::capture::FrameImage::from_bytes(include_bytes!($path))
            .unwrap_or_else(|e| panic!("failed to decode embedded image {}: {}", $path, e))
    }};
}
