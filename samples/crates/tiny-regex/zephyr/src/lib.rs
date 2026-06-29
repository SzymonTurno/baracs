// SPDX-License-Identifier: Apache-2.0

#![no_std]

use tiny_regex::{Regex, TinyRegex};

fn demo() {
    // --- Character class: matching and non-matching ---
    {
        let re = Regex::new(c"[0-9]+").unwrap();

        // "abc123def": digit run starts at byte 3, ends at 6.
        match re.find_at(c"abc123def", 0) {
            Some(m) => zephyr::printk!("digits: {}..{}\n", m.start(), m.end()),
            None    => zephyr::printk!("digits: none\n"),
        }

        // No digits — must report none.
        match re.find_at(c"abcdef", 0) {
            Some(_) => zephyr::printk!("no-digits: unexpected\n"),
            None    => zephyr::printk!("no-digits: none\n"),
        }
    }

    // --- Backtracking: TinyRegex (no memo) vs Regex (with memo) ---
    //
    // "a*a*b" against 32 a's + "cb" produces O(N²) backtrack states.
    // TinyRegex has no memo table and fails to find the match.
    // Regex memoises failed (pattern_node, text_offset) pairs, so each
    // state is visited at most once and the match at offset 33 is found.
    {
        let haystack = c"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaacb";

        match TinyRegex::new(c"a*a*b").unwrap().find_at(haystack, 0) {
            Some(_) => zephyr::printk!("pathological without memo: unexpected\n"),
            None    => zephyr::printk!("pathological without memo: none\n"),
        }

        match Regex::new(c"a*a*b").unwrap().find_at(haystack, 0) {
            Some(m) => zephyr::printk!("pathological with memo: {}..{}\n", m.start(), m.end()),
            None    => zephyr::printk!("pathological with memo: none\n"),
        }
    }
}

#[no_mangle]
extern "C" fn rust_main() {
    demo();
}
