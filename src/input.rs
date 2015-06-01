use char::Char;

pub trait Input {
    fn next_byte_offset(&self) -> usize;
    fn cur(&self) -> Char;
    fn next(&self) -> Char;
    fn set(&mut self, byte_offset: usize);
    fn advance(&mut self);
    fn advance_prefix(&mut self, prefix: &str) -> bool;

    fn beginning(&self) -> bool { self.next_byte_offset() == 0 }
    fn done(&self) -> bool { self.cur().is_none() }
}

#[derive(Debug)]
pub struct CharInput<'t> {
    s: &'t str,
    cur: Char,
    next: Char,
    next_offset: usize,
}

impl<'t> CharInput<'t> {
    pub fn new(s: &'t str, start: usize) -> CharInput<'t> {
        let mut inp = CharInput {
            s: s,
            cur: None.into(),
            next: None.into(),
            next_offset: 0,
        };
        inp.set(start);
        inp
    }
}

impl<'t> Input for CharInput<'t> {
    #[inline] fn next_byte_offset(&self) -> usize { self.next_offset }
    #[inline] fn cur(&self) -> Char { self.cur }
    #[inline] fn next(&self) -> Char { self.next }

    fn set(&mut self, i: usize) {
        self.next_offset = i;
        self.cur = self.s[..i].chars().rev().next().into();
        self.next = self.s[self.next_offset..].chars().next().into();
    }

    fn advance(&mut self) {
        self.cur = self.next;
        self.next_offset += self.cur.len_utf8();
        self.next = self.s[self.next_offset..].chars().next().into();
    }

    fn advance_prefix(&mut self, prefix: &str) -> bool {
        let nexti = self.next_offset;
        let needle = prefix.as_bytes();
        let haystack = &self.s.as_bytes()[nexti..];
        match find_prefix(needle, haystack) {
            None => false,
            Some(i) => { self.set(nexti + i); true }
        }
    }
}

/// Returns the starting location of `needle` in `haystack`.
/// If `needle` is not in `haystack`, then `None` is returned.
///
/// Note that this is using a naive substring algorithm.
#[inline]
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
