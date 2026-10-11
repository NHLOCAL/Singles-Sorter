//! Narrow PCRE2-32 bridge. Frozen regex syntax translated explicitly; no UCP tables.
use std::ffi::c_void;
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
                0,
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
    ) -> Result<Option<(usize, usize)>, String> {
        unsafe {
            let data = pcre2_match_data_create_from_pattern_32(self.0, std::ptr::null_mut());
            if data.is_null() {
                return Err("regex allocation failed".into());
            }
            let result = pcre2_match_32(
                self.0,
                s.as_ptr(),
                s.len(),
                start,
                options,
                data,
                std::ptr::null_mut(),
            );
            let matched = if result >= 0 {
                let v = pcre2_get_ovector_pointer_32(data);
                Ok(Some((*v, *v.add(1))))
            } else if result == -1 {
                Ok(None)
            } else {
                Err(format!("PCRE2 match error {result}"))
            };
            pcre2_match_data_free_32(data);
            matched
        }
    }
    pub fn search(&self, s: &str) -> Result<Option<(usize, usize)>, String> {
        self.find(&s.chars().map(|c| c as u32).collect::<Vec<_>>(), 0, 0)
    }
    pub fn finditer(&self, s: &str) -> Result<Vec<(usize, usize)>, String> {
        let points: Vec<u32> = s.chars().map(|c| c as u32).collect();
        let mut out = vec![];
        let (mut start, mut empty) = (0, false);
        while start <= points.len() {
            let flags = if empty { 0x80000000 | 8 } else { 0 };
            match self.find(&points, start, flags)? {
                Some((a, b)) => {
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
        Ok(out)
    }
}
