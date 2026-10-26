//! Original-only reconstructed classifier. No caller fail-open policy lives here.
//! Every admitted input is analyzed completely; limits abort rather than truncate.
mod unicode_tables;
use sha2::{Digest, Sha256};
use std::{fmt, fs::OpenOptions, io::Read, path::Path};
use unicode_tables::*;

pub const WEIGHTS_BYTES: usize = 3_192_809;
pub const VOCAB_BYTES: usize = 1_268_235;
pub const WEIGHTS_SHA256: &str = "aa0da9a15f1ddbd62054c26fd4331b355b3e659c952ab3d71dfc8a138e561a33";
pub const VOCAB_SHA256: &str = "35b743c5bb3c532222939fa3a421ef25b94c832032926b6d50707902bcd666e3";
const N: usize = 79_816;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    Io(std::io::ErrorKind),
    Size,
    Checksum,
    Format,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClassifierError {
    Model(ModelError),
    Cancelled,
    ResourceLimit(&'static str),
    Allocation,
}
impl fmt::Display for ClassifierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ClassifierError {}
type Result<T> = std::result::Result<T, ClassifierError>;
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_input_bytes: usize,
    pub max_input_chars: usize,
    pub max_work: usize,
    pub max_allocation_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_input_bytes: 16_384,
            max_input_chars: 4_096,
            max_work: 4_000_000,
            max_allocation_bytes: 4_000_000,
        }
    }
}
fn checkpoint(cancel: &dyn Fn() -> bool) -> Result<()> {
    if cancel() {
        Err(ClassifierError::Cancelled)
    } else {
        Ok(())
    }
}
struct Budget<'a> {
    limits: Limits,
    work: usize,
    allocation: usize,
    cancel: &'a dyn Fn() -> bool,
}
impl<'a> Budget<'a> {
    fn new(limits: Limits, cancel: &'a dyn Fn() -> bool) -> Result<Self> {
        checkpoint(cancel)?;
        let hard = Limits::default();
        if limits.max_input_bytes > hard.max_input_bytes
            || limits.max_input_chars > hard.max_input_chars
            || limits.max_work > hard.max_work
            || limits.max_allocation_bytes > hard.max_allocation_bytes
        {
            return Err(ClassifierError::ResourceLimit("invalid limits"));
        }
        Ok(Self {
            limits,
            work: 0,
            allocation: 0,
            cancel,
        })
    }
    fn step(&mut self, n: usize) -> Result<()> {
        checkpoint(self.cancel)?;
        self.work = self
            .work
            .checked_add(n)
            .ok_or(ClassifierError::ResourceLimit("work"))?;
        if self.work > self.limits.max_work {
            return Err(ClassifierError::ResourceLimit("work"));
        }
        Ok(())
    }
    fn reserve<T>(&mut self, vec: &mut Vec<T>, n: usize) -> Result<()> {
        self.step(1)?;
        self.allocation = self
            .allocation
            .checked_add(
                n.checked_mul(std::mem::size_of::<T>())
                    .ok_or(ClassifierError::Allocation)?,
            )
            .ok_or(ClassifierError::Allocation)?;
        if self.allocation > self.limits.max_allocation_bytes {
            return Err(ClassifierError::ResourceLimit("allocation"));
        }
        vec.try_reserve_exact(n)
            .map_err(|_| ClassifierError::Allocation)
    }
    fn string(&mut self, size: usize) -> Result<String> {
        let mut bytes = Vec::<u8>::new();
        self.reserve(&mut bytes, size)?;
        Ok(String::from_utf8(bytes).expect("empty buffer"))
    }
}
fn lookup(cp: u32, r: &[u32], ranges: bool) -> Option<u32> {
    let (mut lo, mut hi) = (0, r.len() / 2);
    while lo < hi {
        let m = (lo + hi) / 2;
        if cp < r[2 * m] {
            hi = m;
        } else if cp > if ranges { r[2 * m + 1] } else { r[2 * m] } {
            lo = m + 1;
        } else {
            return Some(r[2 * m + 1]);
        }
    }
    None
}
fn in_ranges(cp: u32, r: &[u32]) -> bool {
    lookup(cp, r, true).is_some()
}
fn lower_inner(text: &str, b: &mut Budget<'_>) -> Result<String> {
    if text.len() > b.limits.max_input_bytes {
        return Err(ClassifierError::ResourceLimit("input bytes"));
    }
    let mut p = Vec::new();
    b.reserve(&mut p, text.len().min(b.limits.max_input_chars))?;
    for c in text.chars() {
        b.step(1)?;
        if p.len() == b.limits.max_input_chars {
            return Err(ClassifierError::ResourceLimit("input characters"));
        }
        p.push(c as u32);
    }
    let mut out = b.string(p.len() * 4)?;
    for (i, &cp) in p.iter().enumerate() {
        b.step(64)?;
        if cp == 0x130 {
            out.push_str("i\u{307}");
            continue;
        }
        let c = if cp == 0x3a3 {
            let mut before = None;
            let mut after = None;
            for &v in p[..i].iter().rev() {
                b.step(64)?;
                if !in_ranges(v, IGNORABLERANGES) {
                    before = Some(v);
                    break;
                }
            }
            for &v in &p[i + 1..] {
                b.step(64)?;
                if !in_ranges(v, IGNORABLERANGES) {
                    after = Some(v);
                    break;
                }
            }
            if before.is_some_and(|v| in_ranges(v, CASEDRANGES))
                && !after.is_some_and(|v| in_ranges(v, CASEDRANGES))
            {
                0x3c2
            } else {
                0x3c3
            }
        } else {
            lookup(cp, LOWERPAIRS, false).unwrap_or(cp)
        };
        out.push(char::from_u32(c).expect("frozen Unicode scalar"));
    }
    Ok(out)
}
pub fn lower(text: &str, limits: Limits, cancel: &dyn Fn() -> bool) -> Result<String> {
    let out = lower_inner(text, &mut Budget::new(limits, cancel)?)?;
    checkpoint(cancel)?;
    Ok(out)
}
fn analyze_inner(text: &str, b: &mut Budget<'_>) -> Result<Vec<String>> {
    let low = lower_inner(text, b)?;
    // A retained word has >=2 scalars; at most chars/2 words and < chars features.
    let mut words = Vec::new();
    b.reserve(&mut words, low.len().min(2 * b.limits.max_input_chars))?;
    let mut start = None;
    let mut count = 0;
    for (at, c) in low.char_indices().chain(std::iter::once((low.len(), '\0'))) {
        b.step(64)?;
        if in_ranges(c as u32, RANGES) {
            if start.is_none() {
                start = Some(at);
            }
            count += 1;
        } else {
            if let Some(s) = start.take()
                && count >= 2
            {
                let mut word = b.string(at - s)?;
                word.push_str(&low[s..at]);
                words.push(word);
            }
            count = 0;
        }
    }
    let n = words.len();
    for i in 0..n.saturating_sub(1) {
        b.step(words[i].len() + words[i + 1].len() + 1)?;
        let mut bigram = b.string(words[i].len() + words[i + 1].len() + 1)?;
        bigram.push_str(&words[i]);
        bigram.push(' ');
        bigram.push_str(&words[i + 1]);
        words.push(bigram);
    }
    Ok(words)
}
pub fn analyze(text: &str, limits: Limits, cancel: &dyn Fn() -> bool) -> Result<Vec<String>> {
    let out = analyze_inner(text, &mut Budget::new(limits, cancel)?)?;
    checkpoint(cancel)?;
    Ok(out)
}
#[derive(Debug)]
pub struct Prediction {
    pub class: usize,
    pub probabilities: Vec<f64>,
    pub scores: Vec<f64>,
    pub features: Vec<String>,
    pub sparse: Vec<(usize, f64)>,
    pub margin: f64,
}
/// numpy.argmax: first maximum wins, including class zero ties.
pub fn winner(scores: &[f64]) -> usize {
    let mut w = 0;
    for i in 1..scores.len() {
        if scores[i] > scores[w] {
            w = i;
        }
    }
    w
}
pub struct Model {
    vocab: Vec<String>,
    idf: Vec<f64>,
    coef: Vec<f64>,
    intercept: [f64; 4],
}
fn read_exact_asset(path: &Path, size: usize, cancel: &dyn Fn() -> bool) -> Result<Vec<u8>> {
    checkpoint(cancel)?;
    let meta =
        std::fs::metadata(path).map_err(|e| ClassifierError::Model(ModelError::Io(e.kind())))?;
    if !meta.is_file() || meta.len() != size as u64 {
        return Err(ClassifierError::Model(ModelError::Size));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    // Prevent a path-swap to FIFO between metadata and open from blocking this worker.
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    checkpoint(cancel)?;
    let mut file = options
        .open(path)
        .map_err(|e| ClassifierError::Model(ModelError::Io(e.kind())))?;
    let meta = file
        .metadata()
        .map_err(|e| ClassifierError::Model(ModelError::Io(e.kind())))?;
    if !meta.is_file() || meta.len() != size as u64 {
        return Err(ClassifierError::Model(ModelError::Size));
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size)
        .map_err(|_| ClassifierError::Allocation)?;
    bytes.resize(size, 0);
    for chunk in bytes.chunks_mut(4096) {
        checkpoint(cancel)?;
        file.read_exact(chunk)
            .map_err(|e| ClassifierError::Model(ModelError::Io(e.kind())))?;
    }
    checkpoint(cancel)?;
    let mut tail = [0];
    if file
        .read(&mut tail)
        .map_err(|e| ClassifierError::Model(ModelError::Io(e.kind())))?
        != 0
    {
        return Err(ClassifierError::Model(ModelError::Size));
    }
    checkpoint(cancel)?;
    Ok(bytes)
}
fn hash(bytes: &[u8], cancel: &dyn Fn() -> bool) -> Result<[u8; 32]> {
    let mut hash = Sha256::new();
    for chunk in bytes.chunks(4096) {
        checkpoint(cancel)?;
        hash.update(chunk);
    }
    Ok(hash.finalize().into())
}
fn asset_path(base: &Path, name: &str) -> Result<std::path::PathBuf> {
    if base.as_os_str().len() > 32_768 {
        return Err(ClassifierError::ResourceLimit("asset path"));
    }
    let mut path = std::ffi::OsString::new();
    path.try_reserve_exact(base.as_os_str().len() + name.len() + 4)
        .map_err(|_| ClassifierError::Allocation)?;
    path.push(base.as_os_str());
    let mut path = std::path::PathBuf::from(path);
    path.push(name);
    Ok(path)
}
impl Model {
    pub fn load_directory(path: &Path, cancel: &dyn Fn() -> bool) -> Result<Self> {
        let weights = read_exact_asset(
            &asset_path(path, "classifier-v14.weights.bin")?,
            WEIGHTS_BYTES,
            cancel,
        )?;
        let vocab = read_exact_asset(
            &asset_path(path, "classifier-v14.vocab.txt")?,
            VOCAB_BYTES,
            cancel,
        )?;
        Self::load(&weights, &vocab, cancel)
    }
    /// Hash and parse these exact borrowed bytes; never reopen a validated path.
    pub fn load(weights: &[u8], vocab: &[u8], cancel: &dyn Fn() -> bool) -> Result<Self> {
        checkpoint(cancel)?;
        if weights.len() != WEIGHTS_BYTES || vocab.len() != VOCAB_BYTES {
            return Err(ClassifierError::Model(ModelError::Size));
        }
        if hash(weights, cancel)?
            != [
                170, 13, 169, 161, 95, 29, 219, 214, 32, 84, 194, 111, 212, 51, 27, 53, 91, 62,
                101, 156, 149, 42, 179, 215, 29, 252, 138, 19, 142, 86, 26, 51,
            ]
            || hash(vocab, cancel)?
                != [
                    53, 183, 67, 197, 187, 60, 83, 34, 34, 147, 159, 163, 164, 33, 239, 37, 185,
                    76, 131, 32, 50, 146, 107, 109, 80, 112, 121, 2, 188, 214, 102, 227,
                ]
        {
            return Err(ClassifierError::Model(ModelError::Checksum));
        }
        // Both byte sequences are now pinned. Header size is 137 bytes; all scalar reads are checked.
        let mut at: usize = 0;
        let mut take = |n: usize| -> Result<&[u8]> {
            let end = at
                .checked_add(n)
                .ok_or(ClassifierError::Model(ModelError::Format))?;
            let out = weights
                .get(at..end)
                .ok_or(ClassifierError::Model(ModelError::Format))?;
            at = end;
            Ok(out)
        };
        if take(8)? != b"SC14F64\0" {
            return Err(ClassifierError::Model(ModelError::Format));
        }
        for expected in [1, N as u32, 4] {
            if u32::from_be_bytes(take(4)?.try_into().unwrap()) != expected {
                return Err(ClassifierError::Model(ModelError::Format));
            }
        }
        take(64)?;
        for (i, label) in ["ARTIST", "ALBUM", "SONG", "RANDOM"].iter().enumerate() {
            if u32::from_be_bytes(take(4)?.try_into().unwrap()) != i as u32 {
                return Err(ClassifierError::Model(ModelError::Format));
            }
            let n = u32::from_be_bytes(take(4)?.try_into().unwrap()) as usize;
            if take(n)? != label.as_bytes() {
                return Err(ClassifierError::Model(ModelError::Format));
            }
        }
        let mut intercept = [0.; 4];
        for s in &mut intercept {
            *s = f64::from_be_bytes(take(8)?.try_into().unwrap());
        }
        let mut idf = Vec::new();
        idf.try_reserve_exact(N)
            .map_err(|_| ClassifierError::Allocation)?;
        for _ in 0..N {
            checkpoint(cancel)?;
            idf.push(f64::from_be_bytes(take(8)?.try_into().unwrap()));
        }
        let mut coef = Vec::new();
        coef.try_reserve_exact(4 * N)
            .map_err(|_| ClassifierError::Allocation)?;
        for _ in 0..4 * N {
            checkpoint(cancel)?;
            coef.push(f64::from_be_bytes(take(8)?.try_into().unwrap()));
        }
        let text =
            std::str::from_utf8(vocab).map_err(|_| ClassifierError::Model(ModelError::Format))?;
        let mut names: Vec<String> = Vec::new();
        names
            .try_reserve_exact(N)
            .map_err(|_| ClassifierError::Allocation)?;
        for name in text.lines() {
            checkpoint(cancel)?;
            if names.len() >= N {
                return Err(ClassifierError::Model(ModelError::Format));
            }
            let mut s = String::new();
            s.try_reserve_exact(name.len())
                .map_err(|_| ClassifierError::Allocation)?;
            s.push_str(name);
            if names.last().is_some_and(|previous| previous >= &s) {
                return Err(ClassifierError::Model(ModelError::Format));
            }
            names.push(s);
        }
        if names.len() != N {
            return Err(ClassifierError::Model(ModelError::Format));
        }
        checkpoint(cancel)?;
        Ok(Self {
            vocab: names,
            idf,
            coef,
            intercept,
        })
    }
    pub fn predict(
        &self,
        text: &str,
        limits: Limits,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Prediction> {
        let mut b = Budget::new(limits, cancel)?;
        let features = analyze_inner(text, &mut b)?;
        let mut ids = Vec::new();
        b.reserve(&mut ids, features.len())?;
        for f in &features {
            b.step(20 * (f.len() + 1))?;
            if let Ok(i) = self.vocab.binary_search(f) {
                ids.push(i);
            }
        }
        // Fixed-size, in-place sort replaces BTreeMap without changing ascending feature arithmetic.
        b.step(ids.len() * 16)?;
        ids.sort_unstable();
        let mut sparse: Vec<(usize, f64)> = Vec::new();
        b.reserve(&mut sparse, ids.len())?;
        let mut pos = 0;
        while pos < ids.len() {
            b.step(1)?;
            let i = ids[pos];
            let mut end = pos + 1;
            while end < ids.len() && ids[end] == i {
                b.step(1)?;
                end += 1;
            }
            sparse.push((i, (end - pos) as f64 * self.idf[i]));
            pos = end;
        }
        let mut sq = 0.;
        for &(_, v) in &sparse {
            b.step(1)?;
            sq += v * v;
        }
        let norm: f64 = sq.sqrt();
        if norm != 0. {
            for (_, v) in &mut sparse {
                b.step(1)?;
                *v /= norm;
            }
        }
        let mut scores = Vec::new();
        b.reserve(&mut scores, 4)?;
        for c in 0..4 {
            let mut s = 0.;
            for &(i, v) in &sparse {
                b.step(1)?;
                s += v * self.coef[c * N + i];
            }
            scores.push(s + self.intercept[c]);
        }
        let class = winner(&scores);
        let mut probabilities = Vec::new();
        b.reserve(&mut probabilities, 4)?;
        for s in &scores {
            b.step(1)?;
            probabilities.push((s - scores[class]).exp());
        }
        let sum: f64 = probabilities.iter().sum();
        for p in &mut probabilities {
            *p /= sum;
        }
        let mut sorted = [
            probabilities[0],
            probabilities[1],
            probabilities[2],
            probabilities[3],
        ];
        sorted.sort_by(|a, b| b.total_cmp(a));
        let margin = sorted[0] - sorted[1];
        checkpoint(cancel)?;
        Ok(Prediction {
            class,
            probabilities,
            scores,
            features,
            sparse,
            margin,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    #[test]
    fn original_unicode_and_ties() {
        assert_eq!(
            lower("ΟΣ ΟΣΑ ΟΣ! ΣA İAB", Limits::default(), &|| false).unwrap(),
            "ος οσα ος! σa i\u{307}ab"
        );
        assert_eq!(
            analyze("א x 7 🎵", Limits::default(), &|| false).unwrap(),
            Vec::<String>::new()
        );
        assert_eq!(
            analyze("𐐀𐐀 ²²", Limits::default(), &|| false).unwrap(),
            vec!["𐐨𐐨", "²²", "𐐨𐐨 ²²"]
        );
        assert_eq!(winner(&[1., 1., 0., 0.]), 0);
        assert_eq!(winner(&[0., 1., 1., 0.]), 1);
    }
    #[test]
    fn cancellation_inside_analysis_and_token_reuse() {
        let calls = Cell::new(0);
        let cancelled = Cell::new(false);
        let cancel = || {
            calls.set(calls.get() + 1);
            if calls.get() == 120 {
                cancelled.set(true);
            }
            cancelled.get()
        };
        assert_eq!(
            analyze(&"ΟΣ ΑΣ ".repeat(100), Limits::default(), &cancel).unwrap_err(),
            ClassifierError::Cancelled
        );
        assert_eq!(
            analyze("abc", Limits::default(), &cancel).unwrap_err(),
            ClassifierError::Cancelled
        );
        assert!(analyze("abc", Limits::default(), &|| false).is_ok());
    }
    #[test]
    fn resource_limits_never_truncate() {
        for text in ["a".repeat(16385), "א".repeat(4097)] {
            assert!(matches!(
                analyze(&text, Limits::default(), &|| false),
                Err(ClassifierError::ResourceLimit(_))
            ));
        }
        assert!(matches!(
            analyze(
                "ab",
                Limits {
                    max_work: 1,
                    ..Limits::default()
                },
                &|| false
            ),
            Err(ClassifierError::ResourceLimit("work"))
        ));
        assert!(matches!(
            analyze(
                "ab",
                Limits {
                    max_allocation_bytes: 1,
                    ..Limits::default()
                },
                &|| false
            ),
            Err(ClassifierError::ResourceLimit("allocation"))
        ));
        assert!(matches!(
            analyze(
                "ab",
                Limits {
                    max_input_bytes: usize::MAX,
                    ..Limits::default()
                },
                &|| false
            ),
            Err(ClassifierError::ResourceLimit("invalid limits"))
        ));
        assert!(analyze(&"İ ".repeat(2048), Limits::default(), &|| false).is_ok());
    }
    #[test]
    fn typed_load_failures() {
        assert!(matches!(
            Model::load(&[], &[], &|| false),
            Err(ClassifierError::Model(ModelError::Size))
        ));
        assert!(matches!(
            Model::load(&[], &[], &|| true),
            Err(ClassifierError::Cancelled)
        ));
    }
    #[test]
    fn original_assets_when_explicitly_supplied() {
        let Some(dir) = std::env::var_os("ORIGINAL_CLASSIFIER_ASSETS") else {
            return;
        };
        let model = Model::load_directory(Path::new(&dir), &|| false).unwrap();
        assert!(
            model
                .predict("אברהם פריד", Limits::default(), &|| false)
                .is_ok()
        );
        let count = Cell::new(0);
        let cancel = || {
            count.set(count.get() + 1);
            count.get() > 600
        };
        assert!(matches!(
            model.predict(&"אברהם פריד ".repeat(100), Limits::default(), &cancel),
            Err(ClassifierError::Cancelled)
        ));
        let count = Cell::new(0);
        let cancel = || {
            count.set(count.get() + 1);
            count.get() > 30
        };
        assert!(matches!(
            Model::load_directory(Path::new(&dir), &cancel),
            Err(ClassifierError::Cancelled)
        ));
    }
}
