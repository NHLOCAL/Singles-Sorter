//! Frozen spaCy 3.7.5 lexical features. Python is an offline oracle, never runtime.
pub mod regex;
mod tables;
pub mod tokenizer;
pub fn in_ranges(c: char, r: &[(u32, u32)]) -> bool {
    let n = c as u32;
    r.binary_search_by(|&(a, b)| {
        if b < n {
            std::cmp::Ordering::Less
        } else if a > n {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    })
    .is_ok()
}
pub fn is_space(c: char) -> bool {
    in_ranges(c, tables::ISSPACE)
}
pub fn is_word(c: char) -> bool {
    c == '_' || in_ranges(c, tables::ISALNUM)
}
pub fn lower(s: &str) -> String {
    let cs: Vec<char> = s.chars().collect();
    let mut out = String::new();
    for (i, &c) in cs.iter().enumerate() {
        if c == 'Σ' {
            let before = cs[..i]
                .iter()
                .rev()
                .find(|&&x| !in_ranges(x, tables::IGNORABLE));
            let after = cs[i + 1..]
                .iter()
                .find(|&&x| !in_ranges(x, tables::IGNORABLE));
            out.push(
                if before.is_some_and(|&x| in_ranges(x, tables::CASED))
                    && !after.is_some_and(|&x| in_ranges(x, tables::CASED))
                {
                    'ς'
                } else {
                    'σ'
                },
            );
        } else if let Ok(j) = tables::LOWER.binary_search_by_key(&(c as u32), |&(cp, _)| cp) {
            out.push_str(tables::LOWER[j].1)
        } else {
            out.push(c)
        }
    }
    out
}
pub fn shape(s: &str) -> String {
    if s.chars().count() >= 100 {
        return "LONG".into();
    }
    let mut out = String::new();
    let mut last = None;
    let mut seq = 0;
    for c in s.chars() {
        let x = if in_ranges(c, tables::ISALPHA) {
            if in_ranges(c, tables::ISUPPER) {
                'X'
            } else {
                'x'
            }
        } else if in_ranges(c, tables::ISDIGIT) {
            'd'
        } else {
            c
        };
        if last == Some(x) {
            seq += 1
        } else {
            seq = 0;
            last = Some(x)
        }
        if seq < 4 {
            out.push(x)
        }
    }
    out
}
/// MurmurHash64A, UTF-8, seed 1, explicit little-endian loads.
pub fn murmur64(s: &str) -> u64 {
    const M: u64 = 0xc6a4a7935bd1e995;
    let bytes = s.as_bytes();
    let mut h = 1u64 ^ ((bytes.len() as u64).wrapping_mul(M));
    let (chunks, tail) = bytes.as_chunks::<8>();
    for b in chunks {
        let mut k = u64::from_le_bytes(*b);
        k = k.wrapping_mul(M);
        k ^= k >> 47;
        k = k.wrapping_mul(M);
        h ^= k;
        h = h.wrapping_mul(M)
    }
    for (i, &b) in tail.iter().enumerate() {
        h ^= (b as u64) << (8 * i)
    }
    if !tail.is_empty() {
        h = h.wrapping_mul(M)
    }
    h ^= h >> 47;
    h = h.wrapping_mul(M);
    h ^= h >> 47;
    h
}
pub fn string_id(s: &str) -> u64 {
    if s.is_empty() {
        0
    } else {
        tables::symbol(s).unwrap_or_else(|| murmur64(s))
    }
}
/// Exact specialized Thinc8.2.5 routine, despite upstream's x86 name.
pub fn thinc_hash(value: u64, seed: u32) -> [u32; 4] {
    fn mix(mut x: u64) -> u64 {
        x ^= x >> 33;
        x = x.wrapping_mul(0xff51afd7ed558ccd);
        x ^= x >> 33;
        x = x.wrapping_mul(0xc4ceb9fe1a85ec53);
        x ^= x >> 33;
        x
    }
    let mut h1 = value
        .wrapping_mul(0x87c37b91114253d5)
        .rotate_left(31)
        .wrapping_mul(0x4cf5ad432745937f)
        ^ (seed as u64)
        ^ 8;
    let mut h2 = (seed as u64) ^ 8;
    h1 = h1.wrapping_add(h2);
    h2 = h2.wrapping_add(h1);
    h1 = mix(h1);
    h2 = mix(h2);
    h1 = h1.wrapping_add(h2);
    h2 = h2.wrapping_add(h1);
    [h1 as u32, (h1 >> 32) as u32, h2 as u32, (h2 >> 32) as u32]
}
#[derive(Debug, PartialEq)]
pub struct FeatureRow {
    pub strings: [String; 4],
    pub ids: [u64; 4],
    pub hashes: [[u32; 4]; 4],
    pub buckets: [[usize; 4]; 4],
}
pub fn features(text: &str, norm_override: Option<&str>) -> FeatureRow {
    let cs: Vec<char> = text.chars().collect();
    let strings = [
        norm_override.map(str::to_owned).unwrap_or_else(|| {
            tables::norm(text)
                .map(str::to_owned)
                .unwrap_or_else(|| lower(text))
        }),
        cs.first().map(char::to_string).unwrap_or_default(),
        cs[cs.len().saturating_sub(3)..].iter().collect(),
        shape(text),
    ];
    let ids = std::array::from_fn(|i| string_id(&strings[i]));
    let hashes = std::array::from_fn(|i| thinc_hash(ids[i], 8 + i as u32));
    let buckets =
        std::array::from_fn(|i| hashes[i].map(|v| v as usize % if i == 0 { 2000 } else { 1000 }));
    FeatureRow {
        strings,
        ids,
        hashes,
        buckets,
    }
}
