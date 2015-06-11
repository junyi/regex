use std::cmp;

pub fn find_one(needle: &[u8], haystack: &[u8]) -> Option<usize> {
    let (hlen, nlen) = (haystack.len(), needle.len());
    if nlen > hlen || nlen == 0 {
        return None;
    } else if nlen == 1 {
        return memchr(needle[0], haystack);
    } else if nlen == hlen {
        return if needle == haystack { Some(0) } else { None };
    }

    let mut cur = 0;
    // TODO: Use Rabin Karp? But then we can't use memchr.
    while let Some(i) = memchr(needle[0], &haystack[cur..]) {
        cur += i;
        if cur + nlen > haystack.len() {
            break;
        }
        if &haystack[cur..cur+nlen] == needle {
            return Some(cur);
        }
        cur += 1;
    }
    None
}

pub fn find_any(needles: &[String], haystack: &[u8]) -> Option<usize> {
    // TODO: Use Rabin Karp?
    // I don't think there's a good way to use memchr here because it could
    // potentially scan the whole input. Maybe it's so fast that that's OK...
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

/// A safe interface to `memchr`.
///
/// memchr reduces to super-optimized machine code at around 24x the speed
/// of `haystack.iter().position(|&b| b == needle)`.
pub fn memchr(needle: u8, haystack: &[u8]) -> Option<usize> {
    use libc::funcs::c95::string;
    use libc::types::common::c95::c_void;
    use libc::types::os::arch::c95::{c_int, size_t};

    let p = unsafe {
        string::memchr(
            haystack.as_ptr() as *const c_void,
            needle as c_int,
            haystack.len() as size_t)
    };
    if p.is_null() {
        None
    } else {
        Some((p as isize - (haystack.as_ptr() as isize)) as usize)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn find_any() {
        let needles = &[
            "abaa".into(), "abbaa".into(), "abbbaa".into(), "abbbbaa".into(),
        ];
        let haystack = b"ababbabbbabbbabbbbabbbbaa";
        assert_eq!(super::find_any(needles, haystack), Some(18));
    }

    #[test]
    fn find_one_match() {
        let needle = b"abc";
        let haystack = b"zzzzzzzzzzabc";
        assert_eq!(super::find_one(needle, haystack), Some(10));
    }

    #[test]
    fn find_one_no_match() {
        let needle = b"abcz";
        let haystack = b"zzzzzzzzzzabc";
        assert_eq!(super::find_one(needle, haystack), None);
    }

    #[test]
    fn find_one_byte() {
        let needle = b"a";
        let haystack = b"zzzzzzzzzza";
        assert_eq!(super::find_one(needle, haystack), Some(10));
    }

    #[test]
    fn find_one_byte_no_match() {
        let needle = b"y";
        let haystack = b"zzzzzzzzzzabc";
        assert_eq!(super::find_one(needle, haystack), None);
    }
}
