pub type Color = (u8, u8, u8, u8); // (r,g,b,a)
#[inline]
pub const fn rgba_premul(color: Color) -> u32 {
    let (r, g, b, a) = color;
    let a32 = a as u32;
    let r32 = ((r as u32) * a32 + 127) / 255;
    let g32 = ((g as u32) * a32 + 127) / 255;
    let b32 = ((b as u32) * a32 + 127) / 255;
    (a32 << 24) | (r32 << 16) | (g32 << 8) | b32
}

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
/// use overlay::image::common::Rect;
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
    /// Creates a new rectangle with the specified position and dimensions.
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self { x, y, width, height }
    }
}