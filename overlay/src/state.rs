//! # Internal Overlay Window State Manager
//!
//! This module handles the lifetime, event routing, and software-to-OS pixel
//! presentation (`UpdateLayeredWindow`) for a Win32 layered overlay window.
//!
//! It encapsulates the core state mechanics safely away from the public API.
use std::{
    ffi::c_void,
    mem::{size_of, zeroed},
    ptr::null_mut,
};

use crate::OverlayApp;
use crate::canvas::Canvas;
use crate::win32::{
    AC_SRC_ALPHA, AC_SRC_OVER, EventResult, OverlayContext, OverlayEvent, ULW_ALPHA,
};
use windows_sys::Win32::{
    Foundation::{HWND, POINT, SIZE},
    Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateCompatibleDC, CreateDIBSection,
        DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, HBITMAP, HDC, HGDIOBJ, ReleaseDC,
        SelectObject,
    },
    UI::WindowsAndMessaging::UpdateLayeredWindow,
};

/// Internal state manager for a single active overlay window context.
///
/// This struct acts as the core bridge between the OS-level window handle (`HWND`),
/// the memory-mapped graphics buffer ([`Canvas`]), and the user-defined application logic ([`OverlayApp`]).
/// It orchestrates event dispatching, frame updates, and the final presentation to the screen.
pub(super) struct OverlayState {
    pub(super) hwnd: HWND,
    mem_dc: HDC,
    dib: HBITMAP,
    old_obj: HGDIOBJ,
    canvas: Canvas,
    overlay_context: OverlayContext,
    x: i32,
    y: i32,
    app: Box<dyn OverlayApp>,
}

impl Drop for OverlayState {
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
        }
    }
}

/// Helper utility that converts a standard Rust UTF-8 string slice into a
/// null-terminated `Vec<u16>` wide string, which is required by Win32 API endpoints.
pub(crate) fn wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

impl OverlayState {
    /// Allocates and initializes a new `OverlayState` context packaged inside a `Box`.
    ///
    /// This sets up an independent GDI Device Context (DC) and maps a 32-bit Device-Independent
    /// Bitmap (DIB) backing memory block directly to the inner [`Canvas`]. This allows the framework
    /// to support true per-pixel alpha channels needed for seamless transparent overlays.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it accepts a raw Win32 `HWND` window handle. The caller
    /// must guarantee that the provided handle points to a valid, un-dropped layered window
    /// instance on the current thread.
    pub(crate) unsafe fn new(
        hwnd: HWND,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        app: Box<dyn OverlayApp>,
    ) -> Option<Box<Self>> {
        let screen_dc = unsafe {GetDC(null_mut())};
        if screen_dc.is_null() {
            return None;
        }

        let mem_dc = unsafe {CreateCompatibleDC(screen_dc)};
        if mem_dc.is_null() {
            let _ = unsafe {ReleaseDC(null_mut(), screen_dc)};
            return None;
        }

        let mut bmi: BITMAPINFO = unsafe {zeroed()};
        bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = width;
        bmi.bmiHeader.biHeight = -height;
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        let mut bits: *mut c_void = null_mut();
        let dib = unsafe {CreateDIBSection(screen_dc, &bmi, DIB_RGB_COLORS, &mut bits, null_mut(), 0)};

        let _ = unsafe {ReleaseDC(null_mut(), screen_dc)};

        if dib.is_null() || bits.is_null() {
            let _ = unsafe {DeleteDC(mem_dc)};
            return None;
        }

        let old_obj = unsafe {SelectObject(mem_dc, dib as HGDIOBJ)};
        if old_obj.is_null() {
            unsafe {
                let _ = DeleteObject(dib as HGDIOBJ);
                let _ = DeleteDC(mem_dc);
            }
            return None;
        }
        let canvas = Canvas {
            bits: bits as *mut u32,
            len: (width as usize) * (height as usize),
            width,
            height,
        };

        Some(Box::new(Self {
            hwnd,
            mem_dc,
            dib,
            old_obj,
            canvas,
            overlay_context: OverlayContext {
                hwnd,
                width,
                height,
            },
            x,
            y,
            app,
        }))
    }

    pub(super) fn handle_event(&mut self, overlay_event: OverlayEvent) -> EventResult {
        self.app.handler(overlay_event, &mut self.overlay_context)
    }
    pub(super) fn render(&mut self) {
        self.app.render(&mut self.canvas);
    }
    pub(super) fn init(&mut self) {
        self.overlay_context.hwnd = self.hwnd;
        self.app.init(&mut self.overlay_context)
    }

    pub(super) fn update(&mut self, delta: f32) {
        self.app.update(&mut self.overlay_context, delta);
    }

    /// Flushes the active in-memory bitmap structure into the Windows OS window compositing manager.
    ///
    /// It invokes `UpdateLayeredWindow` using standard alpha channels (`AC_SRC_ALPHA`), rendering
    /// any graphics calculated inside the canvas onto the desktop screen with correct transparency rates.
    ///
    /// # Safety
    ///
    /// Interacts directly with OS internal graphic handles (`HDC`). Assumes that the underlying
    /// hardware contexts allocated upon creation are still fully active and valid.
    pub(super) unsafe fn present(&self) {
        let screen_dc = unsafe { GetDC(null_mut()) };
        if screen_dc.is_null() {
            return;
        }

        let dst = POINT {
            x: self.x,
            y: self.y,
        };
        let src = POINT { x: 0, y: 0 };
        let size = SIZE {
            cx: self.canvas.width,
            cy: self.canvas.height,
        };

        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA,
        };

        let _ = unsafe {
            UpdateLayeredWindow(
                self.hwnd,
                screen_dc,
                &dst,
                &size,
                self.mem_dc,
                &src,
                0,
                &blend,
                ULW_ALPHA,
            )
        };

        let _ = unsafe {ReleaseDC(null_mut(), screen_dc)};
    }
}