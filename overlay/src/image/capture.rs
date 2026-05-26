//! # Windows GDI Screen Capture Module
//!
//! This module provides synchronous, high-performance screen capture capabilities for Windows
//! environments using the Win32 GDI (Graphics Device Interface) API.
//!
//! It utilizes an in-memory Device Context (DC) paired with a Device-Independent Bitmap (DIB) Section
//! to pull raw desktop frame bytes into user-space without triggering persistent heap allocations during capture.

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

/// A managed Windows GDI screen capture session targeting a specific rectangular desktop region.
///
/// This session allocates persistent OS-native graphics handles (`HDC`, `HBITMAP`) upon creation.
/// Frame captures are written directly into a pre-allocated memory buffer mapped to a DIB section,
/// making the operation extremely efficient for game loops or real-time overlays.
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
    /// Queries the OS for the aggregate dimensions of the entire virtual screen layout
    /// (supporting multi-monitor environments).
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

    /// Creates a new capture session spanning the entire virtual screen area.
    ///
    /// Automatically forces the calling process to become DPI-Aware to ensure OS coordinate scaling
    /// matches exact desktop pixel buffers.
    ///
    /// # Returns
    /// * `Some(CaptureSession)` - If graphics contexts initialize successfully.
    /// * `None` - If virtual desktop metrics query returns empty bounds.
    pub fn new() -> Option<Self> {
        unsafe {
            SetProcessDPIAware();
        }
        Self::with_rect(CaptureSession::virtual_screen()?, true)
    }

    /// Creates a localized screen capture context mapping a customized rectangular region.
    ///
    /// # Arguments
    ///
    /// * `rect` - Bounding constraints defining the target recording window in global desktop space.
    /// * `include_layered_windows` - Set to `true` to force transparent/alpha-blended overlays
    ///   (such as tooltips or alternative overlay widgets) to render into the captured buffer.
    ///
    /// # Returns
    /// * `Some(CaptureSession)` - Fully configured and bound session ready to poll frames.
    /// * `None` - If Win32 handle allocations (`GetDC`, `CreateCompatibleDC` or `CreateDIBSection`) fail.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use overlay::image::{CaptureSession, Rect};
    ///
    /// let mut cap = CaptureSession::with_rect(Rect::new(0, 0, 800, 600), true).unwrap();
    /// if let Some(frame) = cap.capture() {
    ///     assert_eq!(frame.width(), 800);
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
            // Negative height forces a top-down DIB layout matching standard array traversal constraints.
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

    /// Returns the target coordinate bounds of this session.
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Polls the screen layout, filling the interior memory slice buffer with a fresh screen snapshot.
    ///
    /// This method performs **zero runtime allocations**. The memory block backing the returned
    /// [`ImageView`] is managed entirely by the inner OS DIB structure and is overwritten on consecutive calls.
    ///
    /// # Safety and Lifetimes
    ///
    /// The returned `ImageView` holds an immutable reference bounded to the lifetime of this `CaptureSession`.
    ///
    /// # Returns
    /// * `Some(ImageView)` - Containing volatile raw `u32` layout pixels if `BitBlt` transfers execute successfully.
    /// * `None` - If the underlying Windows hardware block-transfer signals a failure state.
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

    /// Captures the current frame and deep-copies the pixel buffer into an independent, heap-allocated [`FrameImage`].
    ///
    /// Use this if you need to pass the captured frame across threads, cache it, or prevent subsequent calls to
    /// `capture` from mutating its data content.
    pub fn capture_owned(&mut self) -> Option<FrameImage> {
        self.capture().map(|f| f.to_owned())
    }
}