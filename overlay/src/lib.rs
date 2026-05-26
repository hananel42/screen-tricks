#![cfg(target_os = "windows")]
//! # Windows Win32 Overlay Library
//!
//! A specialized Rust library for creating, managing, and rendering high-performance
//! hardware/software overlays on Windows platforms using the native Win32 API.
//!
//! This crate abstracts low-level Windows windowing mechanics, window styles (`WS_EX_LAYERED`, `WS_EX_TRANSPARENT`),
//! and GDI loops, providing a safe, idiomatic, and real-time rendering loop.
//!
//! ## Core Architecture Components
//!
//! * [`Canvas`]: The main software rendering surface. Houses pixel manipulation buffers, text rasterization, and image blitting.
//! * [`OverlayApp`]: A trait implemented by users to handle core overlay state mutations and receive lifecycle events.
//! * [`OverlayContext`]: A handle passed to events allowing runtime modification of the window (e.g., closing, resizing, repositioning).
//! * [`OverlayEvent`]: OS events piped directly into the overlay execution frame loop (e.g., mouse interaction, moving).
//! * [`run`]: The library entry-point. Spawns the window thread, registers window classes, and starts the message pump.
//!
//! ## Platform Support
//!
//! This crate is strictly bound to Windows desktop architectures. It will intentionally fail to compile
//! on non-Windows platforms.
//!
//! # Examples
//!
//! ```rust,no_run
#![doc = include_str!("../examples/simple_video_rect.rs")]
//! ```

mod canvas;
pub mod image;
mod state;
mod win32;

pub use canvas::Canvas;
pub use win32::{EventResult, MouseButton, OverlayApp, OverlayContext, OverlayEvent, run};
