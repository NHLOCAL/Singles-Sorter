//! Narrow PCRE2-32 bridge. Frozen regex syntax translated explicitly; no UCP tables.
use released_ner_probe::control::{InferenceError, RequestControl};
use std::{cell::RefCell, ffi::c_void};

extern "C" {
    fn pcre2_compile_32(
        p: *const u32,
        n: usize,
        options: u32,
        error: *mut i32,
        offset: *mut usize,
        context: *mut c_void,
    ) -> *mut c_void;
    fn pcre2_code_free_32(code: *mut c_void);
    fn pcre2_match_data_create_from_pattern_32(
        code: *const c_void,
        context: *mut c_void,
    ) -> *mut c_void;
    fn pcre2_match_data_free_32(data: *mut c_void);
    fn pcre2_match_32(
        code: *const c_void,
        s: *const u32,
        n: usize,
        start: usize,
        options: u32,
        data: *mut c_void,
        context: *mut c_void,
    ) -> i32;
    fn pcre2_match_context_create_32(context: *mut c_void) -> *mut c_void;
    fn pcre2_match_context_free_32(context: *mut c_void);
    fn pcre2_set_match_limit_32(context: *mut c_void, value: u32) -> i32;
    fn pcre2_set_depth_limit_32(context: *mut c_void, value: u32) -> i32;
    fn pcre2_set_heap_limit_32(context: *mut c_void, value: u32) -> i32;
    fn pcre2_set_callout_32(
        context: *mut c_void,
        callback: unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32,
        user: *mut c_void,
    ) -> i32;
    fn pcre2_get_ovector_pointer_32(data: *mut c_void) -> *mut usize;
}
pub struct Regex(*mut c_void);
impl Drop for Regex {
    fn drop(&mut self) {
        unsafe { pcre2_code_free_32(self.0) }
    }
}
fn class(ranges: &[(u32, u32)]) -> String {
    ranges
        .iter()
        .map(|&(a, b)| {
            if a == b {
                format!("\\x{{{a:x}}}")
            } else {
                format!("\\x{{{a:x}}}-\\x{{{b:x}}}")
            }
        })
        .collect()
}
fn translate(p: &str) -> Result<String, String> {
    let p = p.replace("(?u)", "");
    let cs: Vec<char> = p.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let mut in_class = false;
    while i < cs.len() {
        let c = cs[i];
        if c == '\\' {
            i += 1;
            if i == cs.len() {
                return Err("trailing regex escape".into());
            }
            let e = cs[i];
            match e {
                'u' | 'U' => {
                    let count = if e == 'u' { 4 } else { 8 };
                    if i + count >= cs.len() {
                        return Err("short Unicode escape".into());
                    }
                    let digits: String = cs[i + 1..=i + count].iter().collect();
                    u32::from_str_radix(&digits, 16).map_err(|_| "invalid Unicode escape")?;
                    out.push_str(&format!("\\x{{{digits}}}"));
                    i += count
                }
                'w' | 'd' | 's' | 'S' => {
                    let ranges = match e {
                        'w' => super::tables::ISALNUM,
                        'd' => super::tables::ISDECIMAL,
                        _ => super::tables::ISSPACE,
                    };
                    if e == 'S' && in_class {
                        return Err("negative escape in class unsupported".into());
                    }
                    if !in_class {
                        out.push('[');
                        if e == 'S' {
                            out.push('^')
                        }
                    }
                    out.push_str(&class(ranges));
                    if e == 'w' {
                        out.push('_')
                    }
                    if !in_class {
                        out.push(']')
                    }
                }
                'b' | 'B' | 'D' | 'W' => return Err(format!("unsupported frozen escape {e}")),
                _ => {
                    out.push('\\');
                    out.push(e)
                }
            }
        } else {
            if c == '[' {
                in_class = true
            } else if c == ']' {
                in_class = false
            }
            out.push(c)
        }
        i += 1;
    }
    Ok(out)
}
impl Regex {
    pub fn new(p: &str) -> Result<Self, String> {
        let translated = translate(p)?;
        let points: Vec<u32> = translated.chars().map(|c| c as u32).collect();
        let (mut error, mut offset) = (0, 0);
        let code = unsafe {
            pcre2_compile_32(
                points.as_ptr(),
                points.len(),
                0x00000004, // PCRE2_AUTO_CALLOUT: observe, never alter successful matches.
                &mut error,
                &mut offset,
                std::ptr::null_mut(),
            )
        };
        if code.is_null() {
            Err(format!("PCRE2 compile error {error} at {offset}"))
        } else {
            Ok(Self(code))
        }
    }
    fn find(
        &self,
        s: &[u32],
        start: usize,
        options: u32,
        control: Option<&RequestControl<'_>>,
    ) -> Result<Option<(usize, usize)>, InferenceError> {
        if let Some(c) = control {
            c.charge_regex_call()?;
        }
        let state = CalloutState {
            control,
            error: RefCell::new(None),
        };
        unsafe {
            // Both allocations are request-local. PCRE2 >=10.41 retains heap frames
            // in match_data; never reuse a larger request's frames under smaller limits.
            let context = pcre2_match_context_create_32(std::ptr::null_mut());
            if context.is_null() {
                return Err(InferenceError::Allocation {
                    resource: "regex context",
                });
            }
            let context = MatchContext(context);
            if let Some(c) = control {
                let l = c.limits();
                pcre2_set_match_limit_32(context.0, l.regex_match as u32);
                pcre2_set_depth_limit_32(context.0, l.regex_depth as u32);
                pcre2_set_heap_limit_32(context.0, l.regex_heap_kib as u32);
                pcre2_set_callout_32(context.0, callout, &state as *const _ as *mut c_void);
            }
            let data = pcre2_match_data_create_from_pattern_32(self.0, std::ptr::null_mut());
            if data.is_null() {
                return Err(InferenceError::Allocation {
                    resource: "regex match data",
                });
            }
            let data = MatchData(data);
            let result = pcre2_match_32(
                self.0,
                s.as_ptr(),
                s.len(),
                start,
                options,
                data.0,
                context.0,
            );
            if let Some(c) = control {
                c.checkpoint()?;
            }
            if let Some(error) = state.error.into_inner() {
                return Err(error);
            }
            if result >= 0 {
                let v = pcre2_get_ovector_pointer_32(data.0);
                Ok(Some((*v, *v.add(1))))
            } else if result == -1 {
                Ok(None)
            } else if let Some(c) = control {
                let (resource, limit) = match result {
                    -47 => ("regex match", c.limits().regex_match),
                    -53 => ("regex depth", c.limits().regex_depth),
                    -63 => ("regex heap KiB", c.limits().regex_heap_kib),
                    -48 => {
                        return Err(InferenceError::Allocation {
                            resource: "regex heap",
                        })
                    }
                    _ => return Err(InferenceError::Model(format!("PCRE2 match error {result}"))),
                };
                Err(InferenceError::ResourceLimit { resource, limit })
            } else {
                Err(InferenceError::Model(format!("PCRE2 match error {result}")))
            }
        }
    }
    pub fn search(&self, s: &str) -> Result<Option<(usize, usize)>, String> {
        self.search_controlled(s, None).map_err(|e| e.to_string())
    }
    pub fn search_bounded(
        &self,
        s: &str,
        control: &RequestControl<'_>,
    ) -> Result<Option<(usize, usize)>, InferenceError> {
        self.search_controlled(s, Some(control))
    }
    pub(crate) fn search_controlled(
        &self,
        s: &str,
        control: Option<&RequestControl<'_>>,
    ) -> Result<Option<(usize, usize)>, InferenceError> {
        let points = points(s, control)?;
        self.find(&points, 0, 0, control)
    }
    pub fn finditer(&self, s: &str) -> Result<Vec<(usize, usize)>, String> {
        self.finditer_controlled(s, None).map_err(|e| e.to_string())
    }
    pub fn finditer_bounded(
        &self,
        s: &str,
        control: &RequestControl<'_>,
    ) -> Result<Vec<(usize, usize)>, InferenceError> {
        self.finditer_controlled(s, Some(control))
    }
    pub(crate) fn finditer_controlled(
        &self,
        s: &str,
        control: Option<&RequestControl<'_>>,
    ) -> Result<Vec<(usize, usize)>, InferenceError> {
        let points = points(s, control)?;
        let mut out = vec![];
        let (mut start, mut empty) = (0, false);
        while start <= points.len() {
            let flags = if empty { 0x80000000 | 8 } else { 0 };
            match self.find(&points, start, flags, control)? {
                Some((a, b)) => {
                    if let Some(c) = control {
                        c.resource("regex matches", out.len() + 1, c.limits().input_chars + 1)?;
                        out.try_reserve(1).map_err(|_| InferenceError::Allocation {
                            resource: "regex matches",
                        })?;
                    }
                    out.push((a, b));
                    start = b;
                    empty = a == b
                }
                None if empty => {
                    start += 1;
                    empty = false
                }
                None => break,
            }
        }
        if let Some(c) = control {
            c.checkpoint()?;
        }
        Ok(out)
    }
}
fn points(s: &str, control: Option<&RequestControl<'_>>) -> Result<Vec<u32>, InferenceError> {
    if let Some(c) = control {
        c.resource("input bytes", s.len(), c.limits().input_bytes)?;
        let count = s.chars().count();
        c.resource("input characters", count, c.limits().input_chars)?;
        c.charge(count)?;
        let mut points = Vec::new();
        points
            .try_reserve_exact(count)
            .map_err(|_| InferenceError::Allocation {
                resource: "regex subject",
            })?;
        points.extend(s.chars().map(|c| c as u32));
        Ok(points)
    } else {
        Ok(s.chars().map(|c| c as u32).collect())
    }
}
struct MatchContext(*mut c_void);
impl Drop for MatchContext {
    fn drop(&mut self) {
        unsafe {
            pcre2_match_context_free_32(self.0);
        }
    }
}
struct MatchData(*mut c_void);
impl Drop for MatchData {
    fn drop(&mut self) {
        unsafe {
            pcre2_match_data_free_32(self.0);
        }
    }
}
struct CalloutState<'a, 'b> {
    control: Option<&'a RequestControl<'b>>,
    error: RefCell<Option<InferenceError>>,
}
unsafe extern "C" fn callout(_: *mut c_void, user: *mut c_void) -> i32 {
    // The state is stack-owned throughout this synchronous non-JIT match.
    // No panic, allocation, unwinding or Rust reference escapes this callback.
    #[cfg(test)]
    CANCEL_AT_CALLOUT.with(|v| {
        if let Some((remaining, token)) = v.borrow_mut().as_mut() {
            if *remaining == 0 {
                token.cancel();
            } else {
                *remaining -= 1;
            }
        }
    });
    let state = &*(user as *const CalloutState<'_, '_>);
    if let Some(c) = state.control {
        if let Err(error) = c.charge_regex_callout() {
            *state.error.borrow_mut() = Some(error);
            return -37; // PCRE2_ERROR_CALLOUT, never generated by PCRE2 itself.
        }
    }
    0
}

#[cfg(test)]
thread_local! {
    // Deterministic native-match cancellation injection, absent from release code.
    static CANCEL_AT_CALLOUT: RefCell<Option<(usize, std::sync::Arc<released_ner_probe::control::CancellationToken>)>> = const { RefCell::new(None) };
}
#[cfg(test)]
mod tests {
    use super::*;
    use released_ner_probe::control::{CancellationToken, Limits};
    #[test]
    fn cancellation_inside_native_callout_never_returns_match_or_nomatch() {
        let regex = Regex::new("(a)+$").unwrap();
        for text in ["aaaaaaaaaaaaaaaa", "aaaaaaaaaaaaaaaa!"] {
            let token = std::sync::Arc::new(CancellationToken::new());
            CANCEL_AT_CALLOUT.with(|v| *v.borrow_mut() = Some((8, token.clone())));
            let limits = Limits::default();
            let c = RequestControl::new(&limits, &token).unwrap();
            let result = regex.search_bounded(text, &c);
            CANCEL_AT_CALLOUT.with(|v| *v.borrow_mut() = None);
            assert!(token.is_cancelled(), "native callback was reached");
            assert!(matches!(result, Err(InferenceError::Cancelled)));
            let fresh = CancellationToken::new();
            let c = RequestControl::new(&limits, &fresh).unwrap();
            assert_eq!(regex.search_bounded("aaaa", &c).unwrap(), Some((0, 4)));
        }
    }
    #[test]
    fn nonzero_limits_exhaust_inside_native_match() {
        let regex = Regex::new("(a+)+$").unwrap();
        let token = CancellationToken::new();
        for (limits, expected) in [
            (
                Limits {
                    regex_match: 1,
                    ..Limits::default()
                },
                "regex match",
            ),
            (
                Limits {
                    regex_depth: 1,
                    ..Limits::default()
                },
                "regex depth",
            ),
            (
                Limits {
                    regex_heap_kib: 1,
                    ..Limits::default()
                },
                "regex heap KiB",
            ),
            (
                Limits {
                    regex_callouts: 8,
                    ..Limits::default()
                },
                "regex_callouts",
            ),
        ] {
            let c = RequestControl::new(&limits, &token).unwrap();
            let result = regex.search_bounded("aaaaaaaaaaaaaaaaaaaa!", &c);
            assert!(
                matches!(result, Err(InferenceError::ResourceLimit { resource, .. }) if resource == expected),
                "{expected}: {result:?}"
            );
        }
    }
}
