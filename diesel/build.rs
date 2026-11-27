// build.rs
fn main() {
    println!("cargo::rustc-check-cfg=cfg(mariadbbuild)");
    match std::env::var("DEP_MYSQLCLIENT_LIBMARIADB") {
        Ok(a) if a == "1" => {
            println!("Mariadb is found");
            println!("cargo::rustc-cfg=mariadbbuild");
        }
        Ok(a) if a == "0" => {
            println!("Mysql is found");
        }
        Ok(_) | Err(_) => {
            //Could be changed to a panic if build fails between dependencies
            println!("Cannot determine libmariadb/libmysql version, default to Mysql.")
        }
    }
}
