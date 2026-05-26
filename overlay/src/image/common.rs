/// Represents a color as a tuple of 8-bit channels: `(Red, Green, Blue, Alpha)`.
pub type Color = (u8, u8, u8, u8);

/// Converts a straight [`Color`] tuple into a single packed `u32` integer
/// using **premultiplied alpha** layout (`0xAARRGGBB`).
///
/// This function multiplies each color channel (RGB) by the alpha channel,
/// which optimizes blending performance for 2D rendering engines.
/// It uses integer rounding `(value + 127) / 255` for high-accuracy conversion.
#[inline]
pub const fn rgba_premul(color: Color) -> u32 {
    let (r, g, b, a) = color;
    let a32 = a as u32;
    let r32 = ((r as u32) * a32 + 127) / 255;
    let g32 = ((g as u32) * a32 + 127) / 255;
    let b32 = ((b as u32) * a32 + 127) / 255;
    (a32 << 24) | (r32 << 16) | (g32 << 8) | b32
}

/// Converts a raw slice of straight RGBA bytes into a packed `u32` integer
/// using **premultiplied alpha** layout (`0xAARRGGBB`).
///
/// # Panics
///
/// Panics if the input slice `px` has fewer than 4 elements.
#[inline]
pub fn premul_rgba_bytes_to_u32(px: &[u8]) -> u32 {
    let r = px[0] as u32;
    let g = px[1] as u32;
    let b = px[2] as u32;
    let a = px[3] as u32;
    let r = (r * a + 127) / 255;
    let g = (g * a + 127) / 255;
    let b = (b * a + 127) / 255;
    (a << 24) | (r << 16) | (g << 8) | b
}

/// A 2D rectangle defined by its top-left corner `(x, y)` and its dimensions `(width, height)`.
///
/// This struct represents a bounding box in a 2D coordinate system where the origin `(0, 0)` is
/// located in the top-left corner of the screen, extending rightwards and downwards.
///
/// # Fields
///
/// * `x` - The x-coordinate of the top-left corner.
/// * `y` - The y-coordinate of the top-left corner.
/// * `width` - The horizontal size of the rectangle in pixels.
/// * `height` - The vertical size of the rectangle in pixels.
///
/// # Examples
///
/// ```rust
/// use overlay::image::common::Rect;
/// let rect = Rect::new(10, 20, 100, 50);
/// assert_eq!(rect.width, 100);
/// assert_eq!(rect.height, 50);
/// ```
///
/// # Notes
///
/// All fields are stored as signed 32-bit integers (`i32`). This struct does not perform
/// bounds validation upon construction. It is the caller's responsibility to ensure that
/// `width` and `height` are non-negative where operations require valid geometric bounds.
#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    /// Creates a new rectangle with the specified position and dimensions.
    #[inline]
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}
