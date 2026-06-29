#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

use core::ffi::CStr;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

/// Default node capacity: max pattern symbols in a compiled regex.
pub const DEFAULT_NODES: usize = 32;
/// Default character-class buffer size in bytes.
pub const DEFAULT_CCL: usize = 64;
/// Maximum text offset tracked per node in the memo table.
pub const DEFAULT_MATCH_TEXT_LEN: usize = 64;
/// Memo table size (bytes) for the default node and text-length capacities.
///
/// Computed as `(DEFAULT_NODES × DEFAULT_MATCH_TEXT_LEN + 7) / 8`.
pub const DEFAULT_MEMO: usize =
    (DEFAULT_NODES * DEFAULT_MATCH_TEXT_LEN + 7) / 8;


/// An owned, compiled regex pattern with compile-time-fixed storage.
///
/// - `N` — node capacity: max compiled pattern symbols.
/// - `CCL` — character-class buffer size in bytes.
/// - `MEMO` — memo table size in bytes; use `(N * DEFAULT_MATCH_TEXT_LEN + 7) / 8`.
///
/// For the common case use [`Regex`][crate::Regex] (a type alias for
/// `RegexBuf<32, 64, 256>`) so you never spell out the parameters.
/// Use `RegexBuf` directly only when you need non-default sizes.
///
/// Created via [`RegexBuf::new`]; the pattern can be updated in-place via
/// [`RegexBuf::recompile`]. Use [`find_at`][RegexBuf::find_at] and
/// [`find_iter`][RegexBuf::find_iter] to search.
///
/// A `RegexBuf` is always valid — construction fails explicitly via `None`.
/// Each `find_at` call stack-allocates and zeros a `[u8; MEMO]` memo table
/// independently, so `RegexBuf` is `Send + Sync` without unsafe.
pub struct RegexBuf<
    const N: usize = DEFAULT_NODES,
    const CCL: usize = DEFAULT_CCL,
    const MEMO: usize = DEFAULT_MEMO,
> {
    /// Compiled pattern nodes — char-class entries carry a byte *offset*
    /// into `ccl` (not a raw pointer), so the struct is freely movable.
    re_nodes: [regex_t; N],
    /// Character-class byte buffer referenced by CHAR_CLASS/INV_CHAR_CLASS
    /// nodes via their `u.ccl` byte offset.
    ccl: [u8; CCL],
}

// regex_t.u.ccl is a size_t offset into ccl_buf, not a pointer.
// Both fields contain no raw pointers, so Send + Sync hold automatically.

impl<const N: usize, const CCL: usize, const MEMO: usize> RegexBuf<N, CCL, MEMO> {
    fn compile_into(re_nodes: &mut [regex_t; N], ccl: &mut [u8; CCL], pattern: &CStr) -> bool {
        // Zero-fill ccl so ccl_buf[0]=='\0', satisfying the sentinel required by
        // matchcharclass (reads str[-1] to detect a literal '-' at the start of
        // a character class; str is always >=1 into ccl_buf, so [-1] is ccl_buf[0]).
        ccl.fill(0);
        // SAFETY: re_nodes is our exclusively owned array; re_compile writes into
        // it up to N nodes.  ccl and pattern are valid for their lifetimes.
        let bytes_written = unsafe {
            re_compile(
                re_nodes.as_mut_ptr() as *mut regex_t,
                N,
                ccl.as_mut_ptr(),
                CCL,
                pattern.as_ptr(),
            )
        };
        bytes_written > 0
    }

    /// Compile `pattern` into a new `RegexBuf`.
    ///
    /// Returns `None` if `pattern` is syntactically invalid or exceeds the
    /// node capacity `N` or character-class buffer `CCL`.
    pub fn new(pattern: &CStr) -> Option<RegexBuf<N, CCL, MEMO>> {
        // SAFETY: regex_t is a C struct with unsigned char type + union
        // {u8, size_t}; all-zero bytes are a valid representation (type=0 is
        // UNUSED; union ccl offset 0 refers to the sentinel position in
        // ccl_buf and is safe to read as an empty class).
        let mut re_nodes: [regex_t; N] = unsafe { core::mem::zeroed() };
        let mut ccl = [0u8; CCL];
        if Self::compile_into(&mut re_nodes, &mut ccl, pattern) {
            Some(RegexBuf { re_nodes, ccl })
        } else {
            None
        }
    }

    /// Recompile this `RegexBuf` with a new `pattern`, reusing existing storage.
    ///
    /// Returns `Some` on success.  On failure returns `None` — `self` is dropped.
    pub fn recompile(mut self, pattern: &CStr) -> Option<RegexBuf<N, CCL, MEMO>> {
        if Self::compile_into(&mut self.re_nodes, &mut self.ccl, pattern) {
            Some(self)
        } else {
            None
        }
    }

    /// Match starting at byte offset `start` inside `haystack`.
    ///
    /// Stack-allocates `[u8; MEMO]` per call for an independent, zero-initialised
    /// memo table.
    ///
    /// Returns the absolute match start and writes match length into
    /// `match_length`, or -1 on no match.
    pub(crate) fn matchp_at(
        &self,
        haystack: &CStr,
        start: usize,
        match_length: &mut i32,
    ) -> i32 {
        if start > haystack.to_bytes().len() || start > i32::MAX as usize {
            *match_length = 0;
            return -1;
        }
        // SAFETY: start <= haystack.to_bytes().len() — within allocation;
        // null terminator still terminates the sub-CStr.
        let sub = unsafe { CStr::from_ptr(haystack.as_ptr().add(start)) };
        let mut len: i32 = 0;
        // MEMO is a const generic stack array; MEMO==0 gives a zero-length array.
        // C normalises memo_bytes==0 to NULL before any dereference, so passing
        // as_mut_ptr() unconditionally is safe even for the zero-length case.
        let mut memo = [0u8; MEMO];
        // memo_stride is the text-offset dimension of the memo bit array:
        // MEMO bytes hold N*memo_stride bits, so stride = MEMO*8/N.
        // When MEMO==0 memoisation is disabled; stride is unused by C.
        let memo_stride = if MEMO == 0 { 0 } else { MEMO * 8 / N };
        // SAFETY: re_nodes holds a valid compiled pattern.  ccl is the
        // char-class buffer; entries reference it by byte offset, so moving
        // RegexBuf never dangles anything.  memo is a zeroed stack array;
        // C treats memo_bytes==0 as disabled without dereferencing the pointer.
        let ret = unsafe {
            re_matchp(
                self.re_nodes.as_ptr() as re_t,
                self.ccl.as_ptr(),
                sub.as_ptr(),
                &mut len,
                memo.as_mut_ptr(),
                MEMO,
                memo_stride,
            )
        };
        *match_length = len;
        if ret < 0 { return -1; }
        ret + start as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Finding 3: matchcharclass reads str[-1] to detect a literal '-' at the start
    // of a character class.  That byte is ccl_buf[0], which must be '\0'.
    // The invariant is upheld because compile_into() zero-fills ccl before every
    // compilation, but nothing in the C code documents or asserts it.
    //
    // This test proves the dependency: after RegexBuf::new("[-abc]"), ccl[0] is 0
    // and '-' correctly matches as a literal.  Corrupting ccl[0] to any non-zero
    // byte breaks the match — demonstrating that the sentinel must be '\0'.
    #[test]
    fn matchcharclass_correctness_depends_on_ccl_sentinel_byte() {
        use core::ffi::CStr;
        // Use the default-sized alias so the test is readable.
        type R = RegexBuf;
        let mut re = R::new(c"[-abc]").unwrap();

        let mut ml = 0i32;
        assert!(re.matchp_at(CStr::from_bytes_with_nul(b"-\0").unwrap(), 0, &mut ml) >= 0,
            "dash should match as a literal when ccl[0] == 0");

        re.ccl[0] = b'x';
        let mut ml2 = 0i32;
        assert!(re.matchp_at(CStr::from_bytes_with_nul(b"-\0").unwrap(), 0, &mut ml2) < 0,
            "dash fails to match after sentinel byte is corrupted — proves str[-1] dependency");
    }
}
