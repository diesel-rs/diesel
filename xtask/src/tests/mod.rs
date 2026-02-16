use crate::Backend;
use cargo_metadata::{Metadata, MetadataCommand};
use std::process::Command;
use std::process::Stdio;

#[derive(clap::Args, Debug)]
pub(crate) struct TestArgs {
    /// Run tests for a specific backend
    #[clap(default_value_t = Backend::All)]
    backend: Backend,
    /// skip the unit/integration tests
    #[clap(long = "no-integration-tests")]
    no_integration_tests: bool,
    /// skip the doc tests
    #[clap(long = "no-doc-tests")]
    no_doc_tests: bool,
    // skip the checks for the example schema setup
    #[clap(long = "no-example-schema-check")]
    no_example_schema_check: bool,
    /// do not abort running if we encounter an error
    /// while running tests for all backends
    #[clap(long = "keep-going")]
    keep_going: bool,
    // run wasm tests, currently only supports sqlite
    #[clap(long = "wasm")]
    wasm: bool,
    /// additional flags passed to cargo nextest while running
    /// unit/integration tests.
    ///
    /// This is useful for passing custom test filters/arguments
    ///
    /// See <https://nexte.st/docs/running/> for details
    flags: Vec<String>,
}

impl TestArgs {
    pub(crate) fn run(mut self) {
        let metadata = MetadataCommand::default().exec().unwrap();
        let success = if matches!(self.backend, Backend::All) {
            let mut success = true;
            for backend in Backend::ALL {
                self.backend = *backend;
                let result = self.run_tests(&metadata);
                success = success && result;
                if !result && !self.keep_going {
                    break;
                }
            }
            success
        } else {
            self.run_tests(&metadata)
        };
        if !success {
            std::process::exit(1);
        }
    }

    fn run_tests(&self, metadata: &Metadata) -> bool {
        let backend_name = self.backend.to_string();
        println!("Running tests for {backend_name}");
        let exclude = crate::utils::get_exclude_for_backend(&backend_name, metadata, self.wasm);
        if std::env::var("DATABASE_URL").is_err() {
            match self.backend {
                Backend::Postgres => {
                    if std::env::var("PG_DATABASE_URL").is_err() {
                        println!(
                            "Remember to set `PG_DATABASE_URL` for running the postgres tests"
                        );
                    }
                }
                Backend::Sqlite => {
                    if std::env::var("SQLITE_DATABASE_URL").is_err() {
                        println!(
                            "Remember to set `SQLITE_DATABASE_URL` for running the sqlite tests"
                        );
                    }
                }
                Backend::Mysql => {
                    if std::env::var("MYSQL_DATABASE_URL").is_err()
                        || std::env::var("MYSQL_UNIT_TEST_DATABASE_URL").is_err()
                    {
                        println!(
                            "Remember to set `MYSQL_DATABASE_URL` and `MYSQL_UNIT_TEST_DATABASE_URL` for running the mysql tests"
                        );
                    }
                }
                Backend::All => unreachable!(),
            }
        }
        let backend = &self.backend;
        if matches!(backend, Backend::Postgres | Backend::Mysql | Backend::All if self.wasm) {
            eprintln!(
                "Only the sqlite backend supports wasm for now, the current backend is {backend}"
            );
            return true;
        }
        let url = match backend {
            Backend::Postgres => std::env::var("PG_DATABASE_URL"),
            Backend::Sqlite => std::env::var("SQLITE_DATABASE_URL"),
            Backend::Mysql => std::env::var("MYSQL_DATABASE_URL"),
            Backend::All => unreachable!(),
        };
        let url = url
            .or_else(|_| std::env::var("DATABASE_URL"))
            .expect("DATABASE_URL is set for tests");

        if !self.wasm {
            // run the migrations
            let mut command = Command::new("cargo");
            command
                .args(["run", "-p", "diesel_cli", "--no-default-features", "-F"])
                .arg(backend.to_string())
                .args(["--", "migration", "run", "--migration-dir"])
                .arg(
                    metadata
                        .workspace_root
                        .join("migrations")
                        .join(backend.to_string()),
                )
                .arg("--database-url")
                .arg(&url);
            println!("Run database migration via `{command:?}`");
            let status = command
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .unwrap();
            if !status.success() {
                eprintln!("Failed to run migrations");
                return false;
            }
        }

        if !self.no_integration_tests {
            // run the normal tests via nextest
            let mut command = Command::new("cargo");
            command
                .args(["nextest", "run", "--workspace", "--no-default-features"])
                .current_dir(&metadata.workspace_root)
                .args(exclude)
                .arg("-F")
                .arg(format!("diesel/{backend}"))
                .args(["-F", "diesel/extras"])
                .arg("-F")
                .arg(format!("diesel_derives/{backend}"))
                .arg("-F")
                .arg(format!("migrations_macros/{backend}"))
                .arg("-F")
                .arg(format!("diesel_migrations/{backend}"))
                .arg("-F")
                .arg(format!("diesel_tests/{backend}"))
                .arg("-F")
                .arg(format!("diesel-dynamic-schema/{backend}"))
                .args(&self.flags);

            if matches!(self.backend, Backend::Mysql) {
                // cannot run mysql tests in parallel
                command.args(["-j", "1"]);
            }
            if self.wasm {
                command
                    .env("WASM_BINDGEN_TEST_TIMEOUT", "120")
                    .env(
                        "CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER",
                        "wasm-bindgen-test-runner",
                    )
                    .env("RUSTFLAGS", "--cfg getrandom_backend=\"wasm_js\"")
                    .arg("--target")
                    .arg("wasm32-unknown-unknown");
            } else {
                command.arg("-F").arg(format!("diesel_cli/{backend}"));
            }
            println!("Running tests via `{command:?}`: ");

            let out = command
                .stderr(Stdio::inherit())
                .stdout(Stdio::inherit())
                .status()
                .unwrap();
            if !out.success() {
                eprintln!("Failed to run integration tests");
                return false;
            }
        } else {
            println!("Integration tests skipped because `--no-integration-tests` was passed");
        }
        if !self.no_doc_tests {
            let mut command = Command::new("cargo");

            command
                .current_dir(&metadata.workspace_root)
                .args([
                    "test",
                    "--doc",
                    "--no-default-features",
                    "-p",
                    "diesel",
                    "-p",
                    "diesel_derives",
                    "-p",
                    "diesel_migrations",
                    "-p",
                    "diesel-dynamic-schema",
                    "-p",
                    "dsl_auto_type",
                    "-p",
                    "diesel_table_macro_syntax",
                    "-F",
                    "diesel/extras",
                ])
                .arg("-F")
                .arg(format!("diesel/{backend}"))
                .arg("-F")
                .arg(format!("diesel_derives/{backend}"))
                .arg("-F")
                .arg(format!("diesel-dynamic-schema/{backend}"));
            if matches!(backend, Backend::Mysql) {
                // cannot run mysql tests in parallel
                command.args(["-j", "1"]);
            }
            if self.wasm {
                command
                    .env("WASM_BINDGEN_TEST_TIMEOUT", "120")
                    .env(
                        "CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER",
                        "wasm-bindgen-test-runner",
                    )
                    .env("RUSTFLAGS", "--cfg getrandom_backend=\"wasm_js\"")
                    .arg("--target")
                    .arg("wasm32-unknown-unknown");
            }
            println!("Running tests via `{command:?}`: ");
            let status = command
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .unwrap();
            if !status.success() {
                eprintln!("Failed to run doc tests");
                return false;
            }
        } else {
            println!("Doc tests are skipped because `--no-doc-tests` was passed");
        }

        if !self.no_example_schema_check {
            let examples = metadata
                .workspace_root
                .join("examples")
                .join(backend.to_string());
            let temp_dir = if matches!(backend, Backend::Sqlite) {
                Some(tempfile::tempdir().unwrap())
            } else {
                None
            };
            let mut fail = false;
            for p in metadata
                .workspace_packages()
                .into_iter()
                .filter(|p| p.manifest_path.starts_with(&examples))
            {
                let example_root = p.manifest_path.parent().unwrap();
                if example_root.join("migrations").exists() {
                    let db_url = if matches!(backend, Backend::Sqlite) {
                        temp_dir
                            .as_ref()
                            .unwrap()
                            .path()
                            .join(&p.name)
                            .display()
                            .to_string()
                    } else {
                        // it's a url with the structure postgres://[user:password@host:port/database?options
                        // we parse it manually as we don't want to pull in the url crate with all
                        // its features
                        let (start, end) = url.rsplit_once('/').unwrap();
                        let query = end.split_once('?').map(|(_, q)| q);

                        let mut url = format!("{start}/{}", p.name);
                        if let Some(query) = query {
                            url.push('?');
                            url.push_str(query);
                        }
                        url
                    };

                    let mut command = Command::new("cargo");
                    command
                        .current_dir(example_root)
                        .args(["run", "-p", "diesel_cli", "--no-default-features", "-F"])
                        .arg(backend.to_string())
                        .args(["--", "database", "reset", "--locked-schema"])
                        .env("DATABASE_URL", db_url);
                    println!(
                        "Check schema for example `{}` ({example_root}) with command `{command:?}`",
                        p.name,
                    );
                    let status = command.status().unwrap();
                    if !status.success() {
                        fail = true;
                        eprintln!("Failed to check example schema for `{}`", p.name);
                        if !self.keep_going {
                            return false;
                        }
                    }
                }
            }
            if fail {
                return false;
            }
        } else {
            println!(
                "Example schema check is skipped because `--no-example-schema-check` was passed"
            );
        }

        true
    }
}
