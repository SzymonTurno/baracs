/// Applies LLVM source-based coverage instrumentation flags to `build` when
/// the `coverage` Cargo feature is active (`CARGO_FEATURE_COVERAGE` is set).
///
/// Sets the compiler to Clang (required for LLVM coverage format), overrides
/// opt level to 0 (avoids branch elimination that skews coverage reports), and
/// adds `-fprofile-instr-generate`/`-fcoverage-mapping`.
///
/// Must be called after `build.opt_level()` — if coverage is active this
/// overrides that setting, relying on `cc::Build` applying the last call.
/// No-op when the feature is not active.
#[cfg(feature = "build")]
pub fn apply_coverage(build: &mut cc::Build) {
    if std::env::var_os("CARGO_FEATURE_COVERAGE").is_none() {
        return;
    }
    build
        .compiler("clang")
        .opt_level(0)
        .flag("-fprofile-instr-generate")
        .flag("-fcoverage-mapping");
}

/// Configures `build` for the current target architecture.
///
/// When inside a Zephyr west workspace, sets the cross-compiler by reading
/// `CMAKE_C_COMPILER` from `CMakeCache.txt` (the only reliable source for the
/// cross-compiler path). Skips compiler detection if `CC`/`TARGET_CC` are already
/// set, letting the caller override.
///
/// Also adds any architecture-specific C flags required for the target:
/// - riscv64: `-mcmodel=medany` so relocations work when Zephyr places the binary
///   outside the low-2 GB window.
///
/// Emits `cargo:rerun-if-changed`/`cargo:rerun-if-env-changed` as side effects.
#[cfg(feature = "build")]
pub fn configure_arch(build: &mut cc::Build) {
    println!("cargo:rerun-if-env-changed=ZEPHYR_BASE");
    println!("cargo:rerun-if-env-changed=TARGET_CC");
    println!("cargo:rerun-if-env-changed=CC");
    println!("cargo:rerun-if-env-changed=BUILD_DIR");

    let in_zephyr = std::env::var_os("ZEPHYR_BASE").is_some();
    let cc_override = std::env::var_os("TARGET_CC").is_some()
        || std::env::var_os("CC").is_some();

    if in_zephyr && !cc_override {
        let build_dir = std::env::var("BUILD_DIR").unwrap_or_default();
        let cmake_cache = format!("{build_dir}/CMakeCache.txt");
        println!("cargo:rerun-if-changed={cmake_cache}");

        if let Some(compiler) = std::fs::read_to_string(&cmake_cache).ok().and_then(|s| {
            s.lines().find_map(|line| {
                // CMAKE_C_COMPILER:FILEPATH=/path/to/gcc
                let rest = line.strip_prefix("CMAKE_C_COMPILER:")?;
                rest.split_once('=').map(|(_, path)| path.to_owned())
            })
        }) {
            build.compiler(compiler);
        }
    }

    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("riscv64") {
        build.flag("-mcmodel=medany");
    }
}

