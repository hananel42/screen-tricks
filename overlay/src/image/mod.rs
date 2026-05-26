//! # High-Performance Image Handling and Screen Capture Subsystem
//!
//! This module provides synchronous, low-latency utilities for capturing, scaling,
//! and manipulating image frame buffers directly in user-space memory.
//!
//! ## Sub-Modules and Architecture
//!
//! * **`CaptureSession`**: Interacts with the native Win32 GDI layer to poll high-speed
//!   desktop frame captures. It operates with zero-copy overhead by reusing a persistent,
//!   memory-mapped Device-Independent Bitmap (DIB).
//! * **`ImageView` / `FrameImage`**: Abstract wrappers representing structured 32-bit pixel surfaces,
//!   providing continuous slice access for fast scaling, blending, and texture mapping.
//! * **`ImageSource`**: The unifying trait that abstracts pixel data storage, enabling the
//!   [`Canvas`](crate::Canvas) engine to read seamlessly from both live captured streams and static assets.

pub mod common;
pub mod capture;
pub mod frames;

pub use common::*;
pub use frames::*;
pub use capture::*;