fn main() {
    #[cfg(feature = "oodle")]
    println!(
        "cargo:rustc-link-search={}",
        std::env::var("OODLE").unwrap_or_else(|_| panic!(r"to use the oodle feature set the OODLE variable to a folder containing the oodle static libraries (these can be obtained from any unreal engine install above 4.27 in Engine\Source\Runtime\OodleDataCompression\Sdks\2.9.8)"))
    );
}
