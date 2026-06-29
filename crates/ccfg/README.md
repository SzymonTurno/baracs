# ccfg

[![crates.io](https://img.shields.io/crates/v/ccfg.svg)](https://crates.io/crates/ccfg)
[![docs.rs](https://docs.rs/ccfg/badge.svg)](https://docs.rs/ccfg)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](../../LICENSE)

Build-script helpers for crates that compile C code via the [`cc`](https://docs.rs/cc) crate.

## Installation

```toml
[build-dependencies]
ccfg = { version = "0.1", features = ["build"] }
```

The `build` feature gates the `cc` dependency so it is only pulled in when
building `build.rs`, not in the final binary.

## Usage

```rust
// build.rs
fn main() {
    let mut build = cc::Build::new();
    build.file("src/c_src/re.c");

    ccfg::configure_arch(&mut build);
    ccfg::apply_coverage(&mut build);

    build.compile("re");
}
```

### `configure_arch`

Configures the compiler for the current target:

- **Zephyr cross-compilation** — when `ZEPHYR_BASE` is set and no `CC`/`TARGET_CC`
  override is present, reads `CMAKE_C_COMPILER` from `CMakeCache.txt` and sets it
  as the compiler. This is the only reliable source for the cross-compiler path
  inside a west workspace.
- **riscv64** — adds `-mcmodel=medany` so relocations work when Zephyr places the
  binary outside the low-2 GB address window.

Emits the appropriate `cargo:rerun-if-changed` and `cargo:rerun-if-env-changed`
directives as side effects.

### `apply_coverage`

Reconfigures `build` for LLVM source-based coverage when the calling crate's
`coverage` Cargo feature is active. Sets the compiler to Clang, forces `-O0`,
and adds `-fprofile-instr-generate`/`-fcoverage-mapping`. No-op otherwise.

Must be called after any `build.opt_level()` call, as it overrides that setting.

## License

Apache-2.0 — see [LICENSE](../../LICENSE).
