//! Read-only adapters for the native rebuild. Physical execution is not exposed.
pub mod resources;

pub mod storage;

#[cfg(any(windows, target_os = "linux"))]
pub mod desktop;

pub mod metadata;
