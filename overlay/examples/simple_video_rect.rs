//! A simple overlay application that captures and displays a video frame with smooth movement and collision detection.
//!
//! This application uses the `overlay` crate to create a background overlay that captures video from the screen,
//! moves a rendered image smoothly with velocity constraints, and bounces it off the edges of the screen.
//!
//! The overlay starts with a default size (1/5th of the screen width and height), and the user can change its
//! position by clicking the middle mouse button. Pressing ESC closes the overlay.
//!
//! The application handles mouse input, keyboard input (ESC), and continuous updates to simulate motion.
//! The captured video frame is rendered at the current position with size constraints.

use overlay::{
    Canvas, EventResult, MouseButton, OverlayApp, OverlayContext, OverlayEvent,
    image::capture::CaptureSession, run,
};

struct MyOverlayApp {
    capture_session: CaptureSession,
    x: f32,
    y: f32,
    w: i32,
    h: i32,
    vx: f32,
    vy: f32,
}

impl OverlayApp for MyOverlayApp {
    /// Initializes the component's dimensions and hides it from capture.
    ///
    /// This function sets the width and height of the component to one-fifth of the
    /// overlay context's dimensions. It also hides the component from capture to
    /// prevent it from being included in screen capture or rendering.
    fn init(&mut self, overlay_context: &mut OverlayContext) {
        self.w = overlay_context.width() / 5;
        self.h = overlay_context.height() / 5;
        overlay_context.hide_from_capture(true);
    }

    /// Handles overlay events and updates the overlay state accordingly.
    ///
    /// This function processes various overlay events such as mouse clicks and key presses.
    /// - When the middle mouse button is pressed, it records the current mouse position as the position (x, y).
    /// - When the ESC key is pressed, it closes the overlay.
    /// - All other events are propagated to other handlers.
    fn handler(
        &mut self,
        event: OverlayEvent,
        overlay_context: &mut OverlayContext,
    ) -> EventResult {
        match event {
            OverlayEvent::MouseDown {
                button: MouseButton::Middle,
            } => {
                let (x, y) = overlay_context.mouse_position();
                self.x = x as f32;
                self.y = y as f32;
                EventResult::Consumed
            }
            OverlayEvent::KeyDown { vk: 0x1B } => {
                //ESC
                overlay_context.close();
                EventResult::Consumed
            }
            _ => EventResult::Propagated,
        }
    }

    /// Updates the position of this object based on its velocity and the given time delta.
    ///
    /// This function advances the object's position by multiplying its velocity with the time delta.
    /// It also handles boundary collisions with the overlay context's boundaries.
    fn update(&mut self, overlay_context: &mut OverlayContext, delta: f32) {
        self.y += self.vy * delta;
        self.x += self.vx * delta;
        if self.x < 0.0 {
            self.x = 0.0;
            self.vx = self.vx.abs()
        }
        if self.x > (overlay_context.width() - self.w) as f32 {
            self.x = (overlay_context.width() - self.w) as f32;
            self.vx = -self.vx.abs();
        }
        if self.y < 0.0 {
            self.y = 0.0;
            self.vy = self.vy.abs()
        }
        if self.y > (overlay_context.height() - self.h) as f32 {
            self.y = (overlay_context.height() - self.h) as f32;
            self.vy = -self.vy.abs();
        }
    }
    /// Renders the current frame from the capture session onto the given canvas.
    ///
    /// If a frame is available from the capture session, this function clears the canvas
    /// and draws the frame scaled to fit within the specified rectangle defined by (x, y, w, h).
    fn render(&mut self, canvas: &mut Canvas) {
        if let Some(frame) = self.capture_session.capture() {
            canvas.clear();
            canvas.draw_image_scaled(&frame, self.x as i32, self.y as i32, self.w, self.h);
        }
    }
}

/// Main function that initializes and runs a window overlay application.
///
/// Creates a new `MyOverlayApp` instance with a capture session, initial position and velocity values.
/// The capture session is created using `CaptureSession::new()` and is expected to succeed.
/// The app's initial state sets the position (x, y) to (0.0, 0.0), size (w, h) to (0, 0),
/// and velocity (vx, vy) to (100.0, 42.0).
///
/// After initialization, the application is run using the `run` function, which handles the
/// application lifecycle and rendering loop.

fn main() {
    let app = MyOverlayApp {
        capture_session: CaptureSession::new().unwrap(),
        x: 0.0,
        y: 0.0,
        w: 0,
        h: 0,
        vx: 100.0,
        vy: 42.0,
    };
    run(app);
}