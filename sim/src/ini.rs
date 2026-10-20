//! Reader for the original game's `.ini` files (already re-encoded to UTF-8).
//!
//! Lookups mimic `GetPrivateProfileString`: section and key names are case-insensitive,
//! the first occurrence wins, and a missing key yields the caller's default.

use std::collections::HashMap;

#[derive(Debug, Default, Clone)]
pub struct Ini {
    sections: HashMap<String, HashMap<String, String>>,
}

impl Ini {
    pub fn parse(text: &str) -> Ini {
        let mut ini = Ini::default();
        let mut current = String::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                current = name.trim().to_lowercase();
                ini.sections.entry(current.clone()).or_default();
            } else if let Some((key, value)) = line.split_once('=') {
                let section = ini.sections.entry(current.clone()).or_default();
                section
                    .entry(key.trim().to_lowercase())
                    .or_insert_with(|| unquote(value.trim()).to_owned());
            }
        }
        ini
    }

    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.sections
            .get(&section.to_lowercase())?
            .get(&key.to_lowercase())
            .map(String::as_str)
    }

    pub fn str_or<'a>(&'a self, section: &str, key: &str, default: &'a str) -> &'a str {
        self.get(section, key).unwrap_or(default)
    }

    /// Like VB's `Val()`: parses the leading number and ignores trailing garbage.
    pub fn num_or(&self, section: &str, key: &str, default: f64) -> f64 {
        self.get(section, key).map(vb_val).unwrap_or(default)
    }

    pub fn int_or(&self, section: &str, key: &str, default: i64) -> i64 {
        self.get(section, key).map(|v| vb_val(v) as i64).unwrap_or(default)
    }

    /// The value of `key` in whichever section defines it first (by section name order).
    pub fn find(&self, key: &str) -> Option<&str> {
        let key = key.to_lowercase();
        let mut names: Vec<&String> = self.sections.keys().collect();
        names.sort();
        names.into_iter().find_map(|n| self.sections[n].get(&key).map(String::as_str))
    }

    pub fn has_section(&self, section: &str) -> bool {
        self.sections.contains_key(&section.to_lowercase())
    }

    pub fn section(&self, section: &str) -> Option<&HashMap<String, String>> {
        self.sections.get(&section.to_lowercase())
    }
}

fn unquote(s: &str) -> &str {
    s.strip_prefix('"').and_then(|s| s.strip_suffix('"')).unwrap_or(s)
}

/// VB `Val()`: leading whitespace skipped, accepts sign, digits and one '.', stops at the
/// first other character; returns 0 if nothing parses. (`Val` ignores the locale, so ','
/// is never a decimal separator.)
pub fn vb_val(s: &str) -> f64 {
    let s = s.trim_start();
    let mut end = 0;
    let mut seen_dot = false;
    for (i, c) in s.char_indices() {
        match c {
            '+' | '-' if i == 0 => {}
            '0'..='9' => {}
            '.' if !seen_dot => seen_dot = true,
            _ => break,
        }
        end = i + c.len_utf8();
    }
    s[..end].parse().unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_lookup() {
        let ini = Ini::parse("; c\n[Sec]\nKey = 12abc\nKey=99\nName=\"Hé\"\n[Other]\nx=-1.5\n");
        assert_eq!(ini.int_or("sec", "KEY", 0), 12);
        assert_eq!(ini.str_or("SEC", "name", ""), "Hé");
        assert_eq!(ini.num_or("other", "x", 0.0), -1.5);
        assert_eq!(ini.int_or("other", "missing", 7), 7);
    }

    #[test]
    fn val_like_vb() {
        assert_eq!(vb_val("  3.5.2"), 3.5);
        assert_eq!(vb_val("abc"), 0.0);
        assert_eq!(vb_val("-"), 0.0);
    }
}
