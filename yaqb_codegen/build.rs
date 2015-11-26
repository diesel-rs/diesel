#[cfg(feature = "with-syntex")]
mod inner {
    extern crate syntex;
    extern crate quasi_codegen;

    use std::env;
    use std::path::Path;

    pub fn main() {
        let out_dir = env::var_os("OUT_DIR").unwrap();
        let mut registry = syntex::Registry::new();
        quasi_codegen::register(&mut registry);

        let src = Path::new("src/lib.rs.in");
        let dst = Path::new(&out_dir).join("lib.rs");

        registry.expand("", &src, &dst).unwrap();
    }
}

#[cfg(not(feature = "with-syntex"))]
mod inner {
    pub fn main() {}
}

fn main() {
    inner::main();
}
