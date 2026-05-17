pub mod state;
pub mod win32;
mod canvas;
mod capture;

pub use win32::run;

pub use capture::{*};
pub use crate::overlay::canvas::Canvas;

pub enum MouseButton { Left, Right, Middle, }
pub enum OverlayEvent<'a> {
    Render(&'a mut Canvas),
    MouseMoved { x: i32, y: i32 },
    MouseButtonDown { button: MouseButton },
    KeyPressed { key_code: u32 },
}


pub trait OverlayApp {
    fn handle_events(&mut self,event: OverlayEvent){}
    fn render(&mut self,canvas: &mut Canvas) {}
}