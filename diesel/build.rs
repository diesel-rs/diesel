// build.rs
fn main() {
    println!("cargo::rustc-check-cfg=cfg(mariadbbuild)");
    if std::env::var("DEP_MYSQLCLIENT_MARIADB").is_ok() {
        println!("Mariadb is found");
        println!("cargo::rustc-cfg=mariadbbuild");
    }
}
