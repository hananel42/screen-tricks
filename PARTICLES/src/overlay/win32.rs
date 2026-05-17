use std::{ffi::c_void, mem::zeroed, ptr::{null, null_mut}};

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    Graphics::Gdi::{BeginPaint, EndPaint, PAINTSTRUCT},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Input::KeyboardAndMouse::GetAsyncKeyState,
        WindowsAndMessaging::*,
    },
};
use windows_sys::Win32::Graphics::Gdi::UpdateWindow;
use crate::overlay::canvas::Canvas;
use crate::overlay::state::{
     wide_null, OverlayState,
};

pub(crate) const TIMER_ID: usize = 1;
pub(crate) const FRAME_MS: u32 = 16;
pub(crate) const VK_ESCAPE_KEY: i32 = 0x1B;
pub(crate) const SW_SHOWNOACTIVATE: i32 = 4;
pub(crate) const GWLP_USERDATA: i32 = -21;
pub(crate) const HTTRANSPARENT_VALUE: isize = -1;

pub(crate) const AC_SRC_OVER: u8 = 0x00;
pub(crate) const AC_SRC_ALPHA: u8 = 0x01;
pub(crate) const ULW_ALPHA: u32 = 0x0000_0002;

unsafe extern "system" fn wndproc<F>(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT where F:FnMut(&mut Canvas) {
    match msg {
        WM_NCCREATE => {
            let createstruct = lparam as *const CREATESTRUCTW;
            if createstruct.is_null() {
                return 0;
            }

            let state = (*createstruct).lpCreateParams as *mut OverlayState<F>;
            if state.is_null() {
                return 0;
            }

            let _ = SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize);
            (*state).hwnd = hwnd;
            1
        }

        WM_CREATE => {
            let _ = SetTimer(hwnd, TIMER_ID, FRAME_MS, None);
            0
        }

        WM_TIMER => {
            if (GetAsyncKeyState(VK_ESCAPE_KEY) as u16 & 0x8000) != 0 {
                DestroyWindow(hwnd);
                return 0;
            }

            let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut OverlayState<F>;
            if !state_ptr.is_null() {
                (*state_ptr).render();
                (*state_ptr).present();
            }

            0
        }

        WM_NCHITTEST => HTTRANSPARENT_VALUE,

        WM_ERASEBKGND => 1,

        WM_PAINT => {
            let mut ps: PAINTSTRUCT = zeroed();
            let _ = BeginPaint(hwnd, &mut ps);
            EndPaint(hwnd, &ps);
            0
        }

        WM_DESTROY => {
            let _ = KillTimer(hwnd, TIMER_ID);
            PostQuitMessage(0);
            0
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub fn run<F>(render:F) where F:FnMut(&mut Canvas){
    unsafe {
        SetProcessDPIAware();
        let hinstance = GetModuleHandleW(null());
        if hinstance.is_null() {
            return;
        }

        let class_name = wide_null("OverlayClass");
        let window_title = wide_null("overlay");

        let mut wc: WNDCLASSW = zeroed();
        wc.style = CS_HREDRAW | CS_VREDRAW;
        wc.lpfnWndProc = Some(wndproc::<F>);
        wc.hInstance = hinstance;
        wc.lpszClassName = class_name.as_ptr();
        wc.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
        wc.hbrBackground = null_mut();

        let atom = RegisterClassW(&wc);
        if atom == 0 {
            return;
        }

        let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);

        let state = match OverlayState::new(
            0 as HWND,
            x,
            y,
            width,
            height,
            render
        ) {
            Some(s) => s,
            None => return,
        };

        let state_ptr = Box::into_raw(state);

        let ex_style = WS_EX_LAYERED
            | WS_EX_TRANSPARENT
            | WS_EX_TOPMOST
            | WS_EX_TOOLWINDOW
            | WS_EX_NOACTIVATE;

        let hwnd = CreateWindowExW(
            ex_style,
            class_name.as_ptr(),
            window_title.as_ptr(),
            WS_POPUP,
            x,
            y,
            width,
            height,
            null_mut(),
            null_mut(),
            hinstance,
            state_ptr as *const c_void,
        );

        if hwnd.is_null() {
            let _ = Box::from_raw(state_ptr);
            return;
        }

        let state = &mut *state_ptr;
        state.hwnd = hwnd;
        state.render();
        state.present();

        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        let _ = UpdateWindow(hwnd);

        let mut msg: MSG = zeroed();
        loop {
            let r = GetMessageW(&mut msg, null_mut(), 0, 0);
            if r == -1 || r == 0 {
                break;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let _ = Box::from_raw(state_ptr);
    }
}
