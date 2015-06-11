// Copyright 2014-2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(test)]

extern crate libc;
extern crate test;

use std::iter;

use test::Bencher;

#[path = "../../src/prefix.rs"]
mod prefix;

#[bench]
fn one_byte(b: &mut Bencher) {
    let haystack: String = iter::repeat("z").take(10000).collect();
    let haystack = haystack.as_bytes();
    let needle = b"a";
    b.iter(|| {
        assert!(prefix::find_one(needle, haystack).is_none());
    });
}

#[bench]
fn one(b: &mut Bencher) {
    let haystack: String = iter::repeat("z").take(10000).collect();
    let haystack = haystack.as_bytes();
    let needle = b"abc";
    b.iter(|| {
        assert!(prefix::find_one(needle, haystack).is_none());
    });
}

#[bench]
fn one_tricky(b: &mut Bencher) {
    // We lose the benefit of `memchr` because the first byte matches
    // in every position in the haystack.
    let haystack: String = iter::repeat("z").take(10000).collect();
    let haystack = haystack.as_bytes();
    let needle = b"zbc";
    b.iter(|| {
        assert!(prefix::find_one(needle, haystack).is_none());
    });
}

#[bench]
fn two_bytes(b: &mut Bencher) {
    let haystack: String = iter::repeat("z").take(10000).collect();
    let haystack = haystack.as_bytes();
    let needles = &["a", "b"];
    let needles: Vec<String> = needles.iter().map(|&s| s.to_owned()).collect();
    b.iter(|| {
        assert!(prefix::find_any(&needles, haystack).is_none());
    });
}

#[bench]
fn two(b: &mut Bencher) {
    let haystack: String = iter::repeat("z").take(10000).collect();
    let haystack = haystack.as_bytes();
    let needles = &["abcdef", "bmnopq"];
    let needles: Vec<String> = needles.iter().map(|&s| s.to_owned()).collect();
    b.iter(|| {
        assert!(prefix::find_any(&needles, haystack).is_none());
    });
}

#[bench]
fn two_tricky(b: &mut Bencher) {
    let haystack: String = iter::repeat("z").take(10000).collect();
    let haystack = haystack.as_bytes();
    let needles = &["zbcdef", "zmnopq"];
    let needles: Vec<String> = needles.iter().map(|&s| s.to_owned()).collect();
    b.iter(|| {
        assert!(prefix::find_any(&needles, haystack).is_none());
    });
}

#[bench]
fn ten_bytes(b: &mut Bencher) {
    let haystack: String = iter::repeat("z").take(10000).collect();
    let haystack = haystack.as_bytes();
    let needles = &["a", "b", "c", "d", "e",
                    "f", "g", "h", "i", "j"];
    let needles: Vec<String> = needles.iter().map(|&s| s.to_owned()).collect();
    b.iter(|| {
        assert!(prefix::find_any(&needles, haystack).is_none());
    });
}

#[bench]
fn ten(b: &mut Bencher) {
    let haystack: String = iter::repeat("z").take(10000).collect();
    let haystack = haystack.as_bytes();
    let needles = &["abcdef", "bbcdef", "cbcdef", "dbcdef",
                    "ebcdef", "fbcdef", "gbcdef", "hbcdef",
                    "ibcdef", "jbcdef"];
    let needles: Vec<String> = needles.iter().map(|&s| s.to_owned()).collect();
    b.iter(|| {
        assert!(prefix::find_any(&needles, haystack).is_none());
    });
}

#[bench]
fn ten_tricky(b: &mut Bencher) {
    let haystack: String = iter::repeat("z").take(10000).collect();
    let haystack = haystack.as_bytes();
    let needles = &["zacdef", "zbcdef", "zccdef", "zdcdef",
                    "zecdef", "zfcdef", "zgcdef", "zhcdef",
                    "zicdef", "zjcdef"];
    let needles: Vec<String> = needles.iter().map(|&s| s.to_owned()).collect();
    b.iter(|| {
        assert!(prefix::find_any(&needles, haystack).is_none());
    });
}
