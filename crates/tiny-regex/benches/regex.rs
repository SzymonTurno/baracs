use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use tiny_regex::{RegexBuf, Regex};

fn bench_compile(c: &mut Criterion) {
    let mut group = c.benchmark_group("compile");

    group.bench_function("literal", |b| {
        b.iter(|| Regex::new(black_box(c"foo")).unwrap())
    });
    group.bench_function("char_class", |b| {
        b.iter(|| Regex::new(black_box(c"[a-z]+")).unwrap())
    });
    group.bench_function("metachar", |b| {
        b.iter(|| Regex::new(black_box(c"\\d+")).unwrap())
    });
    group.bench_function("anchored", |b| {
        b.iter(|| Regex::new(black_box(c"^[a-z]+$")).unwrap())
    });

    group.finish();
}

fn bench_recompile(c: &mut Criterion) {
    let mut group = c.benchmark_group("recompile");

    group.bench_function("literal", |b| {
        let re = Regex::new(c"foo").unwrap();
        b.iter_with_setup(
            || Regex::new(c"foo").unwrap(),
            |re| re.recompile(black_box(c"foo")).unwrap(),
        );
        drop(re);
    });
    group.bench_function("char_class", |b| {
        b.iter_with_setup(
            || Regex::new(c"[a-z]+").unwrap(),
            |re| re.recompile(black_box(c"[a-z]+")).unwrap(),
        );
    });
    group.bench_function("metachar", |b| {
        b.iter_with_setup(
            || Regex::new(c"\\d+").unwrap(),
            |re| re.recompile(black_box(c"\\d+")).unwrap(),
        );
    });

    group.finish();
}

fn bench_match(c: &mut Criterion) {
    let literal_re    = Regex::new(c"foo").unwrap();
    let char_class_re = Regex::new(c"[a-z]+").unwrap();
    let metachar_re   = Regex::new(c"\\d+").unwrap();

    let mut group = c.benchmark_group("match/hit");

    group.bench_function("literal", |b| {
        b.iter(|| literal_re.find_at(black_box(c"hello foo world"), black_box(0)))
    });
    group.bench_function("char_class", |b| {
        b.iter(|| char_class_re.find_at(black_box(c"123abc456"), black_box(0)))
    });
    group.bench_function("metachar", |b| {
        b.iter(|| metachar_re.find_at(black_box(c"abc123def"), black_box(0)))
    });

    group.finish();

    let mut group = c.benchmark_group("match/miss");

    group.bench_function("literal", |b| {
        b.iter(|| literal_re.find_at(black_box(c"hello bar world"), black_box(0)))
    });
    group.bench_function("char_class", |b| {
        b.iter(|| char_class_re.find_at(black_box(c"123456789"), black_box(0)))
    });
    group.bench_function("metachar", |b| {
        b.iter(|| metachar_re.find_at(black_box(c"abcdefghi"), black_box(0)))
    });

    group.finish();
}

fn bench_find_iter(c: &mut Criterion) {
    let words_re  = Regex::new(c"[a-z]+").unwrap();
    let digits_re = Regex::new(c"\\d+").unwrap();

    let mut group = c.benchmark_group("find_iter");

    group.bench_function("words_in_sentence", |b| {
        b.iter(|| {
            words_re
                .find_iter(black_box(c"the quick brown fox jumps over the lazy dog"))
                .count()
        })
    });
    group.bench_function("digits_in_log_line", |b| {
        b.iter(|| {
            digits_re
                .find_iter(black_box(c"2024-01-15 12:34:56 error code 42 on line 128"))
                .count()
        })
    });

    group.finish();
}

fn bench_pathological(c: &mut Criterion) {
    let re = Regex::new(c"a*a*b").unwrap();
    // 32 a's — no 'b'; memo caches all failed states so the engine terminates.
    let pathological_cstr = std::ffi::CString::new("a".repeat(32)).unwrap();

    let mut group = c.benchmark_group("pathological");
    group.bench_function("a_star_a_star_b/no_match", |b| {
        b.iter(|| re.find_at(black_box(pathological_cstr.as_c_str()), black_box(0)))
    });
    group.finish();
}

fn bench_small_capacity(c: &mut Criterion) {
    type SmallRe = RegexBuf<8, 16, 64>;
    let re = SmallRe::new(c"[0-9]+").unwrap();
    let mut group = c.benchmark_group("small_capacity");
    group.bench_function("digits_match", |b| {
        b.iter(|| re.find_at(black_box(c"abc123"), black_box(0)))
    });
    group.finish();
}

criterion_group!(benches, bench_compile, bench_recompile, bench_match, bench_find_iter, bench_pathological, bench_small_capacity);
criterion_main!(benches);
