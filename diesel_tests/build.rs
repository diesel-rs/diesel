#[cfg(not(feature = "unstable"))]
mod inner {
    extern crate syntex;
    extern crate diesel_codegen;
    extern crate dotenv_codegen;

    use std::env;
    use std::path::Path;

    pub fn main() {
        let out_dir = env::var_os("OUT_DIR").unwrap();
        let mut registry = syntex::Registry::new();
        diesel_codegen::register(&mut registry);
        dotenv_codegen::register(&mut registry);

        let src = Path::new("tests/lib.in.rs");
        let dst = Path::new(&out_dir).join("lib.rs");

        registry.expand("", &src, &dst).unwrap();
    }
}

#[cfg(feature = "unstable")]
mod inner {
    pub fn main() {}
}

fn main() {
    inner::main();
}
