//! A rust module for creating and managing images and screenshots.
//!
//! ## Public Types and Functions
//!
//! - `CaptureSession`: A session for capturing screen frames, enabling real-time video or image streaming.
//! - `FrameImage`: A captured frame of screen content, typically represented as a pixel buffer.
//! - `ImageSource`: Defines where screen content is sourced from (e.g., full screen, specific window).
//! - `ImageView`: A view into a captured image, allowing for manipulation or display.
use std::{
    ffi::c_void,
    mem::{size_of, zeroed},
    ptr::null_mut,
    slice,
};

use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, CreateCompatibleDC, CreateDIBSection,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, HBITMAP, HDC, HGDIOBJ, ReleaseDC, SRCCOPY,
    SelectObject,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
    SetProcessDPIAware,
};
use crate::image::common::*;
use crate::image::frames::*;
// ------------------------------
// Image model
// ------------------------------


// ------------------------------
// Capture backend
// ------------------------------

/// A capture session that captures a region of the screen, including optional layered windows.
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

    fn virtual_screen() -> Option<Rect> {
        unsafe {
            let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);

            if width <= 0 || height <= 0 {
                None
            } else {
                Some(Rect {
                    x,
                    y,
                    width,
                    height,
                })
            }
        }
    }

    /// Creates a new instance of the type with DPI awareness enabled for the current process.
    /// Returns `Some(CaptureSession)` if successful, otherwise `None` if the virtual screen bounds
    /// could not be retrieved or another error occurred.
    pub fn new() -> Option<Self> {
        unsafe {
            SetProcessDPIAware();
        }
        Self::with_rect(CaptureSession::virtual_screen()?, true)
    }

    /// Creates a new screen capture context for capturing a rectangular region of the screen.
    ///
    /// # Arguments
    ///
    /// * `rect` - A `Rect` struct specifying the region of the screen to capture (in screen coordinates).
    /// * `include_layered_windows` - A boolean flag indicating whether to include layered windows
    ///   in the capture (e.g., transparent or overlay windows). If `true`, layered windows are rendered
    ///   as part of the capture; if `false`, they are skipped.
    ///
    /// # Returns
    ///
    /// Returns `Some(CaptureSession)` if the context is successfully created and initialized. Returns `None`
    /// if any of the Windows API calls fail.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use overlay::capture::{CaptureSession,Rect,ImageView};
    /// let mut cap = CaptureSession::with_rect(Rect::new(0, 0, 800, 600), true).unwrap();
    /// if let Some(frame) = cap.capture() {
    ///     //do stuff with your image :)
    /// }
    /// ```
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

    /// Returns a `Rect` representing the bounding rectangle of the object.
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



