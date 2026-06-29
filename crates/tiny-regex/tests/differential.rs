//! Differential correctness tests: tiny-regex vs regex-lite.
//!
//! For every (pattern, haystack) pair proptest generates, both engines must
//! agree on whether a match exists and, if so, where it starts and ends.
//! Any divergence is a bug in tiny-regex; regex-lite is the reference.
//!
//! Pattern generation is restricted to tiny-regex's supported subset
//! (no alternation, no groups) so both engines should always accept the
//! same patterns. Haystacks are ASCII alphanumeric + space/tab — no \r or \n,
//! which avoids the one known semantic difference: tiny-regex's dot skips both
//! \n and \r, while regex-lite's dot only skips \n.

use proptest::prelude::*;
use std::ffi::CString;
use tiny_regex::Regex;

// ── pattern generation ────────────────────────────────────────────────────────

fn arb_literal() -> impl Strategy<Value = char> {
    prop_oneof![
        3 => (b'a'..=b'z').prop_map(|b| b as char),
        2 => (b'A'..=b'Z').prop_map(|b| b as char),
        1 => (b'0'..=b'9').prop_map(|b| b as char),
    ]
}

fn arb_meta() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(".".to_owned()),
        Just("\\d".to_owned()),
        Just("\\D".to_owned()),
        Just("\\w".to_owned()),
        Just("\\W".to_owned()),
        Just("\\s".to_owned()),
        Just("\\S".to_owned()),
    ]
}

fn arb_char_class() -> impl Strategy<Value = String> {
    (
        proptest::bool::ANY,
        proptest::collection::vec(arb_literal(), 1..=4),
    )
        .prop_map(|(negated, chars)| {
            let body: String = chars.into_iter().collect();
            if negated {
                format!("[^{body}]")
            } else {
                format!("[{body}]")
            }
        })
}

fn arb_atom() -> impl Strategy<Value = String> {
    prop_oneof![
        4 => arb_literal().prop_map(|c| c.to_string()),
        2 => arb_meta(),
        1 => arb_char_class(),
    ]
}

fn arb_element() -> impl Strategy<Value = String> {
    let quantifier = prop_oneof![
        3 => Just(""),
        1 => Just("*"),
        1 => Just("+"),
        1 => Just("?"),
    ];
    (arb_atom(), quantifier).prop_map(|(atom, q)| format!("{atom}{q}"))
}

fn arb_pattern() -> impl Strategy<Value = String> {
    (
        proptest::bool::ANY,
        proptest::collection::vec(arb_element(), 1..=8),
    )
        .prop_map(|(anchored, parts)| {
            let body = parts.join("");
            if anchored {
                format!("^{body}")
            } else {
                body
            }
        })
}

// ── haystack generation ───────────────────────────────────────────────────────

fn arb_haystack() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        prop_oneof![
            5 => (b'a'..=b'z').prop_map(|b| b as char),
            2 => (b'A'..=b'Z').prop_map(|b| b as char),
            1 => (b'0'..=b'9').prop_map(|b| b as char),
            1 => Just(' '),
            1 => Just('\t'),
        ],
        0..=500,
    )
    .prop_map(|v| v.into_iter().collect())
}

// ── differential test ─────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// tiny-regex must agree with regex-lite on every (pattern, haystack) pair
    /// in the supported subset.
    #[test]
    fn agrees_with_regex_lite(
        pattern  in arb_pattern(),
        haystack in arb_haystack(),
    ) {
        let cpattern = CString::new(pattern.as_str()).unwrap();

        // Compile with tiny-regex. Skip if the pattern exceeds the node capacity
        // of the default Regex (32 nodes) — a capacity limit, not a correctness failure.
        let tiny_re = Regex::new(&cpattern);
        prop_assume!(tiny_re.is_some());
        let tiny_re = tiny_re.as_ref().unwrap();

        // regex-lite must accept every pattern our generator produces.
        let lite_re = regex_lite::Regex::new(&pattern)
            .unwrap_or_else(|e| panic!(
                "regex-lite rejected pattern tiny-regex accepted: {pattern:?} — {e}"
            ));

        let chaystack = CString::new(haystack.as_str()).unwrap();
        let tiny_result = tiny_re.find_at(chaystack.as_c_str(), 0);
        let lite_result  = lite_re.find(&haystack);

        match (tiny_result, lite_result) {
            (Some(t), Some(l)) => {
                prop_assert_eq!(
                    t.start(), l.start(),
                    "match START differs — pattern={:?} haystack={:?}",
                    pattern, haystack
                );
                prop_assert_eq!(
                    t.end(), l.end(),
                    "match END differs — pattern={:?} haystack={:?}",
                    pattern, haystack
                );
            }
            (None, None) => {}
            (Some(t), None) => prop_assert!(
                false,
                "tiny found ({}, {}) but regex-lite found none — pattern={:?} haystack={:?}",
                t.start(), t.end(), pattern, haystack
            ),
            (None, Some(l)) => prop_assert!(
                false,
                "regex-lite found ({}, {}) but tiny found none — pattern={:?} haystack={:?}",
                l.start(), l.end(), pattern, haystack
            ),
        }
    }
}
