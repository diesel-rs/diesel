#[cfg(feature = "with-syntex")]
fn main() {
    extern crate diesel_codegen_syntex as diesel_codegen;

    use std::env;
    use std::path::Path;

    let out_dir = env::var_os("OUT_DIR").unwrap();

    let src = Path::new("src/lib.in.rs");
    let dst = Path::new(&out_dir).join("lib.rs");

    diesel_codegen::expand(&src, &dst).unwrap();
}

#[cfg(feature = "nightly")]
fn main() {}
