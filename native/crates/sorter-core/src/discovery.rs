//! Version-2 read observations. These types do not change the v1 supplied-facts protocol.
use std::io::{self, Read, Seek, SeekFrom};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum StorageRef {
    WindowsPath {
        utf16: Vec<u16>,
    },
    AndroidDocument {
        tree_uri: String,
        authority: String,
        document_id: String,
    },
    /// Only meaningful to explicitly synthetic/test adapters, never a host path.
    TestOpaque {
        id: String,
    },
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ObjectIdentity {
    Windows {
        volume: u64,
        file_id: [u8; 16],
    },
    Android {
        authority: String,
        document_id: String,
    },
    Test {
        id: String,
    },
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
    LinkOrReparse,
    Other,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct Capabilities {
    pub read: Option<bool>,
    pub list: Option<bool>,
    pub seek: Option<bool>,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct EntryObservation {
    pub reference: StorageRef,
    pub identity: Option<ObjectIdentity>,
    pub parent_identity: Option<ObjectIdentity>,
    /// Display only. Never reconstruct a reference from this potentially lossy text.
    pub name: String,
    pub kind: EntryKind,
    pub size: Option<u64>,
    pub modified: Option<String>,
    pub capabilities: Capabilities,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyBlocker {
    PhysicalApplyUnavailable,
    LinkOrReparse,
    MissingIdentity,
    UnknownCapability,
}
impl EntryObservation {
    pub fn apply_blockers(&self) -> Vec<ApplyBlocker> {
        let mut result = vec![ApplyBlocker::PhysicalApplyUnavailable];
        if self.kind == EntryKind::LinkOrReparse {
            result.push(ApplyBlocker::LinkOrReparse);
        }
        if self.identity.is_none() {
            result.push(ApplyBlocker::MissingIdentity);
        }
        if self.capabilities.read.is_none()
            || self.capabilities.list.is_none()
            || self.capabilities.seek.is_none()
        {
            result.push(ApplyBlocker::UnknownCapability);
        }
        result
    }
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageErrorKind {
    NotFound,
    PermissionDenied,
    Locked,
    RevokedGrant,
    UnsupportedReference,
    UnsupportedCapability,
    InvalidReference,
    UnsafeLink,
    OutsideRoots,
    NotDirectory,
    NotFile,
    Changed,
    BudgetExceeded,
    Cancelled,
    DiskFull,
    Io,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct StorageError {
    pub kind: StorageErrorKind,
    pub detail: String,
}
impl StorageError {
    pub fn new(kind: StorageErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }
}
impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.detail)
    }
}
impl std::error::Error for StorageError {}
impl From<io::Error> for StorageError {
    fn from(e: io::Error) -> Self {
        if let Some(inner) = e.get_ref().and_then(|e| e.downcast_ref::<StorageError>()) {
            return inner.clone();
        }
        let kind = match e.kind() {
            io::ErrorKind::NotFound => StorageErrorKind::NotFound,
            io::ErrorKind::PermissionDenied => StorageErrorKind::PermissionDenied,
            io::ErrorKind::StorageFull => StorageErrorKind::DiskFull,
            io::ErrorKind::Interrupted => StorageErrorKind::Cancelled,
            _ => StorageErrorKind::Io,
        };
        #[cfg(windows)]
        let kind = if matches!(e.raw_os_error(), Some(32 | 33)) {
            StorageErrorKind::Locked
        } else {
            kind
        };
        Self::new(kind, e.to_string())
    }
}
/// No mutation methods are available, even through a trait object.
/// ```compile_fail
/// use sorter_core::discovery::{ReadStorage, StorageRef};
/// fn cannot_write(storage: &mut dyn ReadStorage, reference: &StorageRef) {
///     storage.open_write(reference);
/// }
/// ```
pub trait ReadStorage {
    fn stat(&mut self, reference: &StorageRef) -> Result<EntryObservation, StorageError>;
    /// Provider encounter order; partial failures are errors, not silently missing entries.
    fn children(&mut self, reference: &StorageRef) -> Result<Vec<EntryObservation>, StorageError>;
    fn open_read(&mut self, reference: &StorageRef) -> Result<ReadInput, StorageError>;
}
pub trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}
enum InputSource {
    Seekable(Box<dyn ReadSeek + Send>),
    Streaming(Box<dyn Read + Send>),
}
/// Evidence obtained from the same open handle or immutable snapshot as a read.
/// An adapter must not construct this from a separate path stat or from the
/// caller's earlier observation. It does not promise immutable file contents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadBinding {
    pub identity: ObjectIdentity,
    pub size: u64,
    pub modified: String,
}
/// Read-only view bounded by the observed length (seekable) or a stream byte limit.
/// Fields are private: callers cannot recover a writable underlying file.
pub struct ReadInput {
    source: InputSource,
    position: u64,
    limit: u64,
    overflowed: bool,
    binding: Option<ReadBinding>,
}
impl ReadInput {
    /// Always reads relative to offset zero, independent of the source cursor.
    pub fn seekable(source: impl ReadSeek + Send + 'static, length: u64) -> Self {
        Self {
            source: InputSource::Seekable(Box::new(source)),
            position: 0,
            limit: length,
            overflowed: false,
            binding: None,
        }
    }
    pub fn streaming(source: impl Read + Send + 'static, limit: u64) -> Self {
        Self {
            source: InputSource::Streaming(Box::new(source)),
            position: 0,
            limit,
            overflowed: false,
            binding: None,
        }
    }
    /// Attach adapter-verified evidence for this exact input source. Generic
    /// readers remain unbound and cannot establish storage-metadata provenance.
    pub fn with_binding(mut self, binding: ReadBinding) -> Self {
        self.binding = Some(binding);
        self
    }
    pub fn binding(&self) -> Option<&ReadBinding> {
        self.binding.as_ref()
    }
    pub fn is_seekable(&self) -> bool {
        matches!(self.source, InputSource::Seekable(_))
    }
    pub fn bound(&self) -> u64 {
        self.limit
    }
}
impl Read for ReadInput {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.overflowed {
            return Err(io::Error::other(StorageError::new(
                StorageErrorKind::BudgetExceeded,
                "stream exceeded its byte limit",
            )));
        }
        let count = (self.limit - self.position).min(buf.len() as u64) as usize;
        if count == 0 {
            return match &mut self.source {
                InputSource::Seekable(_) => Ok(0),
                InputSource::Streaming(r) => {
                    let mut probe = [0];
                    if r.read(&mut probe)? == 0 {
                        Ok(0)
                    } else {
                        self.overflowed = true;
                        Err(io::Error::other(StorageError::new(
                            StorageErrorKind::BudgetExceeded,
                            "stream exceeded its byte limit",
                        )))
                    }
                }
            };
        }
        let n = match &mut self.source {
            InputSource::Seekable(r) => {
                r.seek(SeekFrom::Start(self.position))?;
                let n = r.read(&mut buf[..count])?;
                if n == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "bounded source was truncated",
                    ));
                }
                Ok(n)
            }
            InputSource::Streaming(r) => r.read(&mut buf[..count]),
        }?;
        self.position += n as u64;
        Ok(n)
    }
}
impl Seek for ReadInput {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let InputSource::Seekable(source) = &mut self.source else {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "stream is not seekable",
            ));
        };
        let position = match from {
            SeekFrom::Start(n) => n as i128,
            SeekFrom::Current(n) => self.position as i128 + n as i128,
            SeekFrom::End(n) => self.limit as i128 + n as i128,
        };
        if position < 0 || position > self.limit as i128 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "seek outside bounded source",
            ));
        }
        self.position = source.seek(SeekFrom::Start(position as u64))?;
        Ok(self.position)
    }
}
