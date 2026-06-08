// ============================================================
// KEYBOARD HOOK
// ============================================================

use std::ffi::c_void;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP};
use crate::state::{OverlayAppWithRender, OverlayState};

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

pub(crate) static mut HANDLER_PTR: Handler = Handler {
    self_pointer: None,
    handler_pointer: None,
    mouse_hook: None,
    keyboard_hook: None,
};

unsafe extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let kb = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };

        if (kb.flags & crate::win32::LLKHF_INJECTED) != 0 {
            return unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) };
        }

        let handler = std::ptr::addr_of_mut!(HANDLER_PTR);
        match wparam as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                if (*handler).handle_event(OverlayEvent::KeyDown { vk: kb.vkCode })
                    == EventResult::Consumed
                {
                    return 1;
                }
            }

            WM_KEYUP | WM_SYSKEYUP => {
                if (*handler).handle_event(OverlayEvent::KeyUp { vk: kb.vkCode })
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

        if (mouse.flags & crate::win32::LLMHF_INJECTED) != 0 {
            return unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) };
        }

        let handler = std::ptr::addr_of_mut!(HANDLER_PTR);
        match wparam as u32 {
            WM_MOUSEMOVE => {
                if (*handler).handle_event(OverlayEvent::MouseMove {
                    x: mouse.pt.x,
                    y: mouse.pt.y,
                }) == EventResult::Consumed
                {
                    return 1;
                }
            }

            WM_LBUTTONDOWN => {
                if (*handler).handle_event(OverlayEvent::MouseDown {
                    button: MouseButton::Left,
                }) == EventResult::Consumed
                {
                    return 1;
                }
            }

            WM_LBUTTONUP => {
                if (*handler).handle_event(OverlayEvent::MouseUp {
                    button: MouseButton::Left,
                }) == EventResult::Consumed
                {
                    return 1;
                }
            }

            WM_RBUTTONDOWN => {
                if (*handler).handle_event(OverlayEvent::MouseDown {
                    button: MouseButton::Right,
                }) == EventResult::Consumed
                {
                    return 1;
                }
            }

            WM_RBUTTONUP => {
                if (*handler).handle_event(OverlayEvent::MouseUp {
                    button: MouseButton::Right,
                }) == EventResult::Consumed
                {
                    return 1;
                }
            }

            WM_MBUTTONDOWN => {
                if (*handler).handle_event(OverlayEvent::MouseDown {
                    button: MouseButton::Middle,
                }) == EventResult::Consumed
                {
                    return 1;
                }
            }

            WM_MBUTTONUP => {
                if (*handler).handle_event(OverlayEvent::MouseUp {
                    button: MouseButton::Middle,
                }) == EventResult::Consumed
                {
                    return 1;
                }
            }

            WM_MOUSEWHEEL => {
                let delta = ((mouse.mouseData >> 16) & 0xffff) as i16;

                if (*handler).handle_event(OverlayEvent::MouseWheel { delta }) == EventResult::Consumed {
                    return 1;
                }
            }

            _ => {}
        }
    }

    unsafe { CallNextHookEx(null_mut(), code, wparam, lparam) }
}

pub(crate) struct Handler {
    self_pointer: Option<*mut c_void>,
    handler_pointer: Option<fn(*mut c_void, OverlayEvent) -> EventResult>,
    mouse_hook: Option<HHOOK>,
    keyboard_hook: Option<HHOOK>,
}

impl Handler {
    // גורמים לפונקציה לקבל הפניה (borrow) כדי שה-state לא יימחק בסוף הריצה
    pub(crate) fn register<A: OverlayAppWithRender>(&mut self, state: &mut OverlayState<A>) {
        // שמירת המצביע הגולמי בבטחה
        self.self_pointer = Some(state as *mut OverlayState<A> as *mut c_void);

        // פונקציית הגישור שמקבלת את הטיפוס הגנרי הנכון ומפעילה אותו במהירות סטטית (Inlined)
        fn trampoline_event<A: OverlayAppWithRender>(
            app_ptr: *mut c_void,
            event: OverlayEvent
        ) -> EventResult {
            unsafe {
                let state = &mut *(app_ptr as *mut OverlayState<A>);
                state.handle_event(event)
            }
        }

        self.handler_pointer = Some(trampoline_event::<A>);
    }

    pub(crate) fn handle_event(&mut self, event: OverlayEvent) -> EventResult {
        // קריאה ישירה ומהירה למצביע הפונקציה השמור
        if let (Some(handler), Some(state)) = (self.handler_pointer, self.self_pointer) {
            handler(state, event)
        } else {
            EventResult::Propagated
        }
    }

    pub(crate) fn start(&mut self) {
        let hinstance = unsafe { GetModuleHandleW(null()) };

        if hinstance.is_null() {
            return;
        }
        unsafe {
            self.keyboard_hook = Some(SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_hook_proc),
                hinstance as HINSTANCE,
                0,
            ));

            self.mouse_hook = Some(SetWindowsHookExW(
                WH_MOUSE_LL,
                Some(mouse_hook_proc),
                hinstance as HINSTANCE,
                0,
            ));
        }
    }

    pub(crate) fn stop(&mut self) {
        if let Some(h) = self.mouse_hook {
            unsafe { UnhookWindowsHookEx(h); }
        }

        if let Some(h) = self.keyboard_hook {
            unsafe { UnhookWindowsHookEx(h); }
        }
    }
}
