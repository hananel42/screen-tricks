//! A Rust library for creating and managing overlay applications on Windows using the Win32 API.
//!
//! This module provides a high-level interface for building real-time overlays that can capture screen content,
//! render graphics, and respond to user input. It abstracts low-level Win32 operations while offering a clean,
//! idiomatic Rust API.
//!
//!
//! ## Public Types and Functions
//!
//! - `Canvas`: A rendering surface for drawing visuals (e.g., text, shapes) on the overlay.
//! - `CaptureSession`: A session for capturing screen frames, enabling real-time video or image streaming.
//! - `FrameImage`: A captured frame of screen content, typically represented as a pixel buffer.
//! - `ImageSource`: Defines where screen content is sourced from (e.g., full screen, specific window).
//! - `ImageView`: A view into a captured image, allowing for manipulation or display.
//! - `EventResult`: Result type for event handling, indicating success or failure.
//! - `MouseButton`: Enum representing mouse button states (e.g., left, right, middle).
//! - `OverlayApp`: A trait or struct representing a full overlay application.
//! - `OverlayContext`: Context object for managing overlay lifecycle and state.
//! - `OverlayEvent`: Event types that the overlay can respond to (e.g., mouse movement, click).
//! - `run()`: Entry point function to start and run an overlay application.
//!
//!
//! note: This library is designed exclusively for Windows and relies on Win32 APIs. It does not support
//! cross-platform operation or other operating systems.
//!
//! # Example
//! ```rust,no_run
#![doc=include_str!("../examples/simple_video_rect.rs")]
//! ```


mod canvas;
pub mod capture;
mod state;
mod win32;

pub use win32::{EventResult, MouseButton, OverlayApp, OverlayContext, OverlayEvent, run};
pub use canvas::Canvas;
