use std::char;
use std::cmp::Ordering;
use std::fmt;
use std::u32;

use syntax;

#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Char(u32);

impl fmt::Debug for Char {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match char::from_u32(self.0) {
            None => write!(f, "Empty"),
            Some(c) => write!(f, "{:?}", c),
        }
    }
}

impl Char {
    #[inline]
    pub fn is_none(self) -> bool { self.0 == u32::MAX }

    pub fn len_utf8(self) -> usize {
        char::from_u32(self.0).map(|c| c.len_utf8()).unwrap_or(0)
    }

    pub fn case_fold(self) -> Char {
        char::from_u32(self.0).map(syntax::simple_case_fold).into()
    }

    pub fn is_word_char(self) -> bool {
        char::from_u32(self.0).map(syntax::is_word_char).unwrap_or(false)
    }
}

impl From<char> for Char {
    fn from(c: char) -> Char { Char(c as u32) }
}

impl From<Option<char>> for Char {
    fn from(c: Option<char>) -> Char {
        c.map(|c| c.into()).unwrap_or(Char(u32::MAX))
    }
}

impl PartialEq<char> for Char {
    #[inline] fn eq(&self, other: &char) -> bool { self.0 == *other as u32 }
}

impl PartialEq<Char> for char {
    #[inline] fn eq(&self, other: &Char) -> bool { *self as u32 == other.0 }
}

impl PartialOrd<char> for Char {
    #[inline]
    fn partial_cmp(&self, other: &char) -> Option<Ordering> {
        self.0.partial_cmp(&(*other as u32))
    }
}

impl PartialOrd<Char> for char {
    #[inline]
    fn partial_cmp(&self, other: &Char) -> Option<Ordering> {
        (*self as u32).partial_cmp(&other.0)
    }
}
