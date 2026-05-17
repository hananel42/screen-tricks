use std::{
    ffi::c_void,
    mem::{size_of, zeroed},
    ptr::null_mut,
    slice,
};

use crate::overlay::canvas::Canvas;
use crate::overlay::win32::{
    AC_SRC_ALPHA, AC_SRC_OVER, ULW_ALPHA,
};
use windows_sys::Win32::Graphics::Gdi::SRCCOPY;
use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SetProcessDPIAware, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN};
use windows_sys::Win32::{
    Foundation::{HWND, POINT, SIZE},
    Graphics::Gdi::{
        BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject,
        GetDC, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
        BLENDFUNCTION, CAPTUREBLT, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ,
    },
    UI::WindowsAndMessaging::UpdateLayeredWindow,
};


pub(super) struct OverlayState<F> where F:FnMut(&mut Canvas){
    pub(super) hwnd: HWND,
    mem_dc: HDC,
    dib: HBITMAP,
    old_obj: HGDIOBJ,
    canvas: Canvas,
    x: i32,
    y: i32,
    render_fn: F
}

impl<F> Drop for OverlayState<F> where F:FnMut(&mut Canvas) {
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

pub(crate) fn wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}





impl<F> OverlayState<F> where F:FnMut(&mut Canvas) {
    pub(crate) unsafe fn new(
        hwnd: HWND,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        render_fn:F
    ) -> Option<Box<Self>>{
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
        bmi.bmiHeader.biWidth = width;
        bmi.bmiHeader.biHeight = -height;
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

        let _ = ReleaseDC(null_mut(), screen_dc);

        if dib.is_null() || bits.is_null() {
            let _ = DeleteDC(mem_dc);
            return None;
        }

        let old_obj = SelectObject(mem_dc, dib as HGDIOBJ);
        if old_obj.is_null() {
            let _ = DeleteObject(dib as HGDIOBJ);
            let _ = DeleteDC(mem_dc);
            return None;
        }
        let canvas = Canvas{
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
            x,
            y,
            render_fn,
        }))
    }


    pub(crate) fn render(&mut self){
        (self.render_fn)(&mut self.canvas);

    }

    pub(crate) unsafe fn present(&self) {
        let screen_dc = GetDC(null_mut());
        if screen_dc.is_null() {
            return;
        }

        let dst = POINT { x: self.x, y: self.y };
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

        let _ = UpdateLayeredWindow(
            self.hwnd,
            screen_dc,
            &dst,
            &size,
            self.mem_dc,
            &src,
            0,
            &blend,
            ULW_ALPHA,
        );

        let _ = ReleaseDC(null_mut(), screen_dc);
    }
}
