// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// FIXME: Currently, the VM simulates an NFA. It would be nice to have another
// VM that simulates a DFA.
//
// According to Russ Cox[1], a DFA performs better than an NFA, principally
// because it reuses states previously computed by the machine *and* doesn't
// keep track of capture groups. The drawback of a DFA (aside from its
// complexity) is that it can't accurately return the locations of submatches.
// The NFA *can* do that. (This is my understanding anyway.)
//
// Cox suggests that a DFA ought to be used to answer "does this match" and
// "where does it match" questions. (In the latter, the starting position of
// the match is computed by executing the regex backwards.) Cox also suggests
// that a DFA should be run when asking "where are the submatches", which can
// 1) quickly answer "no" is there's no match and 2) discover the substring
// that matches, which means running the NFA on smaller input.
//
// Currently, the NFA simulation implemented below does some dirty tricks to
// avoid tracking capture groups when they aren't needed (which only works
// for 'is_match', not 'find'). This is a half-measure, but does provide some
// perf improvement.
//
// AFAIK, the DFA/NFA approach is implemented in RE2/C++ but *not* in RE2/Go.
//
// [1] - http://swtch.com/~rsc/regex/regex3.html

use std::mem;

use program::Program;
use input::{Input, CharInput};

pub type CaptureIdxs = [Option<usize>];

#[derive(Debug)]
pub struct Nfa<'r, 't> {
    prog: &'r Program,
    input: CharInput<'t>,
}

/// Indicates the next action to take after a single non-empty instruction
/// is processed.
#[derive(Copy, Clone, Debug)]
pub enum Step {
    /// This is returned if and only if a Match instruction is reached and
    /// we only care about the existence of a match. It instructs the VM to
    /// quit early.
    MatchEarlyReturn,
    /// Indicates that a match was found. Thus, the rest of the states in the
    /// *current* queue should be dropped (i.e., leftmost-first semantics).
    /// States in the "next" queue can still be processed.
    Match,
    /// No match was found. Continue with the next state in the queue.
    Continue,
}

impl<'r, 't> Nfa<'r, 't> {
    /// Runs an NFA simulation on the compiled expression given on the search
    /// text `input`. The search begins at byte index `start` and ends at byte
    /// index `end`. (The range is specified here so that zero-width assertions
    /// will work correctly when searching for successive non-overlapping
    /// matches.)
    ///
    /// The `which` parameter indicates what kind of capture information the
    /// caller wants. There are three choices: match existence only, the
    /// location of the entire match or the locations of the entire match in
    /// addition to the locations of each submatch.
    pub fn run(prog: &'r Program, mut caps: &mut CaptureIdxs, text: &'t str,
               start: usize) -> bool {
        let mut q = prog.nfa_threads.get();
        let matched = Nfa {
            prog: prog,
            input: CharInput::new(text, start),
        }.exec(&mut q, &mut caps);
        prog.nfa_threads.put(q);
        matched
    }

    fn exec(&mut self, mut q: &mut NfaThreads, mut caps: &mut CaptureIdxs) -> bool {
        let mut matched = false;
        q.clist.empty(); q.nlist.empty();
'LOOP:  loop {
            if q.clist.size == 0 {
                // We have a match and we're done exploring alternatives.
                // Time to quit.
                if matched {
                    break
                }

                // If the expression starts with a '^' we can terminate as soon
                // as the last thread dies.
                if !self.input.beginning() && self.prog.anchored_begin {
                    break;
                }

                // If there are no threads to try, then we'll have to start
                // over at the beginning of the regex.
                // BUT, if there's a literal prefix for the program, try to
                // jump ahead quickly. If it can't be found, then we can bail
                // out early.
                if self.prog.prefix.len() > 0
                        && !self.input.advance_prefix(&self.prog.prefix) {
                    // Has a prefix but we couldn't find one, so we're done.
                    break;
                }
            }

            // This simulates a preceding '.*?' for every regex by adding
            // a state starting at the current position in the input for the
            // beginning of the program only if we don't already have a match.
            if q.clist.size == 0 || (!self.prog.anchored_begin && !matched) {
                self.add(&mut q.clist, 0, &mut caps)
            }
            // The previous call to "add" actually inspects the position just
            // before the current character. For stepping through the machine,
            // we can to look at the current character, so we advance the
            // input.
            self.input.advance();
            for i in 0..q.clist.size {
                let pc = q.clist.pc(i);
                let step_state = self.step(caps, &mut q.nlist,
                                           q.clist.groups(i), pc);
                match step_state {
                    Step::MatchEarlyReturn => { matched = true; break 'LOOP }
                    Step::Match => { matched = true; break }
                    Step::Continue => {}
                }
            }
            if self.input.cur().is_none() {
                break;
            }
            mem::swap(&mut q.clist, &mut q.nlist);
            q.nlist.empty();
        }
        matched
    }

    fn step(&self, caps: &mut [Option<usize>], nlist: &mut Threads,
            thread_caps: &mut [Option<usize>], pc: usize)
           -> Step {
        use program::Inst::*;

        match self.prog.insts[pc] {
            Match => {
                if caps.len() == 0 {
                    return Step::MatchEarlyReturn;
                } else {
                    for (slot, val) in caps.iter_mut().zip(thread_caps.iter()) {
                        *slot = *val;
                    }
                    return Step::Match;
                }
            }
            Char(ref inst) => {
                if inst.matches(self.input.cur()) {
                    self.add(nlist, pc+1, thread_caps);
                }
            }
            Ranges(ref inst) => {
                if inst.matches(self.input.cur()).is_some() {
                    self.add(nlist, pc+1, thread_caps);
                }
            }
            EmptyLook(_) | Save(_) | Jump(_) | Split(_, _) => {},
        }
        Step::Continue
    }

    fn add(&self, nlist: &mut Threads, pc: usize, thread_caps: &mut [Option<usize>]) {
        use program::Inst::*;

        if nlist.contains(pc) {
            return
        }
        let ti = nlist.add(pc);
        match self.prog.insts[pc] {
            EmptyLook(ref inst) => {
                if inst.matches(self.input.cur(), self.input.next()) {
                    self.add(nlist, pc + 1, thread_caps);
                }
            }
            Save(slot) => {
                if slot >= thread_caps.len() {
                    self.add(nlist, pc + 1, thread_caps);
                } else {
                    let old = thread_caps[slot];
                    thread_caps[slot] = Some(self.input.next_byte_offset());
                    self.add(nlist, pc + 1, thread_caps);
                    thread_caps[slot] = old;
                }
            }
            Jump(to) => {
                self.add(nlist, to, thread_caps)
            }
            Split(x, y) => {
                self.add(nlist, x, thread_caps);
                self.add(nlist, y, thread_caps);
            }
            Match | Char(_) | Ranges(_) => {
                let mut t = &mut nlist.queue[ti];
                for (slot, val) in t.caps.iter_mut().zip(thread_caps.iter()) {
                    *slot = *val;
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct NfaThreads {
    clist: Threads,
    nlist: Threads,
}

#[derive(Debug)]
struct Threads {
    queue: Vec<Thread>,
    sparse: Vec<usize>,
    size: usize,
}

#[derive(Clone, Debug)]
struct Thread {
    pc: usize,
    caps: Vec<Option<usize>>,
}

impl NfaThreads {
    pub fn new(num_insts: usize, ncaps: usize) -> NfaThreads {
        NfaThreads {
            clist: Threads::new(num_insts, ncaps),
            nlist: Threads::new(num_insts, ncaps),
        }
    }
}

impl Threads {
    fn new(num_insts: usize, ncaps: usize) -> Threads {
        let t = Thread { pc: 0, caps: vec![None; ncaps * 2] };
        Threads {
            queue: vec![t; num_insts],
            sparse: vec![0; num_insts],
            size: 0,
        }
    }

    #[inline]
    fn add(&mut self, pc: usize) -> usize {
        let i = self.size;
        self.queue[i].pc = pc;
        self.sparse[pc] = i;
        self.size += 1;
        i
    }

    #[inline]
    fn contains(&self, pc: usize) -> bool {
        let s = self.sparse[pc];
        s < self.size && self.queue[s].pc == pc
    }

    #[inline]
    fn empty(&mut self) {
        self.size = 0;
    }

    #[inline]
    fn pc(&self, i: usize) -> usize {
        self.queue[i].pc
    }

    #[inline]
    fn groups(&mut self, i: usize) -> &mut [Option<usize>] {
        &mut self.queue[i].caps
    }
}
