fn main() {
    let icu = pkg_config::Config::new()
        .cargo_metadata(false)
        .probe("icu-uc")
        .expect("ICU not found; set PKG_CONFIG_PATH");

    let mut build = cxx_build::bridge("src/bridge.rs");

    build
        .file("cpp/scoring.cpp")
        .file("cpp/text.cpp")
        .file("cpp/metrics.cpp")
        .include("cpp")
        .std("c++17");

    for path in &icu.include_paths {
        build.include(path);
    }

    build.compile("search_scoring");

    // Link ICU after our C++ library.
    pkg_config::Config::new()
        .probe("icu-uc")
        .expect("Failed to configure ICU linking");

    println!("cargo:rerun-if-changed=src/bridge.rs");
    println!("cargo:rerun-if-changed=cpp");
}