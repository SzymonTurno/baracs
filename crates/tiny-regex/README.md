# tiny-regex

[![crates.io](https://img.shields.io/crates/v/tiny-regex.svg)](https://crates.io/crates/tiny-regex)
[![docs.rs](https://docs.rs/tiny-regex/badge.svg)](https://docs.rs/tiny-regex)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](../../LICENSE)

A `no_std`, no `alloc` regex engine for embedded systems, wrapping
[tiny-regex-c](https://github.com/kokke/tiny-regex-c).

## Installation

```toml
[dependencies]
tiny-regex = "0.1"
```

Requires Rust 1.77 or later. `no_std`, no `alloc` — all storage lives on the stack.

## Quick start

```rust
use tiny_regex::Regex;

let re = Regex::new(c"[0-9]+").expect("valid pattern");

let m = re.find_at(c"foo 42 bar", 0).unwrap();
assert_eq!(m.start(), 4);
assert_eq!(m.end(), 6);
```

`Regex` is a type alias for `RegexBuf<32, 64, 256>` — the common case with
default capacity. Call `recompile()` to update the pattern in-place.
`Regex` is `Send + Sync` and can be shared across threads for concurrent matching.

## Custom capacity

For patterns that need more nodes or a larger character-class buffer, use
[`RegexBuf<N, CCL, MEMO>`](crate::RegexBuf) directly. All storage lives on
the stack; there is no heap allocation.

## Disabling memoisation

`TinyRegex` is identical to `Regex` but allocates no memo table, matching
the original tiny-regex-c behaviour:

```rust
use tiny_regex::TinyRegex;

let re = TinyRegex::new(c"[0-9]+").expect("valid pattern");
```

## Supported syntax

| Pattern  | Meaning                                                      |
|----------|--------------------------------------------------------------|
| `.`      | Any character except `\n` (configurable, see Configuration) |
| `^`      | Start of string                                              |
| `$`      | End of string                                                |
| `*`      | Zero or more of the preceding item                           |
| `+`      | One or more of the preceding item                            |
| `?`      | Zero or one of the preceding item                            |
| `[abc]`  | Character class                                              |
| `[^abc]` | Negated character class                                      |
| `[a-z]`  | Character range                                              |
| `\d`     | Digit (`[0-9]`)                                              |
| `\D`     | Non-digit                                                    |
| `\w`     | Word character (`[a-zA-Z0-9_]`)                              |
| `\W`     | Non-word character                                           |
| `\s`     | Whitespace                                                   |
| `\S`     | Non-whitespace                                               |

## Limitations

- **No alternation or grouping** — `|` and `()` are not supported.
- **No backreferences or captures** — matches return byte offsets only.
- **Byte-oriented, not Unicode-aware** — text is treated as raw bytes.
  `\w`, `\d`, `\s` and their complements operate on individual bytes, so
  multibyte characters only match if written out literally in the pattern.
- **Pattern complexity is bounded** by the node capacity `N` (default 32) —
  patterns that require more nodes than the limit fail to compile.
- **Backtracking engine** — uses a recursive backtracking NFA rather than a
  compiled automaton; certain patterns with quantifiers (`*`, `+`) may explore
  many paths before failing on a non-matching input (see Performance).

## Internals

The matching core is [tiny-regex-c](https://github.com/kokke/tiny-regex-c)'s
backtracking NFA. When a pattern with quantifiers (`*`, `+`) fails to match,
a backtracking engine may revisit the same text position many times following
different paths. The memo table bounds this: each `find_at` call
stack-allocates `MEMO` bytes (256 at defaults) and records which
`(pattern node, text offset)` pairs have already been proven to fail —
so they are never retried.

`regex_t` nodes store character-class references as byte *offsets* into the
CCL buffer rather than raw pointers, so `RegexBuf` is freely movable and
`Send + Sync` without unsafe code.

### Binary size

Release build, x86-64, GCC 13.3 / rustc 1.96 (ARM will differ):

```text
$ cargo build --release --package tiny-regex
$ size target/release/libtiny_regex.rlib | sed -E 's/ \(ex [^)]*\)//; s/[0-9a-f]{16,}-//g; s/-[0-9a-f]{16,}//g'
   text    data     bss     dec     hex filename
      0       0       0       0       0  lib.rmeta
      0       0       0       0       0  tiny_regex.tiny_regex.cgu.0.rcgu.o
   2541       0       0    2541     9ed  re.o
```

The Rust wrapper
contributes 0 bytes — const-generic code is instantiated at the call site. The
C matching core is ~2.5 KB of code. BSS is zero because all storage is
stack-allocated; a `find_at` call uses `MEMO` bytes of stack for the memo table
(256 at defaults) plus the `RegexBuf` itself if held on the stack.

### Throughput

Compared to `regex-lite`, tiny-regex trades feature breadth and throughput for
zero `std` dependency and a small code footprint — making it suitable for
targets where `std` is unavailable. `benches/vs_regex_lite.rs` benchmarks the
two side by side if you want to measure the tradeoff on your own hardware:

```sh
cargo bench --bench vs_regex_lite
```

## Configuration

Node capacity (`N`), character-class buffer size (`CCL`), and memo table
size (`MEMO`) are type-level parameters on [`RegexBuf<N, CCL, MEMO>`](crate::RegexBuf) —
set them at the call site, or use the [`Regex`](crate::Regex) alias for the defaults.

## License

Apache-2.0 — see [LICENSE](../../LICENSE).
