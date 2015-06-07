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
use program::Program;
use re::CaptureIdxs;

pub struct Backtrack<'r, 't, 'c> {
    prog: &'r Program,
    input: CharInput<'t>,
    caps: &'c mut CaptureIdxs,
    jobs: Vec<Job>,
    visited: HashSet<(usize, usize)>,
}

struct Job {
    pc: usize,
    at: InputAt,
    alt: bool,
}

impl Job {
    fn new(pc: usize, at: InputAt) -> Job {
        Job { pc: pc, at: at, alt: false }
    }

    fn alt(pc: usize, at: InputAt) -> Job {
        Job { pc: pc, at: at, alt: true }
    }
}

impl<'r, 't, 'c> Backtrack<'r, 't, 'c> {
    pub fn exec(
        prog: &'r Program,
        mut caps: &mut CaptureIdxs,
        text: &'t str,
        start: usize,
    ) -> bool {
        let input = CharInput::new(text);
        let start = input.start_at(start);
        Backtrack {
            prog: prog,
            input: input,
            caps: caps,
            jobs: vec![],
            visited: HashSet::new(),
        }.exec_(start)
    }

    fn exec_(&mut self, mut at: InputAt) -> bool {
        if self.prog.anchored_begin {
            return if !at.is_beginning() {
                false
            } else {
                false
                // match self.input.prefix_at(&self.prog.prefixes, at.pos()) {
                    // None => false,
                    // Some(adv) => {
                        // let at = self.input.start_at(at.pos() + adv);
                        // self.backtrack(at)
                    // }
                // }
            };
        }
        loop {
            println!("Before prefix match: {:?}", at);
            // at = match self.input.prefix_at(&self.prog.prefixes, at.pos()) {
                // None => return false,
                // Some(adv) => self.input.at(at.pos() + adv),
            // };
            println!("Starting backtracking at: {:?}", at);
            if self.backtrack(at) {
                return true;
            }
            at = self.input.at(at.next_pos());
            if at.is_end() {
                return false;
            }
        }
    }

    fn backtrack(&mut self, at: InputAt) -> bool {
        use program::Inst::*;

        self.jobs.push(Job::new(0, at));
        while let Some(j) = self.jobs.pop() {
            let at = j.at;
            match self.prog.insts[j.pc] {
                Match => return true,
                Char(ref inst) => {
                    println!("inst: {:?}, at: {:?}", inst, at);
                    if inst.matches(at.char()) {
                        println!("matched!");
                        let at_next = self.input.at(at.next_pos());
                        self.jobs.push(Job::new(j.pc + 1, at_next));
                    }
                }
                Ranges(ref inst) => {
                    if inst.matches(at.char()).is_some() {
                        let at_next = self.input.at(at.next_pos());
                        self.jobs.push(Job::new(j.pc + 1, at_next));
                    }
                }
                Jump(pc) => self.jobs.push(Job::new(pc, at)),
                EmptyLook(ref inst) => {
                    let at_next = self.input.at(at.next_pos());
                    println!("inst: {:?}, at: {:?}, next: {:?}",
                             inst, at, at_next);
                    if inst.matches(at.char(), at_next.char()) {
                        println!("matched!");
                        self.jobs.push(Job::new(j.pc + 1, at));
                    }
                }
                Save(slot) if !j.alt => {
                    if slot < self.caps.len() {
                        if let Some(i) = self.caps[slot] {
                            self.jobs.push(Job::alt(j.pc, self.input.at(i)));
                        }
                        self.caps[slot] = Some(at.pos());
                    }
                    self.jobs.push(Job::new(j.pc + 1, j.at));
                }
                Save(slot) if j.alt => {
                    self.caps[slot] = Some(at.pos());
                }
                _ => {}
            }
        }
        false
    }
}
