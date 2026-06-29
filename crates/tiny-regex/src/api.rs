#![forbid(unsafe_code)]

use core::ffi::CStr;

use crate::raw;

pub use crate::raw::{RegexBuf, DEFAULT_CCL, DEFAULT_MATCH_TEXT_LEN, DEFAULT_MEMO, DEFAULT_NODES};

/// `Regex` with the default node and character-class buffer sizes (32 nodes, 64-byte CCL,
/// 256-byte memo table).
///
/// This is the common-case type. For non-default sizes use [`RegexBuf<N, CCL, MEMO>`][RegexBuf] directly.
pub type Regex = raw::RegexBuf<DEFAULT_NODES, DEFAULT_CCL, DEFAULT_MEMO>;

/// `Regex` with memoisation disabled — identical to the original tiny-regex-c behaviour.
///
/// Compiles and matches the same patterns as [`Regex`] but allocates no memo table.
pub type TinyRegex = raw::RegexBuf<DEFAULT_NODES, DEFAULT_CCL, 0>;

/// A single match within a haystack.
///
/// Returned by [`RegexBuf::find_at`] and yielded by [`Matches`]. The byte
/// offsets [`start`][Match::start] and [`end`][Match::end] index into the
/// original haystack passed to the search call.
pub struct Match<'a> {
    start: usize,
    end: usize,
    _marker: core::marker::PhantomData<&'a CStr>,
}

impl<'a> Match<'a> {
    /// Byte offset of the first character of the match within the haystack.
    pub fn start(&self) -> usize {
        self.start
    }

    /// Byte offset one past the last character of the match within the haystack.
    ///
    /// For a zero-length match (e.g. `$` at end of string) `end == start`.
    pub fn end(&self) -> usize {
        self.end
    }
}

/// An iterator over all non-overlapping matches of a pattern in a haystack.
///
/// Created by [`RegexBuf::find_iter`]. Yields [`Match`] values in left-to-right order.
pub struct Matches<'a, 'b, const N: usize, const CCL: usize, const MEMO: usize> {
    pattern: &'a raw::RegexBuf<N, CCL, MEMO>,
    haystack: &'b CStr,
    offset: usize,
}

impl<'a, 'b, const N: usize, const CCL: usize, const MEMO: usize>
    Iterator for Matches<'a, 'b, N, CCL, MEMO>
{
    type Item = Match<'b>;

    fn next(&mut self) -> Option<Match<'b>> {
        if self.offset > self.haystack.to_bytes().len() {
            return None;
        }
        let m = self.pattern.find_at(self.haystack, self.offset)?;
        self.offset = if m.end > m.start { m.end } else { m.start + 1 };
        Some(m)
    }
}

impl<const N: usize, const CCL: usize, const MEMO: usize> raw::RegexBuf<N, CCL, MEMO> {
    /// Search `haystack` for the first match of this pattern at or after byte
    /// offset `start`.
    ///
    /// Returns `None` if no match is found. Each call stack-allocates a
    /// zero-initialised memo table to cache failed `(pattern_node, text_offset)`
    /// states and eliminate exponential backtracking.
    pub fn find_at<'a>(&self, haystack: &'a CStr, start: usize) -> Option<Match<'a>> {
        let mut matchlen: i32 = 0;
        let idx = self.matchp_at(haystack, start, &mut matchlen);
        if idx < 0 {
            return None;
        }
        Some(Match {
            start: idx as usize,
            end: idx as usize + matchlen as usize,
            _marker: core::marker::PhantomData,
        })
    }

    /// Return an iterator over all non-overlapping matches in `haystack`.
    pub fn find_iter<'a, 'b>(&'a self, haystack: &'b CStr) -> Matches<'a, 'b, N, CCL, MEMO> {
        Matches { pattern: self, haystack, offset: 0 }
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    extern crate std;
    use super::*;
    use alloc::vec::Vec;

    #[test]
    fn basic_match() {
        let re = Regex::new(c"foo").unwrap();
        let m = re.find_at(c"foo bar", 0).unwrap();
        assert_eq!(m.start(), 0);
        assert_eq!(m.end(), 3);
    }

    #[test]
    fn find_at_offset() {
        let re = Regex::new(c"foo").unwrap();
        let m = re.find_at(c"foo bar foo", 1).unwrap();
        assert_eq!(m.start(), 8);
        assert_eq!(m.end(), 11);
    }

    #[test]
    fn find_at_past_end_returns_none() {
        let re = Regex::new(c"foo").unwrap();
        assert!(re.find_at(c"foo", 4).is_none());
    }

    #[test]
    fn find_iter_all_matches() {
        let re = Regex::new(c"foo").unwrap();
        let matches: Vec<_> = re.find_iter(c"foo bar foo").collect();
        assert_eq!(matches.len(), 2);
        assert_eq!((matches[0].start(), matches[0].end()), (0, 3));
        assert_eq!((matches[1].start(), matches[1].end()), (8, 11));
    }

    #[test]
    fn find_iter_empty_haystack() {
        let re = Regex::new(c"foo").unwrap();
        assert_eq!(re.find_iter(c"").count(), 0);
    }

    #[test]
    fn new_invalid_pattern_returns_none() {
        assert!(Regex::new(c"[abc").is_none());
    }

    #[test]
    fn char_class_match() {
        let re = Regex::new(c"[abc]+").unwrap();
        let m = re.find_at(c"xyzabcxyz", 0).unwrap();
        assert_eq!(m.start(), 3);
        assert_eq!(m.end(), 6);
    }

    // Char-class nodes store a byte *offset* into the ccl array, not a pointer.
    // Moving RegexBuf must not invalidate those offsets.
    #[test]
    fn move_preserves_char_class_match() {
        fn compile() -> Regex { Regex::new(c"[a-z]+").unwrap() }
        let re = compile(); // moved off compile()'s stack frame
        let m = re.find_at(c"123abc", 0).unwrap();
        assert_eq!((m.start(), m.end()), (3, 6));
    }

    #[test]
    fn char_class_no_match() {
        let re = Regex::new(c"[abc]").unwrap();
        assert!(re.find_at(c"xyz", 0).is_none());
    }

    #[test]
    fn char_range_match() {
        let re = Regex::new(c"[a-z]+").unwrap();
        let matches: Vec<_> = re.find_iter(c"123abc 456def").collect();
        assert_eq!(matches.len(), 2);
        assert_eq!((matches[0].start(), matches[0].end()), (3, 6));
        assert_eq!((matches[1].start(), matches[1].end()), (10, 13));
    }

    #[test]
    fn recompile_updates_pattern() {
        let re = Regex::new(c"[abc]+").unwrap();
        assert!(re.find_at(c"a", 0).is_some());
        let re = re.recompile(c"[xyz]+").unwrap();
        assert!(re.find_at(c"a", 0).is_none());
        assert!(re.find_at(c"x", 0).is_some());
    }

    #[test]
    fn recompile_invalid_pattern_returns_none() {
        let re = Regex::new(c"foo").unwrap();
        assert!(re.recompile(c"\\").is_none());
    }

    #[test]
    fn new_empty_pattern_succeeds() {
        assert!(Regex::new(c"").is_some());
    }

    #[test]
    fn recompile_empty_pattern_succeeds() {
        let re = Regex::new(c"foo").unwrap();
        assert!(re.recompile(c"").is_some());
    }

    #[test]
    fn concurrent_compile_then_match() {
        use std::thread;

        let handles: Vec<_> = (0..8)
            .map(|i| {
                thread::spawn(move || {
                    let pattern = if i % 2 == 0 { c"[a-z]+" } else { c"[0-9]+" };
                    let re = Regex::new(pattern).unwrap();
                    if i % 2 == 0 {
                        assert_eq!(re.find_iter(c"abc 123 def").count(), 2);
                    } else {
                        assert_eq!(re.find_iter(c"abc 123 def").count(), 1);
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn concurrent_match_shared_regex() {
        use std::sync::Arc;
        use std::thread;

        let re = Arc::new(Regex::new(c"[a-z]+").unwrap());

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let re = Arc::clone(&re);
                thread::spawn(move || {
                    assert_eq!(re.find_iter(c"abc 123 def").count(), 2);
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn new_trailing_backslash_returns_none() {
        assert!(Regex::new(c"\\").is_none());
    }

    #[test]
    fn compile_long_pattern_returns_none() {
        // Default Regex (N=32): a pattern of 32 symbols needs 33 nodes
        // (32 data + UNUSED sentinel) but capacity is 32 — must fail.
        let pattern = alloc::ffi::CString::new("a".repeat(DEFAULT_NODES)).unwrap();
        assert!(Regex::new(pattern.as_c_str()).is_none());
    }

    // Finding A: a (N-1)-symbol pattern fills all nodes and puts UNUSED at the
    // last slot of the re_compiled array.  matchpattern used to form
    // &pattern[2] from that slot — UB in C11.  The fix returns 1 directly for
    // UNUSED without forming &pattern[2].
    #[test]
    fn max_length_pattern_matches() {
        let pattern = alloc::ffi::CString::new("a".repeat(DEFAULT_NODES - 1)).unwrap();
        let haystack = alloc::ffi::CString::new("a".repeat(DEFAULT_NODES - 1)).unwrap();
        let re = Regex::new(pattern.as_c_str()).unwrap();
        let m = re.find_at(haystack.as_c_str(), 0).unwrap();
        assert_eq!(m.start(), 0);
        assert_eq!(m.end(), DEFAULT_NODES - 1);
    }

    // Finding B: re_matchp's `if (utext[0] == '\0') return -1` guard discards any
    // match found at the null-terminator position, including the only position where
    // a standalone $ anchor can match.  Fix: remove the guard; a zero-length match
    // at the end of string is correct for $ (and for patterns like a? or a*).
    #[test]
    fn dollar_anchor_alone_matches_end_of_string() {
        let re = Regex::new(c"$").unwrap();
        let m = re.find_at(c"foo", 0).unwrap();
        assert_eq!(m.start(), 3);
        assert_eq!(m.end(), 3);
    }

    #[test]
    fn dollar_anchor_with_prefix_still_works() {
        let re = Regex::new(c"foo$").unwrap();
        let m = re.find_at(c"foo", 0).unwrap();
        assert_eq!(m.start(), 0);
        assert_eq!(m.end(), 3);
        assert!(re.find_at(c"foo bar", 0).is_none());
    }

    #[test]
    fn caret_anchor_matches_only_at_start() {
        let re = Regex::new(c"^foo").unwrap();
        let m = re.find_at(c"foobar", 0).unwrap();
        assert_eq!((m.start(), m.end()), (0, 3));
        assert!(re.find_at(c"barfoo", 0).is_none());
    }

    #[test]
    fn dot_does_not_match_newline_by_default() {
        let haystack = std::ffi::CString::new("a\nb").unwrap();
        let re = Regex::new(c"a.b").unwrap();
        assert!(re.find_at(haystack.as_c_str(), 0).is_none());
    }

    #[test]
    fn dot_matches_any_char() {
        let re = Regex::new(c"f.o").unwrap();
        assert!(re.find_at(c"foo", 0).is_some());
        assert!(re.find_at(c"fzo", 0).is_some());
        assert!(re.find_at(c"fo", 0).is_none());
    }

    #[test]
    fn star_matches_zero_or_more() {
        let re = Regex::new(c"a*b").unwrap();
        let m = re.find_at(c"b", 0).unwrap();
        assert_eq!((m.start(), m.end()), (0, 1));
        let m = re.find_at(c"aaab", 0).unwrap();
        assert_eq!((m.start(), m.end()), (0, 4));
        assert!(re.find_at(c"aaa", 0).is_none());
    }

    #[test]
    fn plus_no_match_exercises_backtrack_failure() {
        let re = Regex::new(c"a+b").unwrap();
        assert!(re.find_at(c"aaa", 0).is_none());
    }

    #[test]
    fn question_matches_zero_or_one() {
        let re = Regex::new(c"colou?r").unwrap();
        assert!(re.find_at(c"color", 0).is_some());
        assert!(re.find_at(c"colour", 0).is_some());
        assert!(re.find_at(c"colouur", 0).is_none());
    }

    #[test]
    fn metachar_digit_matches() {
        let re = Regex::new(c"\\d+").unwrap();
        let m = re.find_at(c"abc123def", 0).unwrap();
        assert_eq!((m.start(), m.end()), (3, 6));
        assert!(re.find_at(c"abc", 0).is_none());
    }

    #[test]
    fn metachar_non_digit_matches() {
        let re = Regex::new(c"\\D+").unwrap();
        let m = re.find_at(c"123abc", 0).unwrap();
        assert_eq!((m.start(), m.end()), (3, 6));
    }

    #[test]
    fn metachar_word_matches_including_underscore() {
        let re = Regex::new(c"\\w+").unwrap();
        let m = re.find_at(c"...abc_123", 0).unwrap();
        assert_eq!((m.start(), m.end()), (3, 10));
    }

    #[test]
    fn metachar_non_word_matches() {
        let re = Regex::new(c"\\W+").unwrap();
        let m = re.find_at(c"abc...", 0).unwrap();
        assert_eq!((m.start(), m.end()), (3, 6));
    }

    #[test]
    fn metachar_whitespace_matches() {
        let re = Regex::new(c"\\s+").unwrap();
        let m = re.find_at(c"abc   def", 0).unwrap();
        assert_eq!((m.start(), m.end()), (3, 6));
    }

    #[test]
    fn metachar_non_whitespace_matches() {
        let re = Regex::new(c"\\S+").unwrap();
        let m = re.find_at(c"   abc", 0).unwrap();
        assert_eq!((m.start(), m.end()), (3, 6));
    }

    #[test]
    fn escaped_literal_matches_exact_char() {
        let re = Regex::new(c"\\.").unwrap();
        let m = re.find_at(c"abc.def", 0).unwrap();
        assert_eq!((m.start(), m.end()), (3, 4));
        assert!(re.find_at(c"abc", 0).is_none());
    }

    #[test]
    fn inverted_char_class_matches_complement() {
        let re = Regex::new(c"[^abc]+").unwrap();
        let m = re.find_at(c"abcxyz", 0).unwrap();
        assert_eq!((m.start(), m.end()), (3, 6));
        assert!(re.find_at(c"aaa", 0).is_none());
    }

    #[test]
    fn metachar_inside_char_class_matches() {
        let re = Regex::new(c"[\\d]+").unwrap();
        let m = re.find_at(c"abc123", 0).unwrap();
        assert_eq!((m.start(), m.end()), (3, 6));
        assert!(re.find_at(c"abc", 0).is_none());
    }

    #[test]
    fn char_class_post_loop_overflow_returns_none() {
        // ccl_bufidx starts at 1; CCL-1 chars fill indices 1..CCL-1, leaving
        // ccl_bufidx == CCL at loop exit — triggers the post-loop overflow check.
        let pattern = alloc::ffi::CString::new(alloc::format!("[{}]", "a".repeat(DEFAULT_CCL - 1))).unwrap();
        assert!(Regex::new(pattern.as_c_str()).is_none());
    }

    #[test]
    fn char_class_in_loop_overflow_returns_none() {
        // At the CCL-th char, ccl_bufidx is already CCL going into the loop
        // body — triggers the in-loop overflow check (ccl_bufidx >= ccl_size).
        let pattern = alloc::ffi::CString::new(alloc::format!("[{}]", "a".repeat(DEFAULT_CCL))).unwrap();
        assert!(Regex::new(pattern.as_c_str()).is_none());
    }

    #[test]
    fn char_class_backslash_overflow_returns_none() {
        // CCL-2 normal chars bring ccl_bufidx to CCL-1; the next '\\'
        // checks ccl_bufidx + 1 >= ccl_size — triggers the backslash-specific
        // overflow guard that reserves space for two bytes.
        let pattern = alloc::ffi::CString::new(alloc::format!("[{}\\d]", "a".repeat(DEFAULT_CCL - 2))).unwrap();
        assert!(Regex::new(pattern.as_c_str()).is_none());
    }

    #[test]
    fn inverted_char_class_incomplete_returns_none() {
        assert!(Regex::new(c"[^").is_none());
    }

    #[test]
    fn char_class_backslash_at_end_returns_none() {
        assert!(Regex::new(c"[a\\").is_none());
    }

    #[test]
    fn metachar_non_digit_in_char_class_matches() {
        let re = Regex::new(c"[\\D]+").unwrap();
        let m = re.find_at(c"123abc", 0).unwrap();
        assert_eq!((m.start(), m.end()), (3, 6));
    }

    #[test]
    fn metachar_word_in_char_class_matches() {
        let re = Regex::new(c"[\\w]+").unwrap();
        let m = re.find_at(c"...abc_1", 0).unwrap();
        assert_eq!((m.start(), m.end()), (3, 8));
    }

    #[test]
    fn metachar_non_word_in_char_class_matches() {
        let re = Regex::new(c"[\\W]+").unwrap();
        let m = re.find_at(c"abc...", 0).unwrap();
        assert_eq!((m.start(), m.end()), (3, 6));
    }

    #[test]
    fn metachar_whitespace_in_char_class_matches() {
        let re = Regex::new(c"[\\s]+").unwrap();
        let m = re.find_at(c"abc   ", 0).unwrap();
        assert_eq!((m.start(), m.end()), (3, 6));
    }

    #[test]
    fn metachar_non_whitespace_in_char_class_matches() {
        let re = Regex::new(c"[\\S]+").unwrap();
        let m = re.find_at(c"   abc", 0).unwrap();
        assert_eq!((m.start(), m.end()), (3, 6));
    }

    #[test]
    fn escaped_literal_in_char_class_matches() {
        let re = Regex::new(c"[\\.]+").unwrap();
        let m = re.find_at(c"abc...", 0).unwrap();
        assert_eq!((m.start(), m.end()), (3, 6));
        assert!(re.find_at(c"abc", 0).is_none());
    }

    #[test]
    fn find_iter_zero_length_match_at_end_covers_offset_guard() {
        let re = Regex::new(c"$").unwrap();
        let matches: Vec<_> = re.find_iter(c"foo").collect();
        assert_eq!(matches.len(), 1);
        assert_eq!((matches[0].start(), matches[0].end()), (3, 3));
    }

    #[test]
    fn char_class_trailing_dash_covers_matchrange_empty_str2() {
        let re = Regex::new(c"[a-]+").unwrap();
        let m = re.find_at(c"!a-!", 0).unwrap();
        assert_eq!((m.start(), m.end()), (1, 3));
    }

    #[test]
    fn dash_does_not_match_char_range() {
        let re = Regex::new(c"[a-z]+").unwrap();
        assert!(re.find_at(c"-", 0).is_none());
    }

    #[test]
    fn question_no_match_covers_matchquestion_branches() {
        let re = Regex::new(c"x?y").unwrap();
        assert!(re.find_at(c"z", 0).is_none());
    }

    #[test]
    fn end_anchor_mid_pattern_covers_end_non_unused_branch() {
        let re = Regex::new(c"$a").unwrap();
        assert!(re.find_at(c"a", 0).is_none());
    }

    #[test]
    fn memo_bounds_pathological_backtrack() {
        let re = Regex::new(c"a*a*b").unwrap();
        let haystack = alloc::ffi::CString::new("a".repeat(32)).unwrap();
        assert!(re.find_at(haystack.as_c_str(), 0).is_none());
    }

    // Memo tests ---------------------------------------------------------------

    #[test]
    fn per_attempt_memo_reset_finds_match_past_long_prefix() {
        let re = Regex::new(c"a*a*b").unwrap();
        let haystack = alloc::ffi::CString::new("a".repeat(200) + "cb").unwrap();
        let m = re.find_at(haystack.as_c_str(), 0).unwrap();
        assert_eq!(m.start(), 201);
        assert_eq!(m.end(), 202);
    }

    #[test]
    fn bplus_c_skips_long_decoy_brun() {
        let re = Regex::new(c"b+c").unwrap();
        let haystack = alloc::ffi::CString::new(
            "a".repeat(50) + &"b".repeat(100) + &"a".repeat(50) + &"b".repeat(5) + "c" + &"a".repeat(10)
        ).unwrap();
        let m = re.find_at(haystack.as_c_str(), 0).unwrap();
        assert_eq!(m.start(), 200);
        assert_eq!(m.end(), 206);
    }

    #[test]
    fn memo_finds_match_on_pathological_haystack() {
        let haystack = alloc::ffi::CString::new("a".repeat(32) + "cb").unwrap();
        let re = Regex::new(c"a*a*b").unwrap();
        let m = re.find_at(haystack.as_c_str(), 0).unwrap();
        assert_eq!(m.start(), 33);
        assert_eq!(m.end(), 34);
    }

    #[test]
    fn memo_correctly_returns_none_when_no_match_exists() {
        let haystack = alloc::ffi::CString::new("a".repeat(32)).unwrap();
        let re = Regex::new(c"a*a*b").unwrap();
        assert!(re.find_at(haystack.as_c_str(), 0).is_none());
    }

    // Exercises various RegexBuf<N, CCL, MEMO> parameter combinations.
    //
    // Each row calls check::<N, CCL, MEMO>(pattern, haystack, expected match).
    // The memo_stride passed to C is MEMO*8/N; we verify correctness across:
    //   - minimum and odd N values
    //   - MEMO=0 (no memo), tiny MEMO (stride=1), non-power-of-2 MEMO
    //   - char classes (exercises CCL)
    //   - backtracking-heavy patterns (stress the memo)
    #[test]
    fn regexbuf_parameter_variants() {
        fn check<const N: usize, const CCL: usize, const MEMO: usize>(
            pattern: &CStr,
            haystack: &CStr,
            expected: Option<(usize, usize)>,
        ) {
            let re = RegexBuf::<N, CCL, MEMO>::new(pattern).unwrap_or_else(|| {
                panic!("compile failed: {pattern:?}  N={N} CCL={CCL} MEMO={MEMO}")
            });
            let got = re.find_at(haystack, 0).map(|m| (m.start(), m.end()));
            assert_eq!(
                got, expected,
                "N={N} CCL={CCL} MEMO={MEMO}  pattern={pattern:?}  haystack={haystack:?}"
            );
        }

        // --- N=2: absolute minimum (1 data node + UNUSED sentinel) ---
        // pattern "a" → [CHAR('a'), UNUSED]
        check::<2, 2, 0>(c"a", c"a",   Some((0, 1)));
        check::<2, 2, 0>(c"a", c"b",   None);
        check::<2, 2, 2>(c"a", c"xax", Some((1, 2)));  // stride = 2*8/2 = 8

        // --- N=3: anchors, single quantifier ---
        // "^a" → [BEGIN, CHAR('a'), UNUSED]
        check::<3, 2, 0>(c"^a", c"ax", Some((0, 1)));
        check::<3, 2, 0>(c"^a", c"ba", None);
        // "a+" → [CHAR('a'), PLUS, UNUSED]
        check::<3, 2, 0>(c"a+", c"aaa", Some((0, 3)));
        check::<3, 2, 3>(c"a+", c"bbb", None);          // stride = 3*8/3 = 8

        // --- N=4: two-operand quantifier (CHAR STAR CHAR UNUSED) ---
        // "a*b" — exercises memo on the STAR backtrack path
        check::<4, 2, 0>(c"a*b", c"b",    Some((0, 1)));
        check::<4, 2, 0>(c"a*b", c"aaab", Some((0, 4)));
        check::<4, 2, 0>(c"a*b", c"aaa",  None);
        check::<4, 2, 4>(c"a*b", c"b",    Some((0, 1)));  // stride = 4*8/4 = 8
        check::<4, 2, 4>(c"a*b", c"aaab", Some((0, 4)));
        check::<4, 2, 4>(c"a*b", c"aaa",  None);

        // --- N=5 (odd): "a+b+" → [CHAR(a), PLUS, CHAR(b), PLUS, UNUSED] ---
        check::<5, 2, 0>(c"a+b+", c"ab",   Some((0, 2)));
        check::<5, 2, 5>(c"a+b+", c"aabb", Some((0, 4)));  // stride = 5*8/5 = 8
        check::<5, 2, 5>(c"a+b+", c"aa",   None);

        // --- N=6: "a*b+c" → [CHAR(a), STAR, CHAR(b), PLUS, CHAR(c), UNUSED] ---
        check::<6, 2, 0>(c"a*b+c", c"bc",   Some((0, 2)));
        check::<6, 2, 6>(c"a*b+c", c"abbc", Some((0, 4)));  // stride = 6*8/6 = 8
        check::<6, 2, 6>(c"a*b+c", c"ac",   None);

        // --- N=7 (odd): pathological backtracker "a*a*b" (6 nodes, fits N=7) ---
        check::<7, 2, 0>(c"a*a*b", c"b",    Some((0, 1)));
        check::<7, 2, 7>(c"a*a*b", c"aaab", Some((0, 4)));  // stride = 7*8/7 = 8
        check::<7, 2, 7>(c"a*a*b", c"aaa",  None);

        // --- Non-divisible MEMO: N=3, MEMO=2 → stride = 2*8/3 = 5 ---
        // stride rounds down; memo covers fewer offsets but must stay correct
        check::<3, 2, 2>(c"a+", c"aaa", Some((0, 3)));
        check::<3, 2, 2>(c"a+", c"bbb", None);

        // --- Tiny memo stride: N=8, MEMO=1 → stride = 1*8/8 = 1 ---
        // Only text offset 0 per node is memoised; deeper offsets fall through
        check::<8, 2, 1>(c"a*b", c"b",    Some((0, 1)));
        check::<8, 2, 1>(c"a*b", c"aaab", Some((0, 4)));
        check::<8, 2, 1>(c"a*b", c"aaa",  None);

        // --- Larger stride: N=8, MEMO=64 → stride = 64*8/8 = 64 ---
        check::<8, 2, 64>(c"a+b+", c"aabb",  Some((0, 4)));
        check::<8, 2, 64>(c"a+b+", c"bbbbb", None);

        // --- Character classes ---
        // "[a-z]+" → [CHAR_CLASS, PLUS, UNUSED] (3 nodes); needs CCL >= 5
        check::<3, 8, 0>(c"[a-z]+", c"hello", Some((0, 5)));
        check::<3, 8, 3>(c"[a-z]+", c"123",   None);          // stride = 8
        check::<4, 8, 4>(c"[0-9]+", c"abc42", Some((3, 5)));  // stride = 8
        // Inverted class
        check::<3, 8, 0>(c"[^abc]+", c"abcxyz", Some((3, 6)));
        check::<3, 8, 0>(c"[^abc]+", c"aaa",    None);

        // SmallRe (bench type): N=8, CCL=16, MEMO=64 → stride = 64
        check::<8, 16, 64>(c"[a-z]+", c"hello world", Some((0, 5)));
        check::<8, 16, 64>(c"[a-z]+", c"123",         None);
        check::<8, 16,  0>(c"[a-z]+", c"hello",       Some((0, 5)));  // no memo

        // Default Regex sizes
        check::<32, 64, 256>(c"[a-z]+[0-9]*", c"abc123", Some((0, 6)));
        check::<32, 64, 256>(c"[a-z]+[0-9]*", c"abc",    Some((0, 3)));
        check::<32, 64, 256>(c"[a-z]+[0-9]*", c"123",    None);
    }

    #[test]
    fn memo_is_fresh_each_call() {
        let h1 = alloc::ffi::CString::new("a".repeat(32) + "cb").unwrap();
        let h2 = alloc::ffi::CString::new("xb").unwrap();
        let re = Regex::new(c"a*a*b").unwrap();
        let m1 = re.find_at(h1.as_c_str(), 0).unwrap();
        assert_eq!(m1.start(), 33);
        let m2 = re.find_at(h2.as_c_str(), 0).unwrap();
        assert_eq!(m2.start(), 1);
    }

    #[test]
    fn non_ascii_literal_byte_matches() {
        let re = Regex::new(c"é").unwrap();
        let m = re.find_at(c"caf\xc3\xa9", 0).unwrap();
        assert_eq!(m.start(), 3);
        assert_eq!(m.end(), 5);
    }

    #[test]
    fn question_is_greedy() {
        let re = Regex::new(c"a?").unwrap();
        let m = re.find_at(c"a", 0).unwrap();
        assert_eq!((m.start(), m.end()), (0, 1));
    }

    #[test]
    fn matchlength_not_leaked_across_failed_attempts() {
        let re = Regex::new(c".0+").unwrap();
        let m = re.find_at(c"aa0", 0).unwrap();
        assert_eq!(m.start(), 1);
        assert_eq!(m.end(), 3);
    }

    #[test]
    fn tiny_regex_no_memo_matches_correctly() {
        let re = TinyRegex::new(c"[0-9]+").unwrap();
        let m = re.find_at(c"foo42bar", 0).unwrap();
        assert_eq!((m.start(), m.end()), (3, 5));
        assert!(re.find_at(c"abc", 0).is_none());
    }
}
