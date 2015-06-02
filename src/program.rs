// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::cmp::{self, Ordering};

use syntax;

use Error;
use char::Char;
use compile::Compiler;
use nfa::NfaThreads;
use pool::Pool;

pub type InstIdx = usize;

/// An instruction, the underlying unit of a compiled regular expression
#[derive(Clone, Debug)]
pub enum Inst {
    Match,
    Save(usize),
    Jump(InstIdx),
    Split(InstIdx, InstIdx),
    EmptyLook(LookInst),
    Char(OneChar),
    Ranges(CharRanges),
}

#[derive(Clone, Debug)]
pub struct OneChar {
    pub c: char,
    pub casei: bool,
}

#[derive(Clone, Debug)]
pub struct CharRanges {
    pub ranges: Vec<(char, char)>,
    pub casei: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LookInst {
    StartLine,
    EndLine,
    StartText,
    EndText,
    WordBoundary,
    NotWordBoundary,
}

impl Inst {
    fn as_literal(&self) -> Option<char> {
        match *self {
            Inst::Char(OneChar { c, casei: false }) => Some(c),
            _ => None,
        }
    }

    fn is_literal(&self) -> bool {
        match *self {
            Inst::Char(OneChar { casei: false, .. }) => true,
            _ => false,
        }
    }
}

impl OneChar {
    pub fn matches(&self, c: Char) -> bool {
        self.c == c || (self.casei && self.c == c.case_fold())
    }
}

impl CharRanges {
    pub fn any() -> CharRanges {
        CharRanges {
            ranges: vec![('\x00', '\u{10ffff}')],
            casei: false,
        }
    }

    pub fn any_nonl() -> CharRanges {
        CharRanges {
            ranges: vec![('\x00', '\x09'), ('\x0B', '\u{10ffff}')],
            casei: false,
        }
    }

    pub fn from_class(cls: syntax::CharClass) -> CharRanges {
        let casei = cls.is_case_insensitive();
        CharRanges {
            ranges: cls.into_iter().map(|r| (r.start, r.end)).collect(),
            casei: casei,
        }
    }

    pub fn matches(&self, mut c: Char) -> Option<usize> {
        if self.casei {
            c = c.case_fold();
        }
        // This speeds up the `match_class_unicode` benchmark by checking
        // some common cases quickly without binary search. e.g., Matching
        // a Unicode class on predominantly ASCII text.
        for i in 0..cmp::min(self.ranges.len(), 4) {
            let r = self.ranges[i];
            if c < r.0 {
                return None;
            }
            if c <= r.1 {
                return Some(i);
            }
        }
        self.ranges.binary_search_by(|r| {
            if r.1 < c {
                Ordering::Less
            } else if r.0 > c {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        }).ok()
    }
}

impl LookInst {
    pub fn matches(&self, c1: Char, c2: Char) -> bool {
        use self::LookInst::*;
        match *self {
            StartLine => c1.is_none() || c1 == '\n',
            EndLine => c2.is_none() || c2 == '\n',
            StartText => c1.is_none(),
            EndText => c2.is_none(),
            ref wbty => {
                let (w1, w2) = (c1.is_word_char(), c2.is_word_char());
                (*wbty == WordBoundary && w1 ^ w2)
                || (*wbty == NotWordBoundary && !(w1 ^ w2))
            }
        }
    }
}

/// Program represents a compiled regular expression. Once an expression is
/// compiled, its representation is immutable and will never change.
///
/// All of the data in a compiled expression is wrapped in "MaybeStatic" or
/// "MaybeOwned" types so that a `Program` can be represented as static data.
/// (This makes it convenient and efficient for use with the `regex!` macro.)
#[derive(Debug)]
pub struct Program {
    /// The original regular expression string.
    pub original: String,
    /// A sequence of instructions.
    pub insts: Vec<Inst>,
    /// The sequence of capture group names. There is an entry for each capture
    /// group index and a name exists only if the capture group is named.
    pub cap_names: Vec<Option<String>>,
    /// If the regular expression requires a literal prefix in order to have a
    /// match, that prefix is stored here.
    pub prefixes: Vec<String>,
    /// True iff program is anchored at the beginning.
    pub anchored_begin: bool,
    /// True iff program is anchored at the end.
    pub anchored_end: bool,
    /// Cached NFA threads.
    pub nfa_threads: Pool<NfaThreads>,
}

impl Program {
    /// Compiles a Regex.
    pub fn new(size_limit: usize, re: &str) -> Result<Program, Error> {
        let expr = try!(syntax::Expr::parse(re));
        let (insts, cap_names) = try!(Compiler::new(size_limit).compile(expr));
        let (insts_len, ncaps) = (insts.len(), num_captures(&insts));
        let create_threads = move || NfaThreads::new(insts_len, ncaps);
        let mut prog = Program {
            original: re.into(),
            insts: insts,
            cap_names: cap_names,
            prefixes: vec![],
            anchored_begin: false,
            anchored_end: false,
            nfa_threads: Pool::new(Box::new(create_threads)),
        };

        prog.find_prefixes();
        prog.anchored_begin = match prog.insts[1] {
            Inst::EmptyLook(LookInst::StartText) => true,
            _ => false,
        };
        prog.anchored_end = match prog.insts[prog.insts.len() - 3] {
            Inst::EmptyLook(LookInst::EndText) => true,
            _ => false,
        };
        Ok(prog)
    }

    /// Returns the total number of capture groups in the regular expression.
    /// This includes the zeroth capture.
    pub fn num_captures(&self) -> usize {
        num_captures(&self.insts)
    }

    pub fn alloc_captures(&self) -> Vec<Option<usize>> {
        vec![None; 2 * self.num_captures()]
    }

    pub fn find_prefixes(&mut self) {
        use self::Inst::*;

        fn prefix(insts: &[Inst]) -> String {
            let mut s = String::new();
            for inst in insts {
                match inst.as_literal() {
                    Some(c) => s.push(c),
                    None => break,
                }
            }
            s
        }
        if self.insts[1].is_literal() {
            self.prefixes.push(prefix(&self.insts[1..]));
            return;
        }
        let mut pc = 1;
        let mut prefixes = vec![];
        loop {
            match self.insts[pc] {
                Split(x, y) => {
                    match (&self.insts[x], &self.insts[y]) {
                        (&Char(OneChar { casei: false, .. }),
                         &Char(OneChar { casei: false, .. })) => {
                            prefixes.push(prefix(&self.insts[x..]));
                            prefixes.push(prefix(&self.insts[y..]));
                            break;
                        }
                        (&Char(OneChar { casei: false, .. }), &Split(_, _)) => {
                            prefixes.push(prefix(&self.insts[x..]));
                            pc = y;
                        }
                        (&Split(_, _), &Char(OneChar { casei: false, .. })) => {
                            prefixes.push(prefix(&self.insts[y..]));
                            pc = x;
                        }
                        _ => return,
                    }
                }
                _ => return,
            }
        }
        self.prefixes = prefixes;
    }
}

impl Clone for Program {
    fn clone(&self) -> Program {
        let (insts_len, ncaps) = (self.insts.len(), self.num_captures());
        let create_threads = move || NfaThreads::new(insts_len, ncaps);
        Program {
            original: self.original.clone(),
            insts: self.insts.clone(),
            cap_names: self.cap_names.clone(),
            prefixes: self.prefixes.clone(),
            anchored_begin: self.anchored_begin,
            anchored_end: self.anchored_end,
            nfa_threads: Pool::new(Box::new(create_threads)),
        }
    }
}

pub fn num_captures(insts: &[Inst]) -> usize {
    let mut n = 0;
    for inst in insts {
        match *inst {
            Inst::Save(c) => n = cmp::max(n, c+1),
            _ => {}
        }
    }
    // There's exactly 2 Save slots for every capture.
    n / 2
}
