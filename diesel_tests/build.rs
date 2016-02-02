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

#[cfg(feature = "postgres")]
mod outer {
    extern crate diesel;
    extern crate dotenv;
    use self::diesel::*;
    use self::diesel::pg::PgConnection;
    use self::dotenv::dotenv;
    use std::io;

    pub fn main() {
        dotenv().ok();
        let database_url = ::std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set to run tests");
        let connection = PgConnection::establish(&database_url).unwrap();
        let migrations_dir = migrations::find_migrations_directory().unwrap().join("postgresql");
        migrations::run_pending_migrations_in_directory(&connection, &migrations_dir, &mut io::sink()).unwrap();
        ::inner::main();
    }
}

#[cfg(not(feature = "postgres"))]
mod outer {
    pub fn main() {
        ::inner::main();
    }
}

fn main() {
    outer::main();
}
