use std::cmp;
use std::ops;

use char::Char;

#[derive(Clone, Copy, Debug)]
pub struct InputAt {
    pos: usize,
    c: Char,
    len: usize,
}

impl InputAt {
    pub fn beginning() -> InputAt {
        InputAt {
            pos: 0,
            c: None.into(),
            len: 0,
        }
    }

    pub fn is_beginning(&self) -> bool {
        self.pos == 0
    }

    pub fn is_end(&self) -> bool {
        self.c.is_none()
    }

    pub fn char(&self) -> Char {
        self.c
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn next_pos(&self) -> usize {
        self.pos + self.len
    }
}

pub trait Input {
    fn at(&self, i: usize) -> InputAt;
    fn previous_at(&self, i: usize) -> InputAt;
    fn prefix_at(&self, prefixes: &[String], at: InputAt) -> Option<InputAt>;
}

#[derive(Debug)]
pub struct CharInput<'t>(&'t str);

impl<'t> CharInput<'t> {
    pub fn new(s: &'t str) -> CharInput<'t> {
        CharInput(s)
    }
}

impl<'t> ops::Deref for CharInput<'t> {
    type Target = str;

    fn deref(&self) -> &str {
        self.0
    }
}

impl<'t> Input for CharInput<'t> {
    fn at(&self, i: usize) -> InputAt {
        let c = self[i..].chars().next().into();
        InputAt {
            pos: i,
            c: c,
            len: c.len_utf8(),
        }
    }

    fn previous_at(&self, i: usize) -> InputAt {
        let c: Char = self[..i].chars().rev().next().into();
        let len = c.len_utf8();
        InputAt {
            pos: i - len,
            c: c,
            len: len,
        }
    }

    fn prefix_at(&self, prefixes: &[String], at: InputAt) -> Option<InputAt> {
        let haystack = &self.as_bytes()[at.pos()..];
        match prefixes.len() {
            0 => return Some(at), // empty prefix always matches!
            1 => find_prefix(prefixes[0].as_bytes(), haystack),
            _ => find_prefixes(prefixes, haystack),
        }.map(|adv| self.at(at.pos() + adv))
    }
}

pub fn find_prefix(needle: &[u8], haystack: &[u8]) -> Option<usize> {
    let (hlen, nlen) = (haystack.len(), needle.len());
    if nlen > hlen || nlen == 0 {
        return None
    }
    for (offset, window) in haystack.windows(nlen).enumerate() {
        if window == needle {
            return Some(offset)
        }
    }
    None
}

pub fn find_prefixes(needles: &[String], haystack: &[u8]) -> Option<usize> {
    for hi in 0..haystack.len() {
        for needle in needles {
            let ub = cmp::min(hi + needle.len(), haystack.len());
            if &haystack[hi..ub] == needle.as_bytes() {
                return Some(hi);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_find_prefixes() {
        let needles = &[
            "abaa".into(), "abbaa".into(), "abbbaa".into(), "abbbbaa".into(),
        ];
        let haystack = b"ababbabbbabbbabbbbabbbbaa";
        assert_eq!(super::find_prefixes(needles, haystack), Some(18));
    }
}
