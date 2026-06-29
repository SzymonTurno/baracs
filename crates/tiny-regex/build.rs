fn main() {
    let mut build = cc::Build::new();

    // Set cross-compiler and arch-specific flags for the current target.
    ccfg::configure_arch(&mut build);

    // Set standard C flags and opt level.
    build
        .opt_level_str("s")
        .flag_if_supported("-Wall")
        .flag_if_supported("-Wextra")
        .flag_if_supported("-pedantic")
        .flag_if_supported("-funsigned-char");

    // Add coverage if the coverage feature is active. This will override
    // opt_level and compiler.
    ccfg::apply_coverage(&mut build);

    // Build C code.  Explicitly declare header deps so cargo rebuilds when they change —
    // cc only tracks .c files by default and misses #include'd headers.
    println!("cargo:rerun-if-changed=src/c_src/re.c");
    println!("cargo:rerun-if-changed=src/c_src/re.h");
    println!("cargo:rerun-if-changed=src/c_src/re_memo.h");
    build.file("src/c_src/re.c").compile("tiny-regex-c");

    // Generate Rust FFI bindings from re.h.
    let out_dir = std::env::var("OUT_DIR").unwrap();
    bindgen::Builder::default()
        .header("src/c_src/re.h")
        .clang_arg("-Isrc/c_src")
        .allowlist_type("regex_t")
        .allowlist_function("re_compile")
        .allowlist_function("re_matchp")
        .use_core()
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("bindgen failed on src/c_src/re.h")
        .write_to_file(format!("{out_dir}/bindings.rs"))
        .expect("failed to write bindings.rs");
}
