
fn main() {
    #[cfg(windows)]
    println!("cargo:rustc-link-lib=Ws2_32");
}
