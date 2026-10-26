//! Separate development profile. No legacy Flet storage or music-tree writes.
use crate::job_owner::HostError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sorter_core::Settings;
use sorter_io::resources::load_aliases;
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
type Result<T> = std::result::Result<T, HostError>;
pub const MAX_CSV_BYTES: usize = 16 * 1024 * 1024;
const MAX_PROFILE_BYTES: u64 = 64 * 1024 * 1024;
const PROFILE_VERSION: &str = "0.1.0-preview";
fn error(code: &str, detail: impl std::fmt::Display) -> HostError {
    HostError::new(code, detail.to_string())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MainSettings {
    pub copy_mode: bool,
    pub main_folder_only: bool,
    pub singles_folder: bool,
    pub exist_only: bool,
    pub abc_sort: bool,
}
impl From<Settings> for MainSettings {
    fn from(s: Settings) -> Self {
        Self {
            copy_mode: s.copy_mode,
            main_folder_only: s.main_folder_only,
            singles_folder: s.singles_folder,
            exist_only: s.exist_only,
            abc_sort: s.abc_sort,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredSettings {
    copy_mode: bool,
    main_folder_only: bool,
    singles_folder: bool,
    exist_only: bool,
    abc_sort: bool,
    duet_mode: bool,
}
impl From<Settings> for StoredSettings {
    fn from(s: Settings) -> Self {
        Self {
            copy_mode: s.copy_mode,
            main_folder_only: s.main_folder_only,
            singles_folder: s.singles_folder,
            exist_only: s.exist_only,
            abc_sort: s.abc_sort,
            duet_mode: s.duet_mode,
        }
    }
}
impl From<StoredSettings> for Settings {
    fn from(s: StoredSettings) -> Self {
        Self {
            copy_mode: s.copy_mode,
            main_folder_only: s.main_folder_only,
            singles_folder: s.singles_folder,
            exist_only: s.exist_only,
            abc_sort: s.abc_sort,
            duet_mode: s.duet_mode,
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileData {
    schema_version: u16,
    settings: StoredSettings,
    first_run_seen_version: Option<String>,
    personal_rows: Vec<Vec<String>>,
}
/// The app starts without a singles subfolder. The planning contract keeps its
/// released defaults; only a fresh profile chooses differently.
fn app_default_settings() -> Settings {
    Settings {
        singles_folder: false,
        ..Settings::default()
    }
}
impl Default for ProfileData {
    fn default() -> Self {
        Self {
            schema_version: 1,
            settings: app_default_settings().into(),
            first_run_seen_version: None,
            personal_rows: vec![],
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Diagnostic {
    index: usize,
    code: &'static str,
    width: usize,
}
#[derive(Clone, Debug, Serialize)]
pub struct PersonalList {
    pub revision: String,
    pub rows: Vec<Vec<String>>,
    pub diagnostics: Vec<Diagnostic>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportEncoding {
    Utf8,
    Utf8Bom,
    Windows1255,
    Windows1252,
    Utf16Le,
    Utf16Be,
}
/// Explicit draft operations against original preview indices, never profile rows.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum ImportEdit {
    Set {
        index: usize,
        column: usize,
        value: String,
    },
    Delete {
        index: usize,
    },
}
#[derive(Clone, Debug, Serialize)]
pub struct ImportPreview {
    pub token: String,
    pub encoding: ImportEncoding,
    pub rows: Vec<Vec<String>>,
    pub diagnostics: Vec<Diagnostic>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ImportSelection {
    pub token: String,
    pub filename: String,
    pub encoding_required: bool,
    pub preview: Option<ImportPreview>,
}
#[derive(Serialize)]
pub struct FirstRunStatus {
    version: &'static str,
    pub seen: bool,
}
struct PendingImport {
    token: String,
    bytes: Vec<u8>,
    preview: Option<ImportPreview>,
}
struct Shared {
    directory: PathBuf,
    io: Mutex<()>,
    pending: Mutex<Option<PendingImport>>,
    serial: AtomicU64,
}
#[derive(Clone)]
pub struct ProfileStore {
    shared: Arc<Shared>,
}
impl ProfileStore {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            shared: Arc::new(Shared {
                directory,
                io: Mutex::new(()),
                pending: Mutex::new(None),
                serial: AtomicU64::new(0),
            }),
        }
    }
    fn access<T>(&self, write: bool, f: impl FnOnce(&mut ProfileData) -> Result<T>) -> Result<T> {
        let _local_lock = self
            .shared
            .io
            .lock()
            .map_err(|_| error("PROFILE_STATE", "Profile I/O state unavailable"))?;
        let _directory_guard = prepare_directory(&self.shared.directory)?;
        let lock_path = self.shared.directory.join("profile.lock");
        check_regular_or_missing(&lock_path)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(|e| error("PROFILE_IO", e))?;
        lock.try_lock().map_err(|e| error("PROFILE_BUSY", e))?;
        let path = self.shared.directory.join("profile.json");
        check_regular_or_missing(&path)?;
        let previous = match File::open(&path) {
            Ok(file) => {
                let mut bytes = vec![];
                file.take(MAX_PROFILE_BYTES + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|e| error("PROFILE_IO", e))?;
                if bytes.len() as u64 > MAX_PROFILE_BYTES {
                    return Err(error("PROFILE_TOO_LARGE", "Profile exceeds its size limit"));
                }
                Some(bytes)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(error("PROFILE_IO", e)),
        };
        let mut profile = match &previous {
            Some(bytes) => serde_json::from_slice::<ProfileData>(bytes)
                .map_err(|e| error("PROFILE_INVALID", e))?,
            None => ProfileData::default(),
        };
        if profile.schema_version != 1 {
            return Err(error(
                "PROFILE_SCHEMA",
                "Unsupported profile schema; no migration or overwrite performed",
            ));
        }
        let result = f(&mut profile)?;
        if write {
            let bytes =
                serde_json::to_vec_pretty(&profile).map_err(|e| error("PROFILE_INVALID", e))?;
            if bytes.len() as u64 > MAX_PROFILE_BYTES {
                return Err(error("PROFILE_TOO_LARGE", "Profile exceeds its size limit"));
            }
            if let Some(previous) = previous {
                replace_file(
                    &self.shared.directory.join("profile.previous.json"),
                    &previous,
                )?;
            }
            replace_file(&path, &bytes)?;
        }
        Ok(result)
    }
    pub fn settings(&self) -> Result<Settings> {
        self.access(false, |p| Ok(p.settings.clone().into()))
    }
    pub fn save_main(&self, s: MainSettings) -> Result<Settings> {
        self.access(true, |p| {
            p.settings.copy_mode = s.copy_mode;
            p.settings.main_folder_only = s.main_folder_only;
            p.settings.singles_folder = s.singles_folder;
            p.settings.exist_only = s.exist_only;
            p.settings.abc_sort = s.abc_sort;
            Ok(p.settings.clone().into())
        })
    }
    pub fn set_duet(&self, enabled: bool) -> Result<Settings> {
        self.access(true, |p| {
            p.settings.duet_mode = enabled;
            Ok(p.settings.clone().into())
        })
    }
    pub fn list(&self) -> Result<PersonalList> {
        self.access(false, |p| list(&p.personal_rows))
    }
    pub fn personal_bytes(&self) -> Result<Vec<u8>> {
        self.access(false, |p| encode_csv(&p.personal_rows))
    }
    pub fn save_list(&self, revision: &str, rows: Vec<Vec<String>>) -> Result<PersonalList> {
        if rows
            .iter()
            .any(|r| r.len() != 2 || r.iter().any(String::is_empty))
        {
            return Err(error(
                "INCOMPLETE_ROWS",
                "Every edited row must contain exactly a source name and a folder name. Remove or repair incomplete rows explicitly.",
            ));
        }
        let bytes = encode_csv(&rows)?;
        parse_rows(&bytes)?;
        self.access(true, |p| {
            if list(&p.personal_rows)?.revision != revision {
                return Err(error(
                    "LIST_CHANGED",
                    "Personal list changed; reload before saving",
                ));
            }
            p.personal_rows = rows;
            list(&p.personal_rows)
        })
    }
    pub fn first_run(&self) -> Result<FirstRunStatus> {
        self.access(false, |p| {
            Ok(FirstRunStatus {
                version: PROFILE_VERSION,
                seen: p.first_run_seen_version.as_deref() == Some(PROFILE_VERSION),
            })
        })
    }
    pub fn mark_first_run(&self) -> Result<FirstRunStatus> {
        self.access(true, |p| {
            p.first_run_seen_version = Some(PROFILE_VERSION.into());
            Ok(FirstRunStatus {
                version: PROFILE_VERSION,
                seen: true,
            })
        })
    }
    pub fn stage_import(&self, filename: String, bytes: Vec<u8>) -> Result<ImportSelection> {
        if bytes.len() > MAX_CSV_BYTES {
            return Err(error("CSV_TOO_LARGE", "CSV exceeds 16 MiB"));
        }
        let token = format!(
            "import-{}",
            self.shared.serial.fetch_add(1, Ordering::Relaxed)
        );
        let preview = if std::str::from_utf8(&bytes).is_ok() {
            Some(make_preview(&token, &bytes, ImportEncoding::Utf8)?)
        } else {
            None
        };
        let selection = ImportSelection {
            token: token.clone(),
            filename,
            encoding_required: preview.is_none(),
            preview: preview.clone(),
        };
        *self
            .shared
            .pending
            .lock()
            .map_err(|_| error("PROFILE_STATE", "Import state unavailable"))? =
            Some(PendingImport {
                token,
                bytes,
                preview,
            });
        Ok(selection)
    }
    pub fn preview_import(&self, token: &str, encoding: ImportEncoding) -> Result<ImportPreview> {
        let mut state = self
            .shared
            .pending
            .lock()
            .map_err(|_| error("PROFILE_STATE", "Import state unavailable"))?;
        let pending = state.as_mut().filter(|p| p.token == token).ok_or_else(|| {
            error(
                "INVALID_IMPORT_TOKEN",
                "Import selection is stale or unknown",
            )
        })?;
        let preview = make_preview(token, &pending.bytes, encoding)?;
        pending.preview = Some(preview.clone());
        Ok(preview)
    }
    pub fn confirm_import(
        &self,
        token: &str,
        encoding: ImportEncoding,
        revision: &str,
        edits: &[ImportEdit],
    ) -> Result<PersonalList> {
        let mut state = self
            .shared
            .pending
            .lock()
            .map_err(|_| error("PROFILE_STATE", "Import state unavailable"))?;
        let preview = state
            .as_ref()
            .filter(|p| p.token == token)
            .and_then(|p| p.preview.as_ref())
            .filter(|p| p.encoding == encoding)
            .ok_or_else(|| {
                error(
                    "IMPORT_NOT_PREVIEWED",
                    "Confirm the currently displayed encoding preview",
                )
            })?;
        let rows = edited_import_rows(&preview.rows, edits)?;
        let result = self.access(true, |p| {
            if list(&p.personal_rows)?.revision != revision {
                return Err(error(
                    "LIST_CHANGED",
                    "Personal list changed; reload before importing",
                ));
            }
            p.personal_rows.extend(rows);
            let bytes = encode_csv(&p.personal_rows)?;
            parse_rows(&bytes)?;
            list(&p.personal_rows)
        })?;
        *state = None;
        Ok(result)
    }
}
/// Validate all operations before touching a clone. No trimming, deduplication,
/// repair, or shifting indices: deletions are applied after original-index edits.
fn edited_import_rows(rows: &[Vec<String>], edits: &[ImportEdit]) -> Result<Vec<Vec<String>>> {
    if edits.len() > rows.len().saturating_mul(2) {
        return Err(error("IMPORT_EDIT_INVALID", "Too many draft operations"));
    }
    let mut touched = BTreeMap::<usize, u8>::new();
    let mut value_bytes = 0usize;
    for edit in edits {
        let (index, mask) = match edit {
            ImportEdit::Set {
                index,
                column,
                value,
            } => {
                if *column > 1 {
                    return Err(error(
                        "IMPORT_EDIT_INVALID",
                        "Only the first two columns are editable",
                    ));
                }
                value_bytes = value_bytes
                    .checked_add(value.len())
                    .filter(|n| *n <= MAX_CSV_BYTES)
                    .ok_or_else(|| error("CSV_TOO_LARGE", "Draft values exceed 16 MiB"))?;
                (*index, 1 << column)
            }
            ImportEdit::Delete { index } => (*index, 4),
        };
        if index >= rows.len() {
            return Err(error(
                "IMPORT_EDIT_INVALID",
                "Draft row index is outside the preview",
            ));
        }
        let previous = touched.entry(index).or_default();
        if (*previous & mask) != 0 || (*previous != 0 && (mask == 4 || *previous == 4)) {
            return Err(error(
                "IMPORT_EDIT_INVALID",
                "Duplicate or conflicting draft operations",
            ));
        }
        *previous |= mask;
    }
    let mut result = rows.to_vec();
    for edit in edits {
        if let ImportEdit::Set {
            index,
            column,
            value,
        } = edit
        {
            let row = &mut result[*index];
            if row.len() <= *column {
                row.resize(column + 1, String::new());
            }
            row[*column] = value.clone();
        }
    }
    let mut index = 0;
    result.retain(|_| {
        let keep = touched.get(&index) != Some(&4);
        index += 1;
        keep
    });
    Ok(result)
}
// Guard only our dedicated profile directory, not legitimate OS AppData ancestors.
// This is not a general hostile-ancestor filesystem sandbox.
fn ordinary_directory(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            // FILE_ATTRIBUTE_REPARSE_POINT
            return false;
        }
    }
    metadata.file_type().is_dir()
}
fn check_directory(path: &Path, missing_allowed: bool) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(m) if ordinary_directory(&m) => Ok(()),
        Ok(_) => Err(error(
            "PROFILE_INVALID",
            "Dedicated profile directory must not be a link or reparse point",
        )),
        Err(e) if missing_allowed && e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(error("PROFILE_IO", e)),
    }
}
fn prepare_directory(path: &Path) -> Result<File> {
    // Refuse existing redirects before creating even the lock file.
    check_directory(path, true)?;
    std::fs::create_dir_all(path).map_err(|e| error("PROFILE_IO", e))?;
    check_directory(path, false)?;
    #[cfg(windows)]
    let directory = {
        use std::os::windows::fs::OpenOptionsExt;
        // Attribute-only handles do not participate in Windows sharing checks.
        // Include read-only directory-data access so withholding delete/write
        // sharing actually prevents replacement while this guard is held.
        OpenOptions::new()
            .access_mode(0x1 | 0x80) // FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES
            .share_mode(1) // FILE_SHARE_READ; prevent replacement while held
            .custom_flags(0x02000000 | 0x00200000) // BACKUP_SEMANTICS | OPEN_REPARSE_POINT
            .open(path)
    };
    #[cfg(not(windows))]
    let directory = File::open(path);
    let directory = directory.map_err(|e| error("PROFILE_IO", e))?;
    if !ordinary_directory(&directory.metadata().map_err(|e| error("PROFILE_IO", e))?) {
        return Err(error(
            "PROFILE_INVALID",
            "Dedicated profile directory is not an ordinary directory",
        ));
    }
    check_directory(path, false)?;
    Ok(directory)
}
fn check_regular_or_missing(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(m) if !m.file_type().is_file() => Err(error(
            "PROFILE_INVALID",
            "Profile files must be ordinary files, not links or directories",
        )),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(error("PROFILE_IO", e)),
    }
}
fn replace_file(path: &Path, bytes: &[u8]) -> Result<()> {
    check_regular_or_missing(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| error("PROFILE_IO", "Missing profile directory"))?;
    #[cfg(windows)]
    return replace_profile_windows(parent, path, bytes).map_err(|e| error("PROFILE_IO", e));
    #[cfg(not(windows))]
    {
        let mut tmp =
            tempfile::NamedTempFile::new_in(parent).map_err(|e| error("PROFILE_IO", e))?;
        tmp.write_all(bytes)
            .and_then(|()| tmp.as_file().sync_all())
            .map_err(|e| error("PROFILE_IO", e))?;
        tmp.persist(path)
            .map_err(|e| error("PROFILE_IO", e.error))?;
        Ok(())
    }
}
#[cfg(windows)]
fn replace_profile_windows(parent: &Path, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::{
        mem::{offset_of, size_of},
        os::windows::{ffi::OsStrExt, fs::OpenOptionsExt, io::AsRawHandle},
    };
    use windows::{
        Wdk::Storage::FileSystem::{
            FILE_RENAME_INFORMATION, FileRenameInformation, NtSetInformationFile,
        },
        Win32::{
            Foundation::{HANDLE, RtlNtStatusToDosError},
            System::IO::IO_STATUS_BLOCK,
        },
    };
    // Only these fixed basenames can be replaced, in the temp file's own directory.
    let name = path
        .file_name()
        .filter(|name| *name == "profile.json" || *name == "profile.previous.json")
        .ok_or_else(|| std::io::Error::other("Invalid profile replacement name"))?;
    let name: Vec<u16> = name.encode_wide().collect();
    let mut tmp = tempfile::Builder::new().make_in(parent, |temporary| {
        // Create once with DELETE access for the handle-based rename. Normal file
        // attributes avoid tempfile's temporary attribute and any pathname reopen.
        OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .access_mode(0xc0010000) // GENERIC_READ | GENERIC_WRITE | DELETE
            .open(temporary)
    })?;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;

    // MoveFileExW reopens the destination parent for FILE_ADD_FILE, conflicting
    // with our strict no-write-sharing guard. Native NULL-root + basename rename
    // stays in the source directory without that reopen; keep the guard intact.
    let offset = offset_of!(FILE_RENAME_INFORMATION, FileName);
    let length = (offset + name.len() * 2).max(size_of::<FILE_RENAME_INFORMATION>());
    let mut allocation = vec![0usize; length.div_ceil(size_of::<usize>())];
    let info = allocation.as_mut_ptr().cast::<FILE_RENAME_INFORMATION>();
    // SAFETY: usize-aligned, zeroed storage covers the native header and bounded
    // UTF-16 name. The file handle and buffer live through this synchronous call.
    unsafe {
        (*info).Anonymous.ReplaceIfExists = true;
        (*info).RootDirectory = HANDLE::default();
        (*info).FileNameLength = (name.len() * 2) as u32;
        std::ptr::copy_nonoverlapping(
            name.as_ptr(),
            info.cast::<u8>().add(offset).cast::<u16>(),
            name.len(),
        );
    }
    let mut ios = IO_STATUS_BLOCK::default();
    let status = unsafe {
        NtSetInformationFile(
            HANDLE(tmp.as_file().as_raw_handle()),
            &mut ios,
            info.cast(),
            length as u32,
            FileRenameInformation,
        )
    };
    if status.0 < 0 {
        return Err(std::io::Error::from_raw_os_error(
            unsafe { RtlNtStatusToDosError(status) } as i32,
        ));
    }
    if status.0 != 0 {
        return Err(std::io::Error::other("Profile rename did not complete"));
    }
    // The original temp name no longer exists. Do not attempt pathname cleanup
    // after success (another process could subsequently reuse that old name).
    tmp.disable_cleanup(true);
    Ok(())
}
fn diagnostics(rows: &[Vec<String>]) -> Vec<Diagnostic> {
    rows.iter()
        .enumerate()
        .filter_map(|(index, r)| {
            if r.len() != 2 {
                Some(Diagnostic {
                    index,
                    code: "ALIAS_ROW_WIDTH",
                    width: r.len(),
                })
            } else if r.iter().any(String::is_empty) {
                Some(Diagnostic {
                    index,
                    code: "INCOMPLETE_ROW",
                    width: 2,
                })
            } else {
                None
            }
        })
        .collect()
}
fn list(rows: &[Vec<String>]) -> Result<PersonalList> {
    let bytes = encode_csv(rows)?;
    Ok(PersonalList {
        revision: format!("{:x}", Sha256::digest(bytes)),
        rows: rows.to_vec(),
        diagnostics: diagnostics(rows),
    })
}
fn parse_rows(bytes: &[u8]) -> Result<Vec<Vec<String>>> {
    load_aliases(&[], Some(bytes))
        .map(|b| b.ordered_rows.into_iter().map(|r| r.fields).collect())
        .map_err(|e| error("CSV_INVALID", e))
}
pub fn encode_csv(rows: &[Vec<String>]) -> Result<Vec<u8>> {
    let mut text = String::new();
    for row in rows {
        for (index, field) in row.iter().enumerate() {
            if index > 0 {
                text.push(',');
            }
            let quoted =
                field.contains([',', '"', '\r', '\n']) || (row.len() == 1 && field.is_empty());
            if quoted {
                text.push('"');
                for c in field.chars() {
                    if c == '"' {
                        text.push('"');
                    }
                    text.push(c);
                }
                text.push('"');
            } else {
                text.push_str(field);
            }
        }
        text.push_str("\r\n");
        if text.len() > MAX_CSV_BYTES {
            return Err(error("CSV_TOO_LARGE", "CSV exceeds 16 MiB"));
        }
    }
    Ok(text.into_bytes())
}
fn make_preview(token: &str, bytes: &[u8], encoding: ImportEncoding) -> Result<ImportPreview> {
    let (codec, bytes) = match encoding {
        ImportEncoding::Utf8 => (encoding_rs::UTF_8, bytes),
        ImportEncoding::Utf8Bom => (
            encoding_rs::UTF_8,
            bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes),
        ),
        ImportEncoding::Windows1255 => (encoding_rs::WINDOWS_1255, bytes),
        ImportEncoding::Windows1252 => (encoding_rs::WINDOWS_1252, bytes),
        ImportEncoding::Utf16Le => (
            encoding_rs::UTF_16LE,
            bytes.strip_prefix(&[0xff, 0xfe]).unwrap_or(bytes),
        ),
        ImportEncoding::Utf16Be => (
            encoding_rs::UTF_16BE,
            bytes.strip_prefix(&[0xfe, 0xff]).unwrap_or(bytes),
        ),
    };
    let text = codec
        .decode_without_bom_handling_and_without_replacement(bytes)
        .ok_or_else(|| {
            error(
                "ENCODING_INVALID",
                "Selected encoding cannot decode this file without replacement characters",
            )
        })?;
    if text.len() > MAX_CSV_BYTES {
        return Err(error("CSV_TOO_LARGE", "Decoded CSV exceeds 16 MiB"));
    }
    let rows = parse_rows(text.as_bytes())?;
    Ok(ImportPreview {
        token: token.into(),
        encoding,
        diagnostics: diagnostics(&rows),
        rows,
    })
}

/// Publish only a new, explicitly picked export. Existing files are never replaced.
pub fn export_new_csv(mut path: PathBuf, rows: &[Vec<String>]) -> Result<String> {
    if !path.is_absolute() {
        return Err(error(
            "INVALID_PATH",
            "Export requires an absolute selected path",
        ));
    }
    let bytes = encode_csv(rows)?;
    if !path.to_string_lossy().ends_with(".csv") {
        let mut name = path.into_os_string();
        name.push(".csv");
        path = name.into();
    }
    let parent = path
        .parent()
        .ok_or_else(|| error("INVALID_PATH", "Export directory missing"))?;
    let mut temp =
        tempfile::NamedTempFile::new_in(parent).map_err(|e| error("EXPORT_FAILED", e))?;
    temp.write_all(&bytes)
        .and_then(|()| temp.as_file().sync_all())
        .map_err(|e| error("EXPORT_FAILED", e))?;
    temp.persist_noclobber(&path).map_err(|e| {
        error(
            "EXPORT_FAILED",
            format!(
                "Choose an unused CSV filename; existing files are never overwritten: {}",
                e.error
            ),
        )
    })?;
    Ok(path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn store() -> (tempfile::TempDir, ProfileStore) {
        let directory = tempfile::tempdir().unwrap();
        let store = ProfileStore::new(directory.path().join("preview-profile"));
        (directory, store)
    }
    #[cfg(unix)]
    #[test]
    fn dedicated_directory_link_rejected_before_any_write() {
        let temp = tempfile::tempdir().unwrap();
        let outside = temp.path().join("outside");
        std::fs::create_dir(&outside).unwrap();
        let linked = temp.path().join("profile-v1");
        std::os::unix::fs::symlink(&outside, &linked).unwrap();
        let store = ProfileStore::new(linked);
        assert!(store.set_duet(true).is_err());
        assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0);
        std::fs::write(outside.join("profile.json"), b"keep current").unwrap();
        std::fs::write(outside.join("profile.previous.json"), b"keep backup").unwrap();
        assert!(store.set_duet(false).is_err());
        assert_eq!(
            std::fs::read(outside.join("profile.json")).unwrap(),
            b"keep current"
        );
        assert_eq!(
            std::fs::read(outside.join("profile.previous.json")).unwrap(),
            b"keep backup"
        );
        assert!(!outside.join("profile.lock").exists());
    }
    #[cfg(unix)]
    #[test]
    fn legitimate_appdata_ancestor_link_is_allowed() {
        let temp = tempfile::tempdir().unwrap();
        let real = temp.path().join("appdata-real");
        std::fs::create_dir(&real).unwrap();
        let redirected = temp.path().join("appdata");
        std::os::unix::fs::symlink(&real, &redirected).unwrap();
        let store = ProfileStore::new(redirected.join("profile-v1"));
        assert!(store.set_duet(true).unwrap().duet_mode);
        assert!(real.join("profile-v1/profile.json").is_file());
    }
    #[cfg(windows)]
    #[test]
    fn dedicated_directory_junction_rejected_before_any_write() {
        let temp = tempfile::tempdir().unwrap();
        let outside = temp.path().join("outside");
        std::fs::create_dir(&outside).unwrap();
        let linked = temp.path().join("profile-v1");
        let result = std::process::Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&linked)
            .arg(&outside)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "owned junction fixture creation failed"
        );
        let store = ProfileStore::new(linked);
        assert!(store.set_duet(true).is_err());
        assert_eq!(std::fs::read_dir(&outside).unwrap().count(), 0);
        std::fs::write(outside.join("profile.json"), b"keep current").unwrap();
        std::fs::write(outside.join("profile.previous.json"), b"keep backup").unwrap();
        assert!(store.set_duet(false).is_err());
        assert_eq!(
            std::fs::read(outside.join("profile.json")).unwrap(),
            b"keep current"
        );
        assert_eq!(
            std::fs::read(outside.join("profile.previous.json")).unwrap(),
            b"keep backup"
        );
        assert!(!outside.join("profile.lock").exists());
    }
    #[cfg(windows)]
    #[test]
    fn directory_guard_prevents_rename_until_released() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("profile-v1");
        let guard = prepare_directory(&path).unwrap();
        let moved = temp.path().join("moved");
        assert!(std::fs::rename(&path, &moved).is_err());
        drop(guard);
        std::fs::rename(&path, &moved).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn directory_guard_allows_atomic_profile_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("profile-v1");
        let guard = prepare_directory(&path).unwrap();
        let current = path.join("profile.json");
        let backup = path.join("profile.previous.json");
        replace_file(&current, b"first").unwrap();
        replace_file(&backup, b"first").unwrap();
        replace_file(&current, b"second").unwrap();
        assert_eq!(std::fs::read(&current).unwrap(), b"second");
        assert_eq!(std::fs::read(&backup).unwrap(), b"first");
        assert_eq!(std::fs::read_dir(&path).unwrap().count(), 2);
        let moved = temp.path().join("moved");
        assert!(std::fs::rename(&path, &moved).is_err());
        drop(guard);
        std::fs::rename(&path, &moved).unwrap();
    }
    #[test]
    fn defaults_false_and_independent_save_timing() {
        let (_directory, store) = store();
        assert_eq!(store.settings().unwrap(), app_default_settings());
        assert!(!store.settings().unwrap().singles_folder);
        let mut first = MainSettings::from(app_default_settings());
        first.singles_folder = true;
        first.copy_mode = true;
        store.set_duet(true).unwrap();
        assert!(!store.settings().unwrap().copy_mode);
        let saved = store.save_main(first).unwrap();
        assert!(saved.duet_mode);
        assert!(saved.singles_folder);
        assert!(saved.copy_mode);
        assert!(!store.set_duet(false).unwrap().duet_mode);
        assert!(store.settings().unwrap().copy_mode);
    }
    #[test]
    fn ordered_duplicate_rows_and_conflicts() {
        let (_directory, store) = store();
        let old = store.list().unwrap();
        let rows = vec![
            vec!["אמן".into(), "א".into()],
            vec!["אמן".into(), "ב".into()],
        ];
        let saved = store.save_list(&old.revision, rows.clone()).unwrap();
        assert_eq!(saved.rows, rows);
        assert!(store.save_list(&old.revision, vec![]).is_err());
        assert!(
            store
                .save_list(&saved.revision, vec![vec!["א".into(), "".into()]])
                .is_err()
        );
        assert_eq!(store.list().unwrap().rows, rows);
    }
    #[test]
    fn import_requires_confirm_and_preserves_malformed_rows() {
        let (_directory, store) = store();
        let before = store.list().unwrap();
        let selected = store
            .stage_import(
                "test.csv".into(),
                b"a,b\r\n\r\nextra,field,third\r\na,b\r\n".to_vec(),
            )
            .unwrap();
        let preview = selected.preview.unwrap();
        assert_eq!(store.list().unwrap().rows, before.rows);
        let imported = store
            .confirm_import(&preview.token, preview.encoding, &before.revision, &[])
            .unwrap();
        assert_eq!(imported.rows.len(), 4);
        assert!(imported.rows[1].is_empty());
        assert_eq!(imported.rows[2].len(), 3);
        assert_eq!(imported.rows[0], imported.rows[3]);
        assert!(
            store
                .confirm_import(&preview.token, preview.encoding, &imported.revision, &[])
                .is_err()
        );
    }
    fn set(index: usize, column: usize, value: &str) -> ImportEdit {
        ImportEdit::Set {
            index,
            column,
            value: value.into(),
        }
    }
    #[test]
    fn import_draft_preserves_original_indices_extras_and_duplicates() {
        let (directory, store) = store();
        let source = directory.path().join("source.csv");
        let bytes = b"drop,me\noriginal,target,extra\n\nsingle\ndup,same\ndup,same\n";
        std::fs::write(&source, bytes).unwrap();
        let before = store.list().unwrap();
        let preview = store
            .stage_import("source.csv".into(), std::fs::read(&source).unwrap())
            .unwrap()
            .preview
            .unwrap();
        let edits = [
            ImportEdit::Delete { index: 0 },
            set(1, 0, "edited"),
            set(1, 1, "quoted,\"target\"\n"),
            set(2, 1, "from blank"),
            set(3, 1, "new"),
        ];
        let saved = store
            .confirm_import(&preview.token, preview.encoding, &before.revision, &edits)
            .unwrap();
        assert_eq!(
            saved.rows,
            vec![
                vec!["edited", "quoted,\"target\"\n", "extra"],
                vec!["", "from blank"],
                vec!["single", "new"],
                vec!["dup", "same"],
                vec!["dup", "same"]
            ]
        );
        assert_eq!(std::fs::read(&source).unwrap(), bytes);
        assert_eq!(store.list().unwrap().rows, saved.rows);
        assert_eq!(
            store
                .confirm_import(&preview.token, preview.encoding, &saved.revision, &[])
                .unwrap_err()
                .code,
            "IMPORT_NOT_PREVIEWED"
        );
    }
    #[test]
    fn invalid_import_edits_leave_preview_and_profile_unchanged() {
        let (_directory, store) = store();
        let before = store.list().unwrap();
        let preview = store
            .stage_import("test.csv".into(), b"a,b\nc,d\n".to_vec())
            .unwrap()
            .preview
            .unwrap();
        let cases = vec![
            vec![set(2, 0, "bad")],
            vec![set(0, 2, "bad")],
            vec![set(0, 0, "one"), set(0, 0, "two")],
            vec![
                ImportEdit::Delete { index: 0 },
                ImportEdit::Delete { index: 0 },
            ],
            vec![set(0, 1, "changed"), ImportEdit::Delete { index: 0 }],
            vec![ImportEdit::Delete { index: 0 }, set(0, 0, "changed")],
            vec![set(0, 0, "too many"); 5],
        ];
        for edits in cases {
            assert_eq!(
                store
                    .confirm_import(&preview.token, preview.encoding, &before.revision, &edits)
                    .unwrap_err()
                    .code,
                "IMPORT_EDIT_INVALID"
            );
            assert_eq!(store.list().unwrap().rows, before.rows);
            assert_eq!(
                store
                    .preview_import(&preview.token, preview.encoding)
                    .unwrap()
                    .rows,
                preview.rows
            );
        }
        let saved = store
            .confirm_import(
                &preview.token,
                preview.encoding,
                &before.revision,
                &[
                    ImportEdit::Delete { index: 0 },
                    ImportEdit::Delete { index: 1 },
                ],
            )
            .unwrap();
        assert!(saved.rows.is_empty());
    }
    #[test]
    fn import_draft_failures_are_retryable_and_do_not_mutate_preview() {
        let (_directory, store) = store();
        let before = store.list().unwrap();
        let preview = store
            .stage_import("test.csv".into(), b"a,b\n".to_vec())
            .unwrap()
            .preview
            .unwrap();
        let edits = [set(0, 1, "changed")];
        assert_eq!(
            store
                .confirm_import(
                    &preview.token,
                    ImportEncoding::Windows1252,
                    &before.revision,
                    &edits
                )
                .unwrap_err()
                .code,
            "IMPORT_NOT_PREVIEWED"
        );
        let current = store
            .save_list(
                &before.revision,
                vec![vec!["existing".into(), "row".into()]],
            )
            .unwrap();
        assert_eq!(
            store
                .confirm_import(&preview.token, preview.encoding, &before.revision, &edits)
                .unwrap_err()
                .code,
            "LIST_CHANGED"
        );
        assert_eq!(store.list().unwrap().rows, current.rows);
        for value in [
            "x".repeat(MAX_CSV_BYTES + 1),
            "\"".repeat(MAX_CSV_BYTES / 2),
        ] {
            assert_eq!(
                store
                    .confirm_import(
                        &preview.token,
                        preview.encoding,
                        &current.revision,
                        &[set(0, 0, &value)]
                    )
                    .unwrap_err()
                    .code,
                "CSV_TOO_LARGE"
            );
            assert_eq!(store.list().unwrap().rows, current.rows);
        }
        assert_eq!(
            store
                .preview_import(&preview.token, preview.encoding)
                .unwrap()
                .rows,
            preview.rows
        );
        let saved = store
            .confirm_import(&preview.token, preview.encoding, &current.revision, &edits)
            .unwrap();
        assert_eq!(
            saved.rows,
            vec![vec!["existing", "row"], vec!["a", "changed"]]
        );
    }
    #[test]
    fn mixed_draft_keeps_untouched_malformed_rows_and_only_extends_explicit_column() {
        let rows = vec![
            vec!["remove".into()],
            vec![],
            vec!["one".into()],
            vec!["a".into(), "b".into(), "extra".into()],
            vec![],
        ];
        let edited = edited_import_rows(
            &rows,
            &[ImportEdit::Delete { index: 0 }, set(4, 0, "first")],
        )
        .unwrap();
        assert_eq!(
            edited,
            vec![vec![], vec!["one"], vec!["a", "b", "extra"], vec!["first"]]
        );
    }
    #[test]
    fn edited_import_persistence_failure_retains_token_and_success_backs_up_previous() {
        let (directory, store) = store();
        store.set_duet(true).unwrap();
        let before = store.list().unwrap();
        let profile_path = directory.path().join("preview-profile");
        let previous = std::fs::read(profile_path.join("profile.json")).unwrap();
        let preview = store
            .stage_import("test.csv".into(), b"a,b\n".to_vec())
            .unwrap()
            .preview
            .unwrap();
        let edits = [set(0, 1, "changed")];
        let backup = profile_path.join("profile.previous.json");
        std::fs::create_dir(&backup).unwrap();
        assert!(
            store
                .confirm_import(&preview.token, preview.encoding, &before.revision, &edits)
                .is_err()
        );
        assert_eq!(
            std::fs::read(profile_path.join("profile.json")).unwrap(),
            previous
        );
        assert_eq!(
            store
                .preview_import(&preview.token, preview.encoding)
                .unwrap()
                .rows,
            preview.rows
        );
        std::fs::remove_dir(&backup).unwrap();
        let saved = store
            .confirm_import(&preview.token, preview.encoding, &before.revision, &edits)
            .unwrap();
        assert_eq!(saved.rows, vec![vec!["a", "changed"]]);
        assert_eq!(std::fs::read(backup).unwrap(), previous);
    }
    #[test]
    fn import_edit_wire_rejects_malformed_operations() {
        for payload in [
            r#"{"kind":"set","index":0,"column":0}"#,
            r#"{"kind":"set","index":-1,"column":0,"value":"x"}"#,
            r#"{"kind":"set","index":0,"column":0.5,"value":"x"}"#,
            r#"{"kind":"delete","index":0,"value":"x"}"#,
            r#"{"kind":"replace","index":0}"#,
            r#"{"kind":"set","index":0,"column":0,"value":null}"#,
        ] {
            assert!(
                serde_json::from_str::<ImportEdit>(payload).is_err(),
                "{payload}"
            );
        }
        assert!(matches!(
            serde_json::from_str::<ImportEdit>(
                r#"{"kind":"set","index":1,"column":0,"value":"x"}"#
            )
            .unwrap(),
            ImportEdit::Set {
                index: 1,
                column: 0,
                ..
            }
        ));
        assert!(matches!(
            serde_json::from_str::<ImportEdit>(r#"{"kind":"delete","index":1}"#).unwrap(),
            ImportEdit::Delete { index: 1 }
        ));
    }
    #[test]
    fn explicit_encoding_and_stale_import_token() {
        let (_directory, store) = store();
        let selected = store
            .stage_import("hebrew.csv".into(), vec![0xe0, b',', 0xe1, b'\n'])
            .unwrap();
        assert!(selected.encoding_required);
        assert!(selected.preview.is_none());
        assert!(
            store
                .preview_import(&selected.token, ImportEncoding::Utf8)
                .is_err()
        );
        let preview = store
            .preview_import(&selected.token, ImportEncoding::Windows1255)
            .unwrap();
        assert_eq!(preview.rows, vec![vec!["א", "ב"]]);
        store
            .stage_import("new.csv".into(), b"c,d\n".to_vec())
            .unwrap();
        assert!(
            store
                .preview_import(&selected.token, ImportEncoding::Windows1255)
                .is_err()
        );
    }
    #[test]
    fn csv_roundtrip_quotes_newlines_empty_and_bom() {
        let rows = vec![
            vec![],
            vec![String::new()],
            vec!["a,b".into(), "c\"d\r\ne".into()],
            vec!["\u{feff}א".into(), "ב".into()],
        ];
        let bytes = encode_csv(&rows).unwrap();
        assert_eq!(
            parse_rows(&bytes).unwrap(),
            vec![
                vec![],
                vec![String::new()],
                vec!["a,b".into(), "c\"d\ne".into()],
                vec!["\u{feff}א".into(), "ב".into()]
            ]
        );
    }
    #[test]
    fn corrupted_profile_never_overwritten_and_no_root_persistence() {
        let (directory, store) = store();
        store.set_duet(true).unwrap();
        let path = directory.path().join("preview-profile/profile.json");
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("root"));
        assert!(!text.contains("token"));
        std::fs::write(&path, b"invalid").unwrap();
        assert!(store.set_duet(false).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"invalid");
    }
    #[test]
    fn first_run_marker_changes_only_on_acknowledgement() {
        let (_directory, store) = store();
        assert!(!store.first_run().unwrap().seen);
        assert!(!store.first_run().unwrap().seen);
        assert!(store.mark_first_run().unwrap().seen);
        assert!(store.first_run().unwrap().seen);
    }
    #[test]
    fn cross_instance_lock_and_previous_profile_are_preserved() {
        let (directory, store) = store();
        store.set_duet(true).unwrap();
        let profile_dir = directory.path().join("preview-profile");
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .open(profile_dir.join("profile.lock"))
            .unwrap();
        lock.try_lock().unwrap();
        assert_eq!(store.settings().err().unwrap().code, "PROFILE_BUSY");
        drop(lock);
        store.set_duet(false).unwrap();
        let previous: ProfileData = serde_json::from_slice(
            &std::fs::read(profile_dir.join("profile.previous.json")).unwrap(),
        )
        .unwrap();
        assert!(previous.settings.duet_mode);
        assert!(!store.settings().unwrap().duet_mode);
    }
    #[test]
    fn unknown_schema_and_string_booleans_are_not_migrated() {
        let (directory, store) = store();
        store.set_duet(false).unwrap();
        let path = directory.path().join("preview-profile/profile.json");
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        value["settings"]["duet_mode"] = "false".into();
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(store.settings().err().unwrap().code, "PROFILE_INVALID");
        value["settings"]["duet_mode"] = false.into();
        value["schema_version"] = 2.into();
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(store.set_duet(true).err().unwrap().code, "PROFILE_SCHEMA");
    }
    #[test]
    fn import_conflict_preserves_pending_preview_without_duplicate_append() {
        let (_directory, store) = store();
        let old = store.list().unwrap();
        let selected = store
            .stage_import("test.csv".into(), b"a,b\n".to_vec())
            .unwrap();
        let current = store
            .save_list(&old.revision, vec![vec!["c".into(), "d".into()]])
            .unwrap();
        assert_eq!(
            store
                .confirm_import(&selected.token, ImportEncoding::Utf8, &old.revision, &[])
                .err()
                .unwrap()
                .code,
            "LIST_CHANGED"
        );
        assert_eq!(store.list().unwrap().rows, current.rows);
        let done = store
            .confirm_import(
                &selected.token,
                ImportEncoding::Utf8,
                &current.revision,
                &[],
            )
            .unwrap();
        assert_eq!(done.rows.len(), 2);
        assert_eq!(store.personal_bytes().unwrap(), b"c,d\r\na,b\r\n");
    }
    #[test]
    fn export_publishes_complete_new_csv_and_never_overwrites() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("רשימה");
        let rows = vec![vec!["אמן".into(), "תיקייה".into()]];
        let name = export_new_csv(path.clone(), &rows).unwrap();
        assert!(name.ends_with(".csv"));
        let original = std::fs::read(&name).unwrap();
        assert_eq!(parse_rows(&original).unwrap(), rows);
        assert!(export_new_csv(path, &[vec!["different".into(), "value".into()]]).is_err());
        assert_eq!(std::fs::read(name).unwrap(), original);
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
        assert!(export_new_csv(PathBuf::from("relative.csv"), &rows).is_err());
    }
    #[test]
    fn simultaneous_local_reads_serialize_without_false_busy_errors() {
        let (_directory, store) = store();
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let cloned = store.clone();
                std::thread::spawn(move || {
                    for _ in 0..5 {
                        assert_eq!(cloned.settings().unwrap(), app_default_settings());
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
    }
}
