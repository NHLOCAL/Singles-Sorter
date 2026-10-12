//! Read-only adapters and separate mutation contracts for the native rebuild.
//! Experimental Windows primitives are opt-in and restricted to owned test trees.
pub mod resources;

pub mod storage;

#[cfg(any(windows, target_os = "linux"))]
pub mod desktop;

pub mod metadata;

/// Separate execution contract; read-only adapters never acquire these rights.
pub mod mutation;

/// Bounded staged metadata transforms; no live source writes or path authority.
pub mod staged_tags;

/// Stream-only native FLAC repair; no live source writes or path authority.
pub mod staged_flac;

/// Product dispatch shared by platform backends; no filesystem authority.
pub mod staged_repair;
