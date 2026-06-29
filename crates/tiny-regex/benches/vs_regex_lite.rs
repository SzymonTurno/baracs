use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use tiny_regex::Regex;

// regex-lite works on &str; tiny-regex on &CStr.
// Haystacks are prepared as both types in setup — only the search is timed.

fn bench_compile(c: &mut Criterion) {
    let mut group = c.benchmark_group("vs_regex_lite/compile");

    group.bench_function("tiny-regex/literal", |b| {
        b.iter(|| Regex::new(black_box(c"foo")).unwrap())
    });
    group.bench_function("regex-lite/literal", |b| {
        b.iter(|| {
            regex_lite::Regex::new(black_box("foo")).unwrap()
        })
    });

    group.bench_function("tiny-regex/char_class", |b| {
        b.iter(|| Regex::new(black_box(c"[a-z]+")).unwrap())
    });
    group.bench_function("regex-lite/char_class", |b| {
        b.iter(|| {
            regex_lite::Regex::new(black_box("[a-z]+")).unwrap()
        })
    });

    group.bench_function("tiny-regex/metachar", |b| {
        b.iter(|| Regex::new(black_box(c"\\d+")).unwrap())
    });
    group.bench_function("regex-lite/metachar", |b| {
        b.iter(|| {
            regex_lite::Regex::new(black_box("\\d+")).unwrap()
        })
    });

    group.finish();
}

fn bench_match(c: &mut Criterion) {
    let tr_literal    = Regex::new(c"foo").unwrap();
    let tr_char_class = Regex::new(c"[a-z]+").unwrap();
    let tr_metachar   = Regex::new(c"\\d+").unwrap();

    let rl_literal    = regex_lite::Regex::new("foo").unwrap();
    let rl_char_class = regex_lite::Regex::new("[a-z]+").unwrap();
    let rl_metachar   = regex_lite::Regex::new("\\d+").unwrap();

    let mut group = c.benchmark_group("vs_regex_lite/match_hit");

    group.bench_function("tiny-regex/literal", |b| {
        b.iter(|| tr_literal.find_at(black_box(c"hello foo world"), black_box(0)))
    });
    group.bench_function("regex-lite/literal", |b| {
        b.iter(|| rl_literal.find(black_box("hello foo world")))
    });

    group.bench_function("tiny-regex/char_class", |b| {
        b.iter(|| tr_char_class.find_at(black_box(c"123abc456"), black_box(0)))
    });
    group.bench_function("regex-lite/char_class", |b| {
        b.iter(|| rl_char_class.find(black_box("123abc456")))
    });

    group.bench_function("tiny-regex/metachar", |b| {
        b.iter(|| tr_metachar.find_at(black_box(c"abc123def"), black_box(0)))
    });
    group.bench_function("regex-lite/metachar", |b| {
        b.iter(|| rl_metachar.find(black_box("abc123def")))
    });

    group.finish();
}

fn bench_backtrack(c: &mut Criterion) {
    // "a*a*b" against a long all-'a' string — both engines return no-match;
    // tiny-regex uses memo (O(nodes×N)), regex-lite uses an NFA (O(m×n)).
    let haystack_str: String = "a".repeat(32);
    let haystack_cstr = std::ffi::CString::new(haystack_str.clone()).unwrap();

    let tr_re = Regex::new(c"a*a*b").unwrap();
    let rl_re = regex_lite::Regex::new("a*a*b").unwrap();

    let mut group = c.benchmark_group("vs_regex_lite/backtrack");

    group.bench_function("tiny-regex", |b| {
        b.iter(|| tr_re.find_at(black_box(haystack_cstr.as_c_str()), black_box(0)))
    });
    group.bench_function("regex-lite", |b| {
        b.iter(|| rl_re.find(black_box(haystack_str.as_str())))
    });

    group.finish();
}

criterion_group!(benches, bench_compile, bench_match, bench_backtrack);
criterion_main!(benches);
