//! # Win32 Native Windowing and Hook Subsystem
//!
//! This module encapsulates the low-level Windows OS window lifecycle management.
//! It registers window classes, spawns a borderless layered window (`WS_EX_LAYERED`),
//! instantiates global low-level OS input hooks (`WH_KEYBOARD_LL`, `WH_MOUSE_LL`),
//! and orchestrates the primary real-time game/render loop using high-precision timers.

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

use crate::{
    canvas::Canvas,
    state::{OverlayState, wide_null},
};
use windows_sys::Win32::Foundation::{HINSTANCE, POINT};
use windows_sys::Win32::Graphics::Gdi::UpdateWindow;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT};

pub(crate) const SW_SHOWNOACTIVATE: i32 = 4;
pub(crate) const GWLP_USERDATA: i32 = -21;
pub(crate) const HTTRANSPARENT_VALUE: isize = -1;
pub(crate) const AC_SRC_OVER: u8 = 0x00;
pub(crate) const AC_SRC_ALPHA: u8 = 0x01;
pub(crate) const ULW_ALPHA: u32 = 0x0000_0002;
const LLKHF_INJECTED: u32 = 0x00000010;
const LLMHF_INJECTED: u32 = 0x00000001;

// ============================================================
// SAFE EVENT API
// ============================================================

/// Dictates how an input event should be processed after being intercepted by the overlay.
#[derive(Eq, PartialEq, Copy, Clone)]
pub enum EventResult {
    /// The event is consumed by the overlay application. It will **not** be passed down
    /// to the underlying windows or applications (swallowed input).
    Consumed,
    /// The event is ignored or partially reacted to, allowing it to propagate normally
    /// through the OS down to target foreground applications.
    Propagated,
}

/// Identifies standard hardware mouse button mappings.
#[derive(Clone, Copy, Debug)]
pub enum MouseButton {
    /// Left mouse button.
    Left,
    /// Right mouse button.
    Right,
    /// Middle wheel click mouse button.
    Middle,
    /// Extended side button 1.
    X1,
    /// Extended side button 2.
    X2,
}

/// A unified event container representing structural asynchronous hardware input events.
#[derive(Clone, Copy, Debug)]
pub enum OverlayEvent {
    /// A keyboard button pressed state trigger.
    KeyDown {
        /// The virtual key code identifier (e.g., `VK_ESCAPE`, `0x41` for 'A').
        vk: u32,
    },

    /// A keyboard button released state trigger.
    KeyUp {
        /// The virtual key code identifier.
        vk: u32,
    },

    /// Absolute hardware cursor position motion coordinates tracking.
    MouseMove {
        /// Global desktop x-coordinate position.
        x: i32,
        /// Global desktop y-coordinate position.
        y: i32,
    },

    /// A mouse button pressed state trigger.
    MouseDown {
        /// The specific mouse button triggered.
        button: MouseButton,
    },

    /// A mouse button released state trigger.
    MouseUp {
        /// The specific mouse button released.
        button: MouseButton,
    },

    /// Vertical mouse wheel scrolling rotation delta tracker.
    MouseWheel {
        /// Rotation wheel travel step value (multiples of standard 120 units).
        delta: i16,
    },
}

static mut STATE_PTR: *mut OverlayState = null_mut();

// ============================================================
// KEYBOARD HOOK
// ============================================================

unsafe extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let kb = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };

        if (kb.flags & LLKHF_INJECTED) != 0 {
            return unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) };
        }

        let state = unsafe { &mut *STATE_PTR };
        match wparam as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                if state.handle_event(OverlayEvent::KeyDown { vk: kb.vkCode })
                    == EventResult::Consumed
                {
                    return 1;
                }
            }

            WM_KEYUP | WM_SYSKEYUP => {
                if state.handle_event(OverlayEvent::KeyUp { vk: kb.vkCode })
                    == EventResult::Consumed
                {
                    return 1;
                }
            }

            _ => {}
        }
    }

    unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) }
}

// ============================================================
// MOUSE HOOK
// ============================================================

unsafe extern "system" fn mouse_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let mouse = unsafe { &*(lparam as *const MSLLHOOKSTRUCT) };

        if (mouse.flags & LLMHF_INJECTED) != 0 {
            return unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) };
        }

        let state = unsafe { &mut *STATE_PTR };
        match wparam as u32 {
            WM_MOUSEMOVE => {
                if state.handle_event(OverlayEvent::MouseMove {
                    x: mouse.pt.x,
                    y: mouse.pt.y,
                }) == EventResult::Consumed
                {
                    return 1;
                }
            }

            WM_LBUTTONDOWN => {
                if state.handle_event(OverlayEvent::MouseDown {
                    button: MouseButton::Left,
                }) == EventResult::Consumed
                {
                    return 1;
                }
            }

            WM_LBUTTONUP => {
                if state.handle_event(OverlayEvent::MouseUp {
                    button: MouseButton::Left,
                }) == EventResult::Consumed
                {
                    return 1;
                }
            }

            WM_RBUTTONDOWN => {
                if state.handle_event(OverlayEvent::MouseDown {
                    button: MouseButton::Right,
                }) == EventResult::Consumed
                {
                    return 1;
                }
            }

            WM_RBUTTONUP => {
                if state.handle_event(OverlayEvent::MouseUp {
                    button: MouseButton::Right,
                }) == EventResult::Consumed
                {
                    return 1;
                }
            }

            WM_MBUTTONDOWN => {
                if state.handle_event(OverlayEvent::MouseDown {
                    button: MouseButton::Middle,
                }) == EventResult::Consumed
                {
                    return 1;
                }
            }

            WM_MBUTTONUP => {
                if state.handle_event(OverlayEvent::MouseUp {
                    button: MouseButton::Middle,
                }) == EventResult::Consumed
                {
                    return 1;
                }
            }

            WM_MOUSEWHEEL => {
                let delta = ((mouse.mouseData >> 16) & 0xffff) as i16;

                if state.handle_event(OverlayEvent::MouseWheel { delta }) == EventResult::Consumed {
                    return 1;
                }
            }

            _ => {}
        }
    }

    unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) }
}

// ============================================================
// WINDOW PROC
// ============================================================

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_NCCREATE => {
            let createstruct = lparam as *const CREATESTRUCTW;

            if createstruct.is_null() {
                return 0;
            }

            let state = unsafe { *createstruct }.lpCreateParams as *mut OverlayState;

            if state.is_null() {
                return 0;
            }

            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize) };

            unsafe {
                (*state).hwnd = hwnd;
            }

            1
        }

        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }

        WM_PAINT => unsafe {
            let mut ps: PAINTSTRUCT = zeroed();

            BeginPaint(hwnd, &mut ps);
            EndPaint(hwnd, &ps);

            0
        },

        // Forces the window frame to signal complete transparent mouse hittest transparency,
        // ensuring all standard click interactions click directly through into desktop elements behind it.
        WM_NCHITTEST => HTTRANSPARENT_VALUE,

        WM_ERASEBKGND => 1,

        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// A situational controller interface allowing active overlay updates during application cycles.
pub struct OverlayContext {
    pub(super) hwnd: HWND,
    pub(super) width: i32,
    pub(super) height: i32,
}

impl OverlayContext {
    /// Unregisters and gracefully drops the running window instance.
    pub fn close(&self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }

    /// Returns the active pixel layout width tracking bounds.
    pub fn width(&self) -> i32 {
        self.width
    }

    /// Returns the active pixel layout height tracking bounds.
    pub fn height(&self) -> i32 {
        self.height
    }

    /// Queries the dynamic worldwide hardware desktop coordinate cursor tracking position.
    pub fn mouse_position(&self) -> (i32, i32) {
        unsafe {
            let mut pt = POINT { x: 0, y: 0 };

            GetCursorPos(&mut pt);

            (pt.x, pt.y)
        }
    }

    /// Enables or disables structural OS screen recording exclusion bounds.
    ///
    /// Setting this to `true` leverages `SetWindowDisplayAffinity` to turn the overlay black/invisible
    /// inside popular casting platforms, screenshots, OBS, or streaming layouts.
    pub fn hide_from_capture(&self, hide: bool) {
        unsafe {
            if hide {
                SetWindowDisplayAffinity(self.hwnd, WDA_EXCLUDEFROMCAPTURE);
            } else {
                SetWindowDisplayAffinity(self.hwnd, WDA_NONE);
            }
        }
    }

    /// Synthesizes a discrete, asynchronous hardware keyboard keystroke event.
    ///
    /// This injects a structured sequential press and release structural sequence
    /// directly into the OS input stream. The generated inputs are automatically flagged,
    /// allowing internal low-level hooks to bypass tracking loops and prevent recursive
    /// input deadlocks.
    ///
    /// # Arguments
    /// * `vk_code` - The virtual key code mapping destination target (e.g., `0x41` for 'A').
    pub fn send_keypress(&self, vk_code: u16) {
        unsafe {
            let mut input_down: INPUT = zeroed();
            input_down.r#type = INPUT_KEYBOARD;
            input_down.Anonymous.ki = KEYBDINPUT {
                wVk: vk_code,
                wScan: 0,
                dwFlags: 0,
                time: 0,
                dwExtraInfo: 0,
            };

            let mut input_up: INPUT = zeroed();
            input_up.r#type = INPUT_KEYBOARD;
            input_up.Anonymous.ki = KEYBDINPUT {
                wVk: vk_code,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };

            let mut inputs = [input_down, input_up];

            SendInput(
                inputs.len() as u32,
                inputs.as_mut_ptr(),
                size_of::<INPUT>() as i32,
            );
        }
    }


    /// Synthesizes and injects an asynchronous hardware input event into the OS stream.
    ///
    /// This translates the high-level `OverlayEvent` representation into raw, serialized
    /// structural input payloads. The resulting operations are automatically flagged as injected,
    /// instructing internal low-level event hooks to bypass tracking and prevent operational deadlocks.
    ///
    /// # Arguments
    /// * `event` - A reference to the structural `OverlayEvent` targeted for system injection.
    pub fn send_event(&self, event: &OverlayEvent) {
        unsafe {
            let mut inputs: Vec<INPUT> = Vec::new();

            match event {
                OverlayEvent::KeyDown { vk } => {
                    let mut input = zeroed::<INPUT>();
                    input.r#type = INPUT_KEYBOARD;
                    input.Anonymous.ki = KEYBDINPUT {
                        wVk: *vk as u16,
                        wScan: 0,
                        dwFlags: 0,
                        time: 0,
                        dwExtraInfo: 0,
                    };
                    inputs.push(input);
                }

                OverlayEvent::KeyUp { vk } => {
                    let mut input = zeroed::<INPUT>();
                    input.r#type = INPUT_KEYBOARD;
                    input.Anonymous.ki = KEYBDINPUT {
                        wVk: *vk as u16,
                        wScan: 0,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    };
                    inputs.push(input);
                }

                OverlayEvent::MouseMove { x, y } => {
                    let mut input = zeroed::<INPUT>();
                    input.r#type = INPUT_MOUSE;
                    input.Anonymous.mi = MOUSEINPUT {
                        dx: (*x * 65535) / self.width,
                        dy: (*y * 65535) / self.height,
                        mouseData: 0,
                        dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
                        time: 0,
                        dwExtraInfo: 0,
                    };
                    inputs.push(input);
                }

                OverlayEvent::MouseDown { button } => {
                    let mut input = zeroed::<INPUT>();
                    input.r#type = INPUT_MOUSE;
                    input.Anonymous.mi = MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouseData: match button {
                            MouseButton::X1 => 0x0001,
                            MouseButton::X2 => 0x0002,
                            _ => 0,
                        },
                        dwFlags: match button {
                            MouseButton::Left => MOUSEEVENTF_LEFTDOWN,
                            MouseButton::Right => MOUSEEVENTF_RIGHTDOWN,
                            MouseButton::Middle => MOUSEEVENTF_MIDDLEDOWN,
                            MouseButton::X1 | MouseButton::X2 => MOUSEEVENTF_XDOWN,
                        },
                        time: 0,
                        dwExtraInfo: 0,
                    };
                    inputs.push(input);
                }

                OverlayEvent::MouseUp { button } => {
                    let mut input = zeroed::<INPUT>();
                    input.r#type = INPUT_MOUSE;
                    input.Anonymous.mi = MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouseData: match button {
                            MouseButton::X1 => 0x0001,
                            MouseButton::X2 => 0x0002,
                            _ => 0,
                        },
                        dwFlags: match button {
                            MouseButton::Left => MOUSEEVENTF_LEFTUP,
                            MouseButton::Right => MOUSEEVENTF_RIGHTUP,
                            MouseButton::Middle => MOUSEEVENTF_MIDDLEUP,
                            MouseButton::X1 | MouseButton::X2 => MOUSEEVENTF_XUP,
                        },
                        time: 0,
                        dwExtraInfo: 0,
                    };
                    inputs.push(input);
                }

                OverlayEvent::MouseWheel { delta } => {
                    let mut input = zeroed::<INPUT>();
                    input.r#type = INPUT_MOUSE;
                    input.Anonymous.mi = MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouseData: (*delta as u32) << 16,
                        dwFlags: MOUSEEVENTF_WHEEL,
                        time: 0,
                        dwExtraInfo: 0,
                    };
                    inputs.push(input);
                }
            }

            if !inputs.is_empty() {
                SendInput(
                    inputs.len() as u32,
                    inputs.as_mut_ptr(),
                    size_of::<INPUT>() as i32,
                );
            }
        }
    }

    /// Mutates the global hardware cursor tracking position to the specified coordinates.
    ///
    /// This immediately moves the desktop mouse cursor to an absolute target position,
    /// defined relative to the top-left corner of the primary virtual monitor space.
    ///
    /// # Arguments
    /// * `x` - The absolute global desktop x-coordinate pixel destination.
    /// * `y` - The absolute global desktop y-coordinate pixel destination.
    /// # Example
    /// ```rust
    /// //trap - Reversing the mouse direction.
    /// use overlay::{EventResult, OverlayEvent, OverlayContext, OverlayApp};
    /// struct MyApp;
    /// impl OverlayApp for MyApp {
    ///     fn handler(&mut self, event:OverlayEvent, context:&mut OverlayContext) -> EventResult{
    ///         match event {
    ///             OverlayEvent::MouseMove { x, y } => {
    ///                 let (src_x,src_y) = context.mouse_position();
    ///                 context.set_mouse_position(src_x+(src_x-x),src_y+(src_y-y));
    ///                 EventResult::Consumed
    ///             }
    ///             _ => {EventResult::Propagated}
    ///         }
    ///
    ///     }
    /// }
    ///
    ///
    /// ```
    pub fn set_mouse_position(&self, x: i32, y: i32) {
        unsafe {
            SetCursorPos(x, y);
        }
    }


}

// ============================================================
// API
// ============================================================

/// The fundamental trait governing user overlay app runtime bindings.
/// Implemented by developers to handle events, state steps, and custom drawing loops.
pub trait OverlayApp {
    /// Fired exactly once immediately following native window handle binding instantiation.
    fn init(&mut self, _overlay_context: &mut OverlayContext) {}

    /// Main input dispatcher hook targeted at filtering globally captured mouse/keyboard operations.
    fn handler(
        &mut self,
        _event: OverlayEvent,
        _overlay_context: &mut OverlayContext,
    ) -> EventResult {
        EventResult::Propagated
    }

    /// Executed iteratively at each loop iteration step. Used for state calculations and logic increments.
    ///
    /// # Arguments
    /// * `_delta` - Floating-point duration interval measurement denoting seconds elapsed since the prior iteration frame.
    fn update(&mut self, _overlay_context: &mut OverlayContext, _delta: f32) {}

    /// Triggered following logic update completions. Used to draw visuals directly onto the frame memory canvas block.
    fn render(&mut self, _canvas: &mut Canvas) {}

    /// Fired right before the window context resources are unlinked and destroyed by the OS.
    fn shutdown(&mut self, _overlay_context: &mut OverlayContext) {}
}

/// The core bootstrapping framework block execution initialization engine.
///
/// Spawns the underlying Win32 window infrastructure, configures virtual monitor coordinates scaling layouts,
/// installs localized low-level intercept hardware hooks, and retains active main execution thread focus blocks
/// until standard shutdown sequences exit.
///
/// # Thread Safety
///
/// This call actively hijacks execution flow focus limits on the caller thread to loop structural
/// window polling hooks until structural `WM_QUIT` actions occur.
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

        let class_name = wide_null("OverlayClass");

        let window_title = wide_null("overlay");

        let mut wc: WNDCLASSW = zeroed();

        wc.style = CS_HREDRAW | CS_VREDRAW;

        wc.lpfnWndProc = Some(wndproc);

        wc.hInstance = hinstance;

        wc.lpszClassName = class_name.as_ptr();

        wc.hCursor = LoadCursorW(null_mut(), IDC_ARROW);

        wc.hbrBackground = null_mut();

        if RegisterClassW(&wc) == 0 {
            return;
        }

        // ====================================
        // CREATE STATE
        // ====================================

        let x = GetSystemMetrics(SM_XVIRTUALSCREEN);

        let y = GetSystemMetrics(SM_YVIRTUALSCREEN);

        let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);

        let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);

        let state = match OverlayState::new(0 as HWND, x, y, width, height, Box::new(app)) {
            Some(s) => s,
            None => return,
        };

        let state_ptr = Box::into_raw(state);

        STATE_PTR = state_ptr;

        // ====================================
        // CREATE WINDOW
        // ====================================

        let ex_style =
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;

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

        // ====================================
        // INSTALL HOOKS
        // ====================================

        let keyboard_hook = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(keyboard_hook_proc),
            hinstance as HINSTANCE,
            0,
        );

        let mouse_hook = SetWindowsHookExW(
            WH_MOUSE_LL,
            Some(mouse_hook_proc),
            hinstance as HINSTANCE,
            0,
        );

        // ====================================
        // INITIAL PRESENT
        // ====================================

        let state = &mut *state_ptr;

        state.hwnd = hwnd;
        state.init();
        state.update(0.0);
        state.present();

        ShowWindow(hwnd, SW_SHOWNOACTIVATE);

        UpdateWindow(hwnd);

        // ====================================
        // MAIN LOOP
        // ====================================

        let mut msg: MSG = zeroed();
        let mut last = Instant::now();

        'a: loop {
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
            UnhookWindowsHookEx(keyboard_hook);
        }

        if !mouse_hook.is_null() {
            UnhookWindowsHookEx(mouse_hook);
        }

        let _ = Box::from_raw(state_ptr);
    }
}
