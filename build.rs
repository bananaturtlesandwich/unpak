#[cfg(feature = "oodle")]
fn main() {
    cxx_build::bridge("src/entry.rs")
        .file("ooz/lzna.cpp")
        // MSVC flags
        .flag_if_supported("/std:c++latest")
        // GCC flags to suppress warnings
        .flag_if_supported("-Wno-conversion-null")
        .flag_if_supported("-Wno-sequence-point")
        .flag_if_supported("-Wno-sign-compare")
        .flag_if_supported("-Wno-shift-negative-value")
        .flag_if_supported("-Wno-unused-variable")
        .flag_if_supported("-Wno-unused-parameter")
        .compile("unpak");

    println!("cargo:rerun-if-changed=src/entry.rs");

    println!("cargo:rerun-if-changed=ooz/lzna.cpp");

    println!("cargo:rerun-if-changed=ooz/lzna.h");
    println!("cargo:rerun-if-changed=ooz/stdafx.h");
}

#[cfg(not(feature = "oodle"))]
fn main() {}
