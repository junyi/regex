// Copyright 2014-2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::collections::HashSet;

use input::{Input, InputAt, CharInput};
use program::{Inst, InstIdx, Program};
use re::CaptureIdxs;

#[derive(Debug)]
pub struct Backtrack<'r, 't, 'c> {
    prog: &'r Program,
    input: CharInput<'t>,
    caps: &'c mut CaptureIdxs,
    m: BackMachine,
}

#[derive(Debug)]
pub struct BackMachine {
    jobs: Vec<Job>,
    visited: HashSet<(usize, usize)>,
}

impl BackMachine {
    pub fn new(insts_len: usize) -> BackMachine {
        BackMachine {
            jobs: Vec::with_capacity(insts_len),
            visited: HashSet::new(),
        }
    }

    fn clear(&mut self) {
        // self.jobs.clear();
        unsafe { self.jobs.set_len(0); }
        self.visited.clear();
    }
}

#[derive(Clone, Copy, Debug)]
enum Job {
    Inst { pc: InstIdx, at: InputAt },
    SaveRestore { slot: usize, old_pos: Option<usize> },
    SplitNext { pc: InstIdx, at: InputAt },
}

impl<'r, 't, 'c> Backtrack<'r, 't, 'c> {
    pub fn exec(
        prog: &'r Program,
        mut caps: &mut CaptureIdxs,
        text: &'t str,
        start: usize,
    ) -> bool {
        let input = CharInput::new(text);
        let start = input.at(start);
        let mut m = prog.backtrack.get();
        m.clear();
        let mut b = Backtrack {
            prog: prog,
            input: input,
            caps: caps,
            m: m,
        };
        let matched = b.exec_(start);
        prog.backtrack.put(b.m);
        matched
    }

    fn exec_(&mut self, mut at: InputAt) -> bool {
        if self.prog.anchored_begin {
            return if !at.is_beginning() {
                false
            } else {
                match self.input.prefix_at(&self.prog.prefixes, at) {
                    None => false,
                    Some(at) => self.backtrack(at),
                }
            };
        }
        loop {
            at = match self.input.prefix_at(&self.prog.prefixes, at) {
                None => return false,
                Some(at) => at,
            };
            // println!("Starting backtracking at: {:?}", at);
            if self.backtrack(at) {
                return true;
            }
            if at.is_end() {
                return false;
            }
            at = self.input.at(at.next_pos());
        }
    }

    fn backtrack(&mut self, start: InputAt) -> bool {
        self.push(0, start);
        while let Some(job) = self.m.jobs.pop() {
            match job {
                Job::Inst { pc, at } => {
                    if self.step(pc, at) {
                        return true;
                    }
                }
                Job::SaveRestore { slot, old_pos } => {
                    self.caps[slot] = old_pos;
                }
                Job::SplitNext { pc, at } => {
                    self.push(pc, at);
                }
            }
        }
        false
    }

    fn step(&mut self, mut pc: InstIdx, mut at: InputAt) -> bool {
        use program::Inst::*;
        loop {
            match self.prog.insts[pc] {
                Match => return true,
                Save(slot) => {
                    if slot < self.caps.len() {
                        // If this path doesn't work out, then we save the old
                        // capture index (if one exists) in an alternate
                        // job. If the next path fails, then the alternate
                        // job is popped and the old capture index is restored.
                        let old_pos = self.caps[slot];
                        self.push_save_restore(slot, old_pos);
                        self.caps[slot] = Some(at.pos());
                    }
                    pc += 1;
                }
                Jump(pc2) => pc = pc2,
                Split(x, y) => {
                    self.push_split_next(y, at);
                    pc = x;
                }
                EmptyLook(ref inst) => {
                    let prev = self.input.previous_at(at.pos());
                    if inst.matches(prev.char(), at.char()) {
                        pc += 1;
                    } else {
                        return false;
                    }
                }
                Char(ref inst) => {
                    if inst.matches(at.char()) {
                        pc += 1;
                        at = self.input.at(at.next_pos());
                    } else {
                        return false;
                    }
                }
                Ranges(ref inst) => {
                    if inst.matches(at.char()).is_some() {
                        pc += 1;
                        at = self.input.at(at.next_pos());
                    } else {
                        return false;
                    }
                }
            }
            if self.has_visited(pc, at) {
                return false;
            }
        }
    }

    fn push(&mut self, pc: InstIdx, at: InputAt) {
        if !self.has_visited(pc, at) {
            self.m.jobs.push(Job::Inst { pc: pc, at: at });
        }
    }

    fn push_save_restore(&mut self, slot: usize, old_pos: Option<usize>) {
        self.m.jobs.push(Job::SaveRestore { slot: slot, old_pos: old_pos });
    }

    fn push_split_next(&mut self, pc: InstIdx, at: InputAt) {
        self.m.jobs.push(Job::SplitNext { pc: pc, at: at });
    }

    fn has_visited(&mut self, pc: InstIdx, at: InputAt) -> bool {
        let key = (pc, at.pos());
        if !self.m.visited.contains(&key) {
            self.m.visited.insert(key);
            false
        } else {
            true
        }
    }
}
