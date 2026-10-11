//! Ordered resource decoding; malformed widths are a lazy lookup boundary.
use sha2::{Digest, Sha256};
use sorter_core::{Alias, exact_match};
use std::fmt;

/// CPython 3.12 csv.reader default, measured in Unicode code points.
pub const PYTHON_CSV_FIELD_LIMIT: usize = 131_072;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AliasSource {
    Bundled,
    Personal,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawAliasRow {
    pub record: u64,
    pub fields: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AliasSourceHash {
    pub source: AliasSource,
    pub sha256: String,
    pub bytes: usize,
    pub records: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceDiagnostic {
    pub code: &'static str,
    pub source: AliasSource,
    pub index: usize,
    pub record: u64,
    pub width: usize,
    pub blocks_apply: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceError {
    InvalidUtf8 {
        source: AliasSource,
        valid_up_to: usize,
        error_len: Option<usize>,
    },
    MalformedRow {
        source: AliasSource,
        index: usize,
        record: u64,
        width: usize,
    },
    FieldTooLarge {
        source: AliasSource,
        record: u64,
        limit: usize,
    },
    IndexOutOfBounds {
        index: usize,
        rows: usize,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AliasMatch {
    pub index: usize,
    pub alias: Alias,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AliasBundle {
    pub ordered_rows: Vec<RawAliasRow>,
    pub source_hashes: Vec<AliasSourceHash>,
    pub diagnostics: Vec<ResourceDiagnostic>,
}
impl AliasBundle {
    /// Unpack precisely the reached row, as the released `for source, target` does.
    /// Diagnostics never cause an earlier successful decision to disappear.
    pub fn alias_at(&self, index: usize) -> Result<Alias, ResourceError> {
        let row = self
            .ordered_rows
            .get(index)
            .ok_or(ResourceError::IndexOutOfBounds {
                index,
                rows: self.ordered_rows.len(),
            })?;
        if row.fields.len() != 2 {
            let bundled_count = self.source_hashes[0].records as usize;
            let source = if index < bundled_count {
                AliasSource::Bundled
            } else {
                AliasSource::Personal
            };
            return Err(ResourceError::MalformedRow {
                source,
                index,
                record: row.record,
                width: row.fields.len(),
            });
        }
        Ok(Alias {
            source: row.fields[0].clone(),
            target: row.fields[1].clone(),
        })
    }
    /// Decision-only alias lookup, retaining the release's first-row precedence.
    /// This does not claim to perform the later metadata/title steps of a song job.
    pub fn find_alias(&self, text: &str) -> Result<Option<AliasMatch>, ResourceError> {
        for index in 0..self.ordered_rows.len() {
            let alias = self.alias_at(index)?;
            if text.contains(&alias.source) && exact_match(text, &alias.source) {
                return Ok(Some(AliasMatch { index, alias }));
            }
        }
        Ok(None)
    }
    pub fn has_apply_blockers(&self) -> bool {
        !self.diagnostics.is_empty()
    }
}
/// Decode the released UTF-8/default-excel dialect after Python text-mode universal
/// newline translation. No header, BOM, whitespace, width or order normalization.
/// Codec/parser failures abort the resource; width failures are retained lazily.
pub fn load_aliases(bundled: &[u8], personal: Option<&[u8]>) -> Result<AliasBundle, ResourceError> {
    let mut bundle = AliasBundle {
        ordered_rows: Vec::new(),
        source_hashes: Vec::new(),
        diagnostics: Vec::new(),
    };
    for (source, bytes) in [
        (AliasSource::Bundled, Some(bundled)),
        (AliasSource::Personal, personal),
    ] {
        let Some(bytes) = bytes else { continue };
        let text = std::str::from_utf8(bytes).map_err(|error| ResourceError::InvalidUtf8 {
            source,
            valid_up_to: error.valid_up_to(),
            error_len: error.error_len(),
        })?;
        let rows = parse_python_csv(text, source)?;
        bundle.source_hashes.push(AliasSourceHash {
            source,
            sha256: format!("{:x}", Sha256::digest(bytes)),
            bytes: bytes.len(),
            records: rows.len() as u64,
        });
        for row in rows {
            let index = bundle.ordered_rows.len();
            if row.fields.len() != 2 {
                bundle.diagnostics.push(ResourceDiagnostic {
                    code: "ALIAS_ROW_WIDTH",
                    source,
                    index,
                    record: row.record,
                    width: row.fields.len(),
                    blocks_apply: true,
                });
            }
            bundle.ordered_rows.push(row);
        }
    }
    Ok(bundle)
}

#[derive(Clone, Copy)]
enum State {
    StartRecord,
    StartField,
    Unquoted,
    Quoted,
    AfterQuote,
}

fn parse_python_csv(text: &str, source: AliasSource) -> Result<Vec<RawAliasRow>, ResourceError> {
    let mut rows = Vec::new();
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut field_length = 0;
    let mut state = State::StartRecord;
    let mut chars = text.chars().peekable();
    while let Some(mut ch) = chars.next() {
        // Path.open('r', encoding='utf-8') uses newline=None in the oracle.
        if ch == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            ch = '\n';
        }
        match state {
            State::StartRecord if ch == '\n' => {
                rows.push(RawAliasRow {
                    record: rows.len() as u64 + 1,
                    fields: Vec::new(),
                });
                continue;
            }
            State::StartRecord | State::StartField => match ch {
                '"' => state = State::Quoted,
                ',' => {
                    fields.push(std::mem::take(&mut field));
                    field_length = 0;
                    state = State::StartField;
                }
                '\n' => {
                    fields.push(std::mem::take(&mut field));
                    field_length = 0;
                    rows.push(RawAliasRow {
                        record: rows.len() as u64 + 1,
                        fields: std::mem::take(&mut fields),
                    });
                    state = State::StartRecord;
                }
                _ => {
                    append(
                        &mut field,
                        &mut field_length,
                        ch,
                        source,
                        rows.len() as u64 + 1,
                    )?;
                    state = State::Unquoted;
                }
            },
            State::Quoted => {
                if ch == '"' {
                    state = State::AfterQuote;
                } else {
                    append(
                        &mut field,
                        &mut field_length,
                        ch,
                        source,
                        rows.len() as u64 + 1,
                    )?;
                }
            }
            State::AfterQuote if ch == '"' => {
                append(
                    &mut field,
                    &mut field_length,
                    ch,
                    source,
                    rows.len() as u64 + 1,
                )?;
                state = State::Quoted;
            }
            State::AfterQuote | State::Unquoted => match ch {
                ',' => {
                    fields.push(std::mem::take(&mut field));
                    field_length = 0;
                    state = State::StartField;
                }
                '\n' => {
                    fields.push(std::mem::take(&mut field));
                    field_length = 0;
                    rows.push(RawAliasRow {
                        record: rows.len() as u64 + 1,
                        fields: std::mem::take(&mut fields),
                    });
                    state = State::StartRecord;
                }
                _ => {
                    append(
                        &mut field,
                        &mut field_length,
                        ch,
                        source,
                        rows.len() as u64 + 1,
                    )?;
                    state = State::Unquoted;
                }
            },
        }
    }
    // strict=False accepts a quoted field ending at EOF, even without a close quote.
    if !matches!(state, State::StartRecord) {
        fields.push(field);
        rows.push(RawAliasRow {
            record: rows.len() as u64 + 1,
            fields,
        });
    }
    Ok(rows)
}

fn append(
    field: &mut String,
    length: &mut usize,
    ch: char,
    source: AliasSource,
    record: u64,
) -> Result<(), ResourceError> {
    if *length == PYTHON_CSV_FIELD_LIMIT {
        return Err(ResourceError::FieldTooLarge {
            source,
            record,
            limit: PYTHON_CSV_FIELD_LIMIT,
        });
    }
    field.push(ch);
    *length += 1;
    Ok(())
}

impl fmt::Display for ResourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedRow { width, .. } if *width < 2 => write!(
                formatter,
                "not enough values to unpack (expected 2, got {width})"
            ),
            Self::MalformedRow { .. } => {
                formatter.write_str("too many values to unpack (expected 2)")
            }
            Self::InvalidUtf8 {
                source,
                valid_up_to,
                ..
            } => write!(
                formatter,
                "invalid UTF-8 in {source:?} at byte {valid_up_to}"
            ),
            Self::FieldTooLarge { limit, .. } => {
                write!(formatter, "field larger than field limit ({limit})")
            }
            Self::IndexOutOfBounds { index, rows } => {
                write!(formatter, "alias index {index} outside {rows} rows")
            }
        }
    }
}
impl std::error::Error for ResourceError {}
