TINY_REGEX_LIB = target/release/libtiny_regex.rlib

.PHONY: test asan bench bench-vs coverage doc size help

## Run the standard test suite.
test:
	cargo test --package tiny-regex

## Run lib tests under AddressSanitizer (requires nightly).
asan:
	RUSTFLAGS="-Z sanitizer=address" CFLAGS="-fsanitize=address" \
	cargo +nightly test --package tiny-regex --lib --target x86_64-unknown-linux-gnu

## Run regex.rs benchmarks.
bench:
	cargo bench --package tiny-regex --bench regex

## Run vs_regex_lite.rs benchmarks.
bench-vs:
	cargo bench --package tiny-regex --bench vs_regex_lite

## Generate HTML coverage report (opens target/llvm-cov/html/index.html).
coverage:
	cargo llvm-cov --lib --features coverage --html --package tiny-regex

$(TINY_REGEX_LIB):
	cargo build --release --package tiny-regex

## Build documentation for all published crates.
doc:
	cargo doc --package tiny-regex --package ccfg --no-deps

## Show binary size by section (release build).
size: $(TINY_REGEX_LIB)
	size $(TINY_REGEX_LIB) | sed -E 's/ \(ex [^)]*\)//; s/[0-9a-f]{16,}-//g; s/-[0-9a-f]{16,}//g'

## Print this help.
help:
	@grep -E '^##' Makefile | sed 's/^## //'
