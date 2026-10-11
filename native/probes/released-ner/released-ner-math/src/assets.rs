//! Bounded asset acquisition. Validation always uses the very bytes read here,
//! never a hash-by-path followed by a second open.
use sha2::{Digest, Sha256};
use std::{
    fmt,
    fs::OpenOptions,
    io::{self, Read},
    path::Path,
};

pub const MANIFEST_LIMIT: usize = 64 * 1024;
pub const PARAMETER_BYTES: usize = 3_829_976;
pub const TOKENIZER_LIMIT: usize = 128 * 1024;
pub const TENSOR_SHA256: &str = "84b8c643b32d734c626e90890c389807000741a2863930585e931467f7e00745";
pub const TOKENIZER_SHA256: &str =
    "33841cad4949620c48fbb0570865a31a19cfbb463088b1fec3d9fb7e1a8c8574";

#[derive(Debug)]
pub enum LoadError {
    Io {
        asset: &'static str,
        source: io::Error,
    },
    NotRegular {
        asset: &'static str,
    },
    TooLarge {
        asset: &'static str,
        limit: usize,
    },
    Allocation {
        asset: &'static str,
    },
    Json {
        asset: &'static str,
        source: serde_json::Error,
    },
    Unsupported {
        field: String,
    },
    Length {
        asset: &'static str,
        expected: usize,
        actual: usize,
    },
    Hash {
        asset: &'static str,
    },
    InvalidTensor {
        reason: &'static str,
    },
    Tokenizer {
        reason: String,
    },
}
impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { asset, source } => write!(f, "cannot read {asset}: {source}"),
            Self::NotRegular { asset } => write!(f, "{asset} is not a regular file"),
            Self::TooLarge { asset, limit } => write!(f, "{asset} exceeds {limit} bytes"),
            Self::Allocation { asset } => write!(f, "cannot allocate bounded {asset} buffer"),
            Self::Json { asset, source } => write!(f, "invalid {asset}: {source}"),
            Self::Unsupported { field } => write!(f, "unsupported model contract field {field}"),
            Self::Length {
                asset,
                expected,
                actual,
            } => write!(f, "{asset} has {actual} bytes, expected {expected}"),
            Self::Hash { asset } => write!(f, "{asset} differs from frozen released bytes"),
            Self::InvalidTensor { reason } => write!(f, "invalid tensor: {reason}"),
            Self::Tokenizer { reason } => write!(f, "invalid tokenizer: {reason}"),
        }
    }
}
impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Json { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Retain at most `limit` bytes (read one extra byte to detect growth).
/// Metadata rejects oversized files before allocation;
/// the streaming bound also protects against growth after that check.
pub fn read_bounded(path: &Path, asset: &'static str, limit: usize) -> Result<Vec<u8>, LoadError> {
    if !cfg!(all(
        target_arch = "x86_64",
        any(target_os = "linux", target_os = "windows")
    )) {
        return Err(LoadError::Unsupported {
            field: "asset-loading platform".into(),
        });
    }
    // Reject obvious special files before opening. The descriptor check below
    // remains authoritative in case the directory entry changes concurrently.
    let before = std::fs::metadata(path).map_err(|source| LoadError::Io { asset, source })?;
    if !before.is_file() {
        return Err(LoadError::NotRegular { asset });
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // Linux O_NONBLOCK (asm-generic/fcntl.h): a concurrent replacement by a
        // FIFO must not block open before we can reject its descriptor type.
        options.custom_flags(0o4000);
    }
    let mut file = options
        .open(path)
        .map_err(|source| LoadError::Io { asset, source })?;
    let metadata = file
        .metadata()
        .map_err(|source| LoadError::Io { asset, source })?;
    if !metadata.is_file() {
        return Err(LoadError::NotRegular { asset });
    }
    if metadata.len() > limit as u64 {
        return Err(LoadError::TooLarge { asset, limit });
    }
    read_stream(&mut file, asset, limit)
}
fn read_stream(
    reader: &mut impl Read,
    asset: &'static str,
    limit: usize,
) -> Result<Vec<u8>, LoadError> {
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let request = limit
            .saturating_sub(bytes.len())
            .saturating_add(1)
            .min(chunk.len());
        let count = match reader.read(&mut chunk[..request]) {
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            result => result.map_err(|source| LoadError::Io { asset, source })?,
        };
        if count == 0 {
            return Ok(bytes);
        }
        let size = bytes
            .len()
            .checked_add(count)
            .ok_or(LoadError::TooLarge { asset, limit })?;
        if size > limit {
            return Err(LoadError::TooLarge { asset, limit });
        }
        bytes
            .try_reserve_exact(count)
            .map_err(|_| LoadError::Allocation { asset })?;
        bytes.extend_from_slice(&chunk[..count]);
    }
}
pub fn verify_hash(bytes: &[u8], expected: &str, asset: &'static str) -> Result<(), LoadError> {
    if format!("{:x}", Sha256::digest(bytes)) != expected {
        return Err(LoadError::Hash { asset });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn streaming_growth_cannot_exceed_bound() {
        let mut input = io::repeat(0);
        assert!(matches!(
            read_stream(&mut input, "test", 3),
            Err(LoadError::TooLarge { .. })
        ));
        let mut exact = io::Cursor::new(b"abc");
        assert_eq!(read_stream(&mut exact, "test", 3).unwrap(), b"abc");
    }
}
