use std::cmp::PartialEq;
use std::time::Instant;
use std::{
    ffi::c_void,
    mem::zeroed,
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    Graphics::Gdi::{BeginPaint, EndPaint, PAINTSTRUCT},
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::*,
};

use crate::overlay::{
    canvas::Canvas,
    state::{wide_null, OverlayState},
};
use windows_sys::Win32::Foundation::{HINSTANCE, POINT};
use windows_sys::Win32::Graphics::Gdi::UpdateWindow;

pub(crate) const TIMER_ID: usize = 1;
pub(crate) const FRAME_MS: u32 = 16;
pub(crate) const SW_SHOWNOACTIVATE: i32 = 4;
pub(crate) const GWLP_USERDATA: i32 = -21;
pub(crate) const HTTRANSPARENT_VALUE: isize = -1;
pub(crate) const AC_SRC_OVER: u8 = 0x00;
pub(crate) const AC_SRC_ALPHA: u8 = 0x01;
pub(crate) const ULW_ALPHA: u32 = 0x0000_0002;




// ============================================================
// SAFE EVENT API
// ============================================================
#[derive(Eq, PartialEq, Copy, Clone)]
pub enum EventResult {
    Consumed,
    Propagated,
}

#[derive(Clone, Copy, Debug)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

#[derive(Clone, Copy, Debug)]
pub enum OverlayEvent {
    KeyDown {
        vk: u32,
    },

    KeyUp {
        vk: u32,
    },

    MouseMove {
        x: i32,
        y: i32,
    },

    MouseDown {
        button: MouseButton,
    },

    MouseUp {
        button: MouseButton,
    },

    MouseWheel {
        delta: i16,
    },
}



static mut STATE_PTR: *mut OverlayState = null_mut();



// ============================================================
// KEYBOARD HOOK
// ============================================================

unsafe extern "system" fn keyboard_hook_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {

    if code >= 0 {
        let kb = &*(lparam as *const KBDLLHOOKSTRUCT);
        let state = &mut *STATE_PTR;
        match wparam as u32 {

            WM_KEYDOWN | WM_SYSKEYDOWN => {
                if state.handle_event(OverlayEvent::KeyDown {
                    vk: kb.vkCode,
                }) == EventResult::Consumed {return 1}

            }

            WM_KEYUP | WM_SYSKEYUP => {

                if state.handle_event(OverlayEvent::KeyUp {
                    vk: kb.vkCode,
                }) == EventResult::Consumed {return 1}
            }

            _ => {}
        }
    }

    CallNextHookEx(null_mut(), code, wparam, lparam)
}



// ============================================================
// MOUSE HOOK
// ============================================================

unsafe extern "system" fn mouse_hook_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {

    if code >= 0 {

        let mouse = &*(lparam as *const MSLLHOOKSTRUCT);
        let state = &mut *STATE_PTR;
        match wparam as u32 {

            WM_MOUSEMOVE => {
                if state.handle_event(OverlayEvent::MouseMove {
                    x: mouse.pt.x,
                    y: mouse.pt.y,
                }) == EventResult::Consumed {return 1}
            }

            WM_LBUTTONDOWN => {
                if state.handle_event(OverlayEvent::MouseDown {
                    button: MouseButton::Left,
                }) == EventResult::Consumed {return 1}
            }

            WM_LBUTTONUP => {
                if state.handle_event(OverlayEvent::MouseUp {
                    button: MouseButton::Left,
                }) == EventResult::Consumed {return 1}
            }

            WM_RBUTTONDOWN => {
                if state.handle_event(OverlayEvent::MouseDown {
                    button: MouseButton::Right,
                }) == EventResult::Consumed {return 1}
            }

            WM_RBUTTONUP => {
                if state.handle_event(OverlayEvent::MouseUp {
                    button: MouseButton::Right,
                }) == EventResult::Consumed {return 1}
            }

            WM_MBUTTONDOWN => {
                if state.handle_event(OverlayEvent::MouseDown {
                    button: MouseButton::Middle,
                }) == EventResult::Consumed {return 1}
            }

            WM_MBUTTONUP => {
                if state.handle_event(OverlayEvent::MouseUp {
                    button: MouseButton::Middle,
                }) == EventResult::Consumed {return 1}
            }

            WM_MOUSEWHEEL => {

                let delta =
                    ((mouse.mouseData >> 16) & 0xffff) as i16;

                if state.handle_event(OverlayEvent::MouseWheel {
                    delta,
                }) == EventResult::Consumed {return 1;}
            }

            _ => {}
        }
    }

    CallNextHookEx(null_mut(), code, wparam, lparam)
}





// ============================================================
// WINDOW PROC
// ============================================================

unsafe extern "system" fn wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT  {
    match msg {

        WM_NCCREATE => {

            let createstruct =
                lparam as *const CREATESTRUCTW;

            if createstruct.is_null() {
                return 0;
            }

            let state =
                (*createstruct).lpCreateParams
                    as *mut OverlayState;

            if state.is_null() {
                return 0;
            }

            SetWindowLongPtrW(
                hwnd,
                GWLP_USERDATA,
                state as isize,
            );

            (*state).hwnd = hwnd;

            1
        }



        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }

        WM_PAINT => {

            let mut ps: PAINTSTRUCT = zeroed();

            BeginPaint(hwnd, &mut ps);
            EndPaint(hwnd, &ps);

            0
        }

        WM_NCHITTEST => HTTRANSPARENT_VALUE,

        WM_ERASEBKGND => 1,

        _ => DefWindowProcW(
            hwnd,
            msg,
            wparam,
            lparam,
        ),
    }
}




pub struct OverlayContext {
    pub(super) hwnd: HWND,
    pub(super) width: i32,
    pub(super) height: i32,
}

impl OverlayContext {
    pub fn close(&self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
    pub fn width(&self) -> i32 {
        self.width
    }
    pub fn height(&self) -> i32 {
        self.height
    }
    pub fn set_render_fps(&self, fps: f32) {todo!()}
    fn mouse_position(&self) -> (i32, i32) {
        unsafe {

            let mut pt = POINT {
                x: 0,
                y: 0,
            };

            GetCursorPos(&mut pt);

            (pt.x, pt.y)
        }
    }
}



// ============================================================
// API
// ============================================================

pub trait OverlayApp {

    fn init(&mut self,overlay_context: &mut OverlayContext){}
    fn handler(
        &mut self,
        _event: OverlayEvent,
        _overlay_context: &mut OverlayContext
    ) -> EventResult {EventResult::Propagated}


    fn update(&mut self,_overlay_context: &mut OverlayContext,_delta: f32) {}

    fn render(
        &mut self,
        _canvas: &mut Canvas,
    ) {}

    fn shutdown(&mut self, _overlay_context: &mut OverlayContext) {}


}


pub fn run(app: impl OverlayApp + 'static) {

    unsafe {

        SetProcessDPIAware();

        let hinstance = GetModuleHandleW(null());

        if hinstance.is_null() {
            return;
        }



        // ====================================
        // REGISTER WINDOW CLASS
        // ====================================

        let class_name =
            wide_null("OverlayClass");

        let window_title =
            wide_null("overlay");

        let mut wc: WNDCLASSW = zeroed();

        wc.style =
            CS_HREDRAW
                | CS_VREDRAW;

        wc.lpfnWndProc =
            Some(wndproc);

        wc.hInstance = hinstance;

        wc.lpszClassName =
            class_name.as_ptr();

        wc.hCursor =
            LoadCursorW(
                null_mut(),
                IDC_ARROW,
            );

        wc.hbrBackground =
            null_mut();

        if RegisterClassW(&wc) == 0 {
            return;
        }





        // ====================================
        // CREATE STATE
        // ====================================

        let x =
            GetSystemMetrics(
                SM_XVIRTUALSCREEN,
            );

        let y =
            GetSystemMetrics(
                SM_YVIRTUALSCREEN,
            );

        let width =
            GetSystemMetrics(
                SM_CXVIRTUALSCREEN,
            );

        let height =
            GetSystemMetrics(
                SM_CYVIRTUALSCREEN,
            );

        let mut state =
            match OverlayState::new(
                0 as HWND,
                x,
                y,
                width,
                height,
                Box::new(app),
            ) {
                Some(s) => s,
                None => return,
            };

        let state_ptr =
            Box::into_raw(state);

        STATE_PTR = state_ptr;







        // ====================================
        // CREATE WINDOW
        // ====================================

        let ex_style =
            WS_EX_LAYERED
                | WS_EX_TRANSPARENT
                | WS_EX_TOPMOST
                | WS_EX_TOOLWINDOW
                | WS_EX_NOACTIVATE;

        let hwnd =
            CreateWindowExW(
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

            let _ =
                Box::from_raw(state_ptr);

            return;
        }









        // ====================================
        // INSTALL HOOKS
        // ====================================

        let keyboard_hook =
            SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_hook_proc),
                hinstance as HINSTANCE,
                0,
            );

        let mouse_hook =
            SetWindowsHookExW(
                WH_MOUSE_LL,
                Some(mouse_hook_proc),
                hinstance as HINSTANCE,
                0,
            );









        // ====================================
        // INITIAL PRESENT
        // ====================================

        let state =
            &mut *state_ptr;

        state.hwnd = hwnd;
        state.init();
        state.update(0.0);
        state.present();

        ShowWindow(
            hwnd,
            SW_SHOWNOACTIVATE,
        );

        UpdateWindow(hwnd);









        // ====================================
        // MAIN LOOP
        // ====================================

        let mut msg: MSG = zeroed();
        let mut last = Instant::now();

        'a :loop {
            // =========================
            // 1. HANDLE ALL MESSAGES
            // =========================
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT {
                    break 'a;
                }

                TranslateMessage(&msg);
                DispatchMessageW(&msg);


            }

            // =========================
            // 2. TIME STEP
            // =========================
            let now = Instant::now();
            let delta = now.duration_since(last).as_secs_f32();
            last = now;


            // =========================
            // 4. UPDATE
            // =========================
            state.update(delta);


            // =========================
            // 5. RENDER
            // =========================
            state.render();
            state.present();
        }









        // ====================================
        // CLEANUP
        // ====================================

        if !keyboard_hook.is_null() {
            UnhookWindowsHookEx(
                keyboard_hook,
            );
        }

        if !mouse_hook.is_null() {
            UnhookWindowsHookEx(
                mouse_hook,
            );
        }

        let _ =
            Box::from_raw(state_ptr);
    }
}