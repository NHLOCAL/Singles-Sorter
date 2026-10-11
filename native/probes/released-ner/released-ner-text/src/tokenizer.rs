//! Scoped port of pinned spaCy tokenizer.pyx, with synthetic differential gates.
use crate::{is_space, regex::Regex};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub text: String,
    pub offset: usize,
    pub whitespace: String,
    pub norm_override: Option<String>,
    pub sent_start: Option<bool>,
}
impl Token {
    fn plain(text: &str) -> Self {
        Self {
            text: text.into(),
            offset: 0,
            whitespace: String::new(),
            norm_override: None,
            sent_start: None,
        }
    }
}
pub struct Tokenizer {
    prefix: Regex,
    suffix: Regex,
    infix: Regex,
    url: Regex,
    exceptions: BTreeMap<String, Vec<Token>>,
    special_patterns: Vec<(String, Vec<String>)>,
}
fn sub(s: &str, a: usize, b: usize) -> String {
    s.chars().skip(a).take(b - a).collect()
}
impl Tokenizer {
    /// Load the exact original export from bounded, same-opened bytes. Parsing
    /// below remains the existing frozen tokenizer implementation.
    pub fn load(dir: &std::path::Path) -> Result<Self, released_ner_probe::LoadError> {
        use released_ner_probe::{LoadError, assets::{read_bounded, verify_hash, TOKENIZER_LIMIT, TOKENIZER_SHA256}};
        let bytes = read_bounded(&dir.join("tokenizer.json"), "tokenizer.json", TOKENIZER_LIMIT)?;
        verify_hash(&bytes, TOKENIZER_SHA256, "tokenizer.json")?;
        let value = serde_json::from_slice(&bytes)
            .map_err(|source| LoadError::Json { asset: "tokenizer.json", source })?;
        Self::from_json(&value).map_err(|reason| LoadError::Tokenizer { reason })
    }

    pub fn from_json(v: &Value) -> Result<Self, String> {
        let frozen: Value = serde_json::from_str(include_str!("../tokenizer.json"))
            .map_err(|e| format!("invalid embedded frozen tokenizer: {e}"))?;
        if v != &frozen {
            return Err("tokenizer differs from frozen released export".into());
        }
        if !v["token_match"].is_null() {
            return Err("unsupported token_match".into());
        }
        if v["unicode"] != "15.0.0" {
            return Err("unsupported Unicode version".into());
        }
        let re = |name: &str| Regex::new(v[name].as_str().ok_or(format!("missing regex {name}"))?);
        let mut exceptions = BTreeMap::new();
        for (k, vals) in v["exceptions"].as_object().ok_or("missing exceptions")? {
            let mut ts = vec![];
            for val in vals.as_array().ok_or("invalid exception")? {
                let mut t = Token::plain(val["ORTH"].as_str().ok_or("missing ORTH")?);
                t.norm_override = val["NORM"].as_str().map(str::to_owned);
                ts.push(t)
            }
            if ts.iter().map(|t| t.text.as_str()).collect::<String>() != *k {
                return Err("exception ORTH concatenation".into());
            }
            exceptions.insert(k.clone(), ts);
        }
        let mut this = Self {
            prefix: re("prefix_search")?,
            suffix: re("suffix_search")?,
            infix: re("infix_finditer")?,
            url: re("url_match")?,
            exceptions,
            special_patterns: vec![],
        };
        for k in this.exceptions.keys() {
            if v["faster_heuristics"] == false
                || this.prefix.search(k)?.is_some()
                || !this.infix.finditer(k)?.is_empty()
                || this.suffix.search(k)?.is_some()
                || k.contains(' ')
            {
                this.special_patterns.push((
                    k.clone(),
                    this.affixes(k, false)?
                        .into_iter()
                        .map(|t| t.text)
                        .collect(),
                ));
            }
        }
        Ok(this)
    }
    fn has_special(&self, s: &str, enabled: bool) -> bool {
        enabled && self.exceptions.contains_key(s)
    }
    fn chunk(&self, chunk: &str, special: bool) -> Result<Vec<Token>, String> {
        let mut s = chunk.to_owned();
        let mut prefixes = vec![];
        let mut suffixes = vec![];
        let mut last = 0;
        while !s.is_empty() && s.chars().count() != last {
            if self.has_special(&s, special) {
                break;
            }
            last = s.chars().count();
            let pre = self.prefix.search(&s)?.map(|(a, b)| b - a).unwrap_or(0);
            let minus_pre = sub(&s, pre, last);
            if pre > 0 && !minus_pre.is_empty() && self.has_special(&minus_pre, special) {
                prefixes.push(Token::plain(&sub(&s, 0, pre)));
                s = minus_pre;
                break;
            }
            let suf = self
                .suffix
                .search(&minus_pre)?
                .map(|(a, b)| b - a)
                .unwrap_or(0);
            let minus_suf = sub(&s, 0, last - suf);
            if suf > 0 && !minus_suf.is_empty() && self.has_special(&minus_suf, special) {
                suffixes.push(Token::plain(&sub(&s, last - suf, last)));
                s = minus_suf;
                break;
            }
            if pre > 0 && suf > 0 && pre + suf <= last {
                prefixes.push(Token::plain(&sub(&s, 0, pre)));
                suffixes.push(Token::plain(&sub(&s, last - suf, last)));
                s = sub(&s, pre, last - suf)
            } else if pre > 0 {
                prefixes.push(Token::plain(&sub(&s, 0, pre)));
                s = minus_pre
            } else if suf > 0 {
                suffixes.push(Token::plain(&sub(&s, last - suf, last)));
                s = minus_suf
            }
        }
        let mut result = prefixes;
        if !s.is_empty() {
            if self.has_special(&s, special) {
                result.extend(self.exceptions[&s].clone())
            } else if self.url.search(&s)?.is_some_and(|(a, _)| a == 0) {
                result.push(Token::plain(&s))
            } else {
                let matches = self.infix.finditer(&s)?;
                let mut start = 0;
                for (a, b) in matches {
                    if a == 0 {
                        continue;
                    }
                    if a != start {
                        result.push(Token::plain(&sub(&s, start, a)))
                    }
                    if a != b {
                        result.push(Token::plain(&sub(&s, a, b)))
                    }
                    start = b
                }
                let tail = sub(&s, start, s.chars().count());
                if !tail.is_empty() {
                    result.push(Token::plain(&tail))
                }
            }
        }
        result.extend(suffixes.into_iter().rev());
        Ok(result)
    }
    fn affixes(&self, s: &str, special: bool) -> Result<Vec<Token>, String> {
        let cs: Vec<char> = s.chars().collect();
        let mut result: Vec<Token> = vec![];
        if cs.is_empty() {
            return Ok(result);
        }
        if cs.len() >= (1 << 30) {
            return Err("input exceeds spaCy maximum length".into());
        }
        let mut start = 0;
        let mut ws = is_space(cs[0]);
        for (i, &c) in cs.iter().enumerate() {
            if is_space(c) != ws {
                if start < i {
                    result.extend(self.chunk(&sub(s, start, i), special)?)
                }
                if c == ' ' {
                    result
                        .last_mut()
                        .ok_or("invalid whitespace transition")?
                        .whitespace = " ".into();
                    start = i + 1
                } else {
                    start = i
                }
                ws = !ws
            }
        }
        if start < cs.len() {
            result.extend(self.chunk(&sub(s, start, cs.len()), special)?);
            result.last_mut().ok_or("empty final chunk")?.whitespace =
                if cs[cs.len() - 1] == ' ' && !ws {
                    " ".into()
                } else {
                    String::new()
                }
        }
        Ok(result)
    }
    pub fn tokenize(&self, s: &str) -> Result<Vec<Token>, String> {
        let mut ts = self.affixes(s, true)?;
        // Doc.push_back initializes the first token as a sentence start.
        // A second-pass exception copies the cached TokenC wholesale, replacing
        // this with its default None. Preserve that behavior, including spaces.
        if let Some(first) = ts.first_mut() {
            first.sent_start = Some(true);
        }
        // PhraseMatcher ORTH patterns; longest first, then earliest start.
        let mut matches = vec![];
        for (_, pat) in &self.special_patterns {
            if pat.is_empty() {
                continue;
            }
            for start in 0..ts.len() {
                if start + pat.len() <= ts.len()
                    && ts[start..start + pat.len()]
                        .iter()
                        .zip(pat)
                        .all(|(t, p)| t.text == *p)
                {
                    matches.push((start, start + pat.len()))
                }
            }
        }
        matches.sort_by_key(|&(a, b)| (std::cmp::Reverse(b - a), a));
        let mut seen = BTreeSet::new();
        let mut chosen = BTreeMap::new();
        for (a, b) in matches {
            if !seen.contains(&a) && !seen.contains(&(b - 1)) {
                chosen.insert(a, b);
            }
            for i in a..b {
                seen.insert(i);
            }
        }
        let mut result = vec![];
        let mut i = 0;
        while i < ts.len() {
            if let Some(&end) = chosen.get(&i) {
                let mut text = String::new();
                for (j, t) in ts[i..end].iter().enumerate() {
                    text.push_str(&t.text);
                    if j + 1 < end - i {
                        text.push_str(&t.whitespace)
                    }
                }
                if let Some(parts) = self.exceptions.get(&text) {
                    let mut parts = parts.clone();
                    parts.last_mut().unwrap().whitespace = ts[end - 1].whitespace.clone();
                    result.extend(parts)
                } else {
                    result.extend_from_slice(&ts[i..end])
                }
                i = end
            } else {
                result.push(ts[i].clone());
                i += 1
            }
        }
        let mut offset = 0;
        let mut reconstructed = String::new();
        for t in &mut result {
            t.offset = offset;
            offset += t.text.chars().count() + t.whitespace.chars().count();
            reconstructed.push_str(&t.text);
            reconstructed.push_str(&t.whitespace)
        }
        if reconstructed != s {
            return Err("token reconstruction invariant failed".into());
        }
        ts.clear();
        Ok(result)
    }
}
