//! Scoped port of pinned spaCy tokenizer.pyx, with synthetic differential gates.
use crate::{is_space, regex::Regex};
use released_ner_probe::control::{InferenceError, RequestControl};
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
        use released_ner_probe::{
            assets::{read_bounded, verify_hash, TOKENIZER_LIMIT, TOKENIZER_SHA256},
            LoadError,
        };
        let bytes = read_bounded(
            &dir.join("tokenizer.json"),
            "tokenizer.json",
            TOKENIZER_LIMIT,
        )?;
        verify_hash(&bytes, TOKENIZER_SHA256, "tokenizer.json")?;
        let value = serde_json::from_slice(&bytes).map_err(|source| LoadError::Json {
            asset: "tokenizer.json",
            source,
        })?;
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
                    this.affixes(k, false, None)
                        .map_err(|e| e.to_string())?
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
    fn chunk(
        &self,
        chunk: &str,
        special: bool,
        control: Option<&RequestControl<'_>>,
    ) -> Result<Vec<Token>, InferenceError> {
        check_text(chunk, control)?;
        let mut s = chunk.to_owned();
        let mut prefixes = vec![];
        let mut suffixes = vec![];
        let mut last = 0;
        while !s.is_empty() && s.chars().count() != last {
            charge(control, s.len())?;
            if self.has_special(&s, special) {
                break;
            }
            last = s.chars().count();
            let pre = self
                .prefix
                .search_controlled(&s, control)?
                .map(|(a, b)| b - a)
                .unwrap_or(0);
            let minus_pre = sub(&s, pre, last);
            if pre > 0 && !minus_pre.is_empty() && self.has_special(&minus_pre, special) {
                push_plain(&mut prefixes, &sub(&s, 0, pre), control)?;
                s = minus_pre;
                break;
            }
            let suf = self
                .suffix
                .search_controlled(&minus_pre, control)?
                .map(|(a, b)| b - a)
                .unwrap_or(0);
            let minus_suf = sub(&s, 0, last - suf);
            if suf > 0 && !minus_suf.is_empty() && self.has_special(&minus_suf, special) {
                push_plain(&mut suffixes, &sub(&s, last - suf, last), control)?;
                s = minus_suf;
                break;
            }
            if pre > 0 && suf > 0 && pre + suf <= last {
                push_plain(&mut prefixes, &sub(&s, 0, pre), control)?;
                push_plain(&mut suffixes, &sub(&s, last - suf, last), control)?;
                s = sub(&s, pre, last - suf)
            } else if pre > 0 {
                push_plain(&mut prefixes, &sub(&s, 0, pre), control)?;
                s = minus_pre
            } else if suf > 0 {
                push_plain(&mut suffixes, &sub(&s, last - suf, last), control)?;
                s = minus_suf
            }
        }
        let mut result = prefixes;
        if !s.is_empty() {
            if self.has_special(&s, special) {
                extend_tokens(&mut result, &self.exceptions[&s], control)?
            } else if self
                .url
                .search_controlled(&s, control)?
                .is_some_and(|(a, _)| a == 0)
            {
                push_plain(&mut result, &s, control)?
            } else {
                let matches = self.infix.finditer_controlled(&s, control)?;
                let mut start = 0;
                for (a, b) in matches {
                    charge(control, 1)?;
                    if a == 0 {
                        continue;
                    }
                    if a != start {
                        push_plain(&mut result, &sub(&s, start, a), control)?
                    }
                    if a != b {
                        push_plain(&mut result, &sub(&s, a, b), control)?
                    }
                    start = b
                }
                let tail = sub(&s, start, s.chars().count());
                if !tail.is_empty() {
                    push_plain(&mut result, &tail, control)?
                }
            }
        }
        reserve_tokens(&mut result, suffixes.len(), control)?;
        result.extend(suffixes.into_iter().rev());
        if let Some(c) = control {
            c.checkpoint()?;
        }
        Ok(result)
    }
    fn affixes(
        &self,
        s: &str,
        special: bool,
        control: Option<&RequestControl<'_>>,
    ) -> Result<Vec<Token>, InferenceError> {
        check_text(s, control)?;
        let cs: Vec<char> = s.chars().collect();
        let mut result: Vec<Token> = vec![];
        if cs.is_empty() {
            return Ok(result);
        }
        if cs.len() >= (1 << 30) {
            return Err(InferenceError::Model(
                "input exceeds spaCy maximum length".into(),
            ));
        }
        let mut start = 0;
        let mut ws = is_space(cs[0]);
        for (i, &c) in cs.iter().enumerate() {
            charge(control, 1)?;
            if is_space(c) != ws {
                if start < i {
                    {
                        let chunk = self.chunk(&sub(s, start, i), special, control)?;
                        reserve_tokens(&mut result, chunk.len(), control)?;
                        result.extend(chunk);
                    }
                }
                if c == ' ' {
                    result
                        .last_mut()
                        .ok_or_else(|| {
                            InferenceError::Model("invalid whitespace transition".into())
                        })?
                        .whitespace = " ".into();
                    start = i + 1
                } else {
                    start = i
                }
                ws = !ws
            }
        }
        if start < cs.len() {
            let chunk = self.chunk(&sub(s, start, cs.len()), special, control)?;
            reserve_tokens(&mut result, chunk.len(), control)?;
            result.extend(chunk);
            result
                .last_mut()
                .ok_or_else(|| InferenceError::Model("empty final chunk".into()))?
                .whitespace = if cs[cs.len() - 1] == ' ' && !ws {
                " ".into()
            } else {
                String::new()
            }
        }
        if let Some(c) = control {
            c.checkpoint()?;
        }
        Ok(result)
    }
    /// Legacy unbounded proof entry. Never use as a production inference boundary.
    pub fn tokenize(&self, s: &str) -> Result<Vec<Token>, String> {
        self.tokenize_controlled(s, None).map_err(|e| e.to_string())
    }
    pub fn tokenize_bounded(
        &self,
        s: &str,
        control: &RequestControl<'_>,
    ) -> Result<Vec<Token>, InferenceError> {
        self.tokenize_controlled(s, Some(control))
    }
    fn tokenize_controlled(
        &self,
        s: &str,
        control: Option<&RequestControl<'_>>,
    ) -> Result<Vec<Token>, InferenceError> {
        check_text(s, control)?;
        let mut ts = self.affixes(s, true, control)?;
        // Doc.push_back initializes the first token as a sentence start.
        // A second-pass exception copies the cached TokenC wholesale, replacing
        // this with its default None. Preserve that behavior, including spaces.
        if let Some(first) = ts.first_mut() {
            first.sent_start = Some(true);
        }
        // PhraseMatcher ORTH patterns; longest first, then earliest start.
        let mut matches = vec![];
        for (_, pat) in &self.special_patterns {
            charge(control, 1)?;
            if pat.is_empty() {
                continue;
            }
            for start in 0..ts.len() {
                charge(control, pat.len())?;
                if start + pat.len() <= ts.len()
                    && ts[start..start + pat.len()]
                        .iter()
                        .zip(pat)
                        .all(|(t, p)| t.text == *p)
                {
                    if let Some(c) = control {
                        c.resource("special matches", matches.len() + 1, c.limits().regex_calls)?;
                        matches
                            .try_reserve(1)
                            .map_err(|_| InferenceError::Allocation {
                                resource: "special matches",
                            })?;
                    }
                    matches.push((start, start + pat.len()))
                }
            }
        }
        matches.sort_by_key(|&(a, b)| (std::cmp::Reverse(b - a), a));
        let mut seen = BTreeSet::new();
        let mut chosen = BTreeMap::new();
        for (a, b) in matches {
            charge(control, 1)?;
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
            charge(control, 1)?;
            if let Some(&end) = chosen.get(&i) {
                let mut text = String::new();
                for (j, t) in ts[i..end].iter().enumerate() {
                    text.push_str(&t.text);
                    if j + 1 < end - i {
                        text.push_str(&t.whitespace)
                    }
                }
                if let Some(parts) = self.exceptions.get(&text) {
                    reserve_tokens(&mut result, parts.len(), control)?;
                    let mut parts = parts.clone();
                    parts.last_mut().unwrap().whitespace = ts[end - 1].whitespace.clone();
                    result.extend(parts)
                } else {
                    extend_tokens(&mut result, &ts[i..end], control)?
                }
                i = end
            } else {
                extend_tokens(&mut result, &ts[i..i + 1], control)?;
                i += 1
            }
        }
        let mut offset = 0;
        let mut reconstructed = String::new();
        for t in &mut result {
            charge(control, t.text.len() + t.whitespace.len())?;
            t.offset = offset;
            offset += t.text.chars().count() + t.whitespace.chars().count();
            reconstructed.push_str(&t.text);
            reconstructed.push_str(&t.whitespace)
        }
        if reconstructed != s {
            return Err(InferenceError::Model(
                "token reconstruction invariant failed".into(),
            ));
        }
        ts.clear();
        if let Some(c) = control {
            c.checkpoint()?;
        }
        Ok(result)
    }
}

fn charge(control: Option<&RequestControl<'_>>, units: usize) -> Result<(), InferenceError> {
    if let Some(c) = control {
        c.charge(units)?;
    }
    Ok(())
}
fn check_text(s: &str, control: Option<&RequestControl<'_>>) -> Result<(), InferenceError> {
    if let Some(c) = control {
        c.resource("input bytes", s.len(), c.limits().input_bytes)?;
        c.resource(
            "input characters",
            s.chars().count(),
            c.limits().input_chars,
        )?;
        c.charge(s.len())?;
    }
    Ok(())
}
fn reserve_tokens(
    out: &mut Vec<Token>,
    additional: usize,
    control: Option<&RequestControl<'_>>,
) -> Result<(), InferenceError> {
    if let Some(c) = control {
        c.resource(
            "tokens",
            out.len().checked_add(additional).unwrap_or(usize::MAX),
            c.limits().tokens,
        )?;
        out.try_reserve_exact(additional)
            .map_err(|_| InferenceError::Allocation { resource: "tokens" })?;
    }
    Ok(())
}
fn push_plain(
    out: &mut Vec<Token>,
    text: &str,
    control: Option<&RequestControl<'_>>,
) -> Result<(), InferenceError> {
    reserve_tokens(out, 1, control)?;
    out.push(Token::plain(text));
    Ok(())
}
fn extend_tokens(
    out: &mut Vec<Token>,
    tokens: &[Token],
    control: Option<&RequestControl<'_>>,
) -> Result<(), InferenceError> {
    reserve_tokens(out, tokens.len(), control)?;
    out.extend_from_slice(tokens);
    Ok(())
}
