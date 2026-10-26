use std::os::windows::{ffi::OsStrExt, fs::OpenOptionsExt, io::AsRawHandle};
use std::{fs, path::Path};
use windows::Win32::{Foundation::HANDLE, Storage::FileSystem::*, System::IO::DeviceIoControl};
fn identity_of(file: &fs::File) -> (u64, [u8; 16]) {
    let mut info = FILE_ID_INFO::default();
    unsafe {
        GetFileInformationByHandleEx(
            HANDLE(file.as_raw_handle()),
            FileIdInfo,
            (&mut info as *mut FILE_ID_INFO).cast(),
            size_of::<FILE_ID_INFO>() as u32,
        )
    }
    .unwrap();
    (info.VolumeSerialNumber, info.FileId.Identifier)
}
pub fn set_junction(path: &Path, target: &Path) {
    // Modify the existing empty directory in place, with ScratchSpace's
    // no-share-delete pins still live. No rename/replacement shortcut.
    let file = fs::OpenOptions::new()
        .access_mode(FILE_WRITE_ATTRIBUTES.0)
        .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE).0)
        .custom_flags((FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS).0)
        .open(path)
        .unwrap();
    let before = identity_of(&file);
    let print: Vec<u16> = target.as_os_str().encode_wide().collect();
    let substitute: Vec<u16> = "\\??\\"
        .encode_utf16()
        .chain(print.iter().copied())
        .collect();
    let mut data = Vec::new();
    data.extend_from_slice(&0xA0000003u32.to_le_bytes()); // IO_REPARSE_TAG_MOUNT_POINT
    data.extend_from_slice(
        &u16::try_from(8 + (substitute.len() + print.len() + 2) * 2)
            .unwrap()
            .to_le_bytes(),
    );
    data.extend_from_slice(&0u16.to_le_bytes());
    for n in [
        0,
        substitute.len() * 2,
        (substitute.len() + 1) * 2,
        print.len() * 2,
    ] {
        data.extend_from_slice(&u16::try_from(n).unwrap().to_le_bytes());
    }
    for c in substitute.into_iter().chain([0]).chain(print).chain([0]) {
        data.extend_from_slice(&c.to_le_bytes());
    }
    let mut returned = 0;
    unsafe {
        DeviceIoControl(
            HANDLE(file.as_raw_handle()),
            0x000900A4,
            Some(data.as_ptr().cast()),
            data.len() as u32,
            None,
            0,
            Some(&mut returned),
            None,
        )
    }
    .expect("in-place mount-point fixture must be created; do not silently skip");
    assert_eq!(
        identity_of(&file),
        before,
        "fixture must retain original identity"
    );
}
