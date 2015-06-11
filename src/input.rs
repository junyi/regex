use std::ops;

use char::Char;
use prefix;

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
    #[inline(always)]
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
            1 => prefix::find_one(prefixes[0].as_bytes(), haystack),
            _ => prefix::find_any(prefixes, haystack),
        }.map(|adv| self.at(at.pos() + adv))
    }
}
