#![allow(dead_code)]
use sorter_io::{desktop::*, storage::*};
use std::path::Path;
#[cfg(windows)]
pub type TestDesktop = DesktopStorage;
#[cfg(target_os = "linux")]
pub type TestDesktop = LinuxTestStorage;
pub fn reference(storage: &mut TestDesktop, path: &Path) -> StorageRef {
    storage.reference(path).unwrap()
}
pub fn synthetic_tree() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("sorter-synthetic-")
        .tempdir()
        .unwrap()
}

#[cfg(windows)]
pub mod windows_reparse;
