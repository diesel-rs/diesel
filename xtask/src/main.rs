use std::fmt::Display;

use clap::{Parser, ValueEnum};

mod clippy;
mod semver_checks;
mod tests;
mod tidy;
mod utils;

#[derive(Debug, Parser)]
enum Commands {
    /// Run all tests for diesel
    ///
    /// Requires `cargo-nextest` to be installed
    RunTests(tests::TestArgs),
    /// Run clippy on all crates
    Clippy(clippy::ClippyArgs),
    /// Perform a set of preliminary checks
    ///
    /// This command will execute `cargo fmt --check` to verify that
    /// the code is formatted, `typos` to check for spelling errors
    /// and it will execute `xtask clippy` to verify that the code
    /// compiles without warning
    Tidy(tidy::TidyArgs),
    /// Check for semver relevant changes
    ///
    /// This command will execute `cargo semver-checks` to verify that
    /// no breaking changes are included
    SemverChecks(semver_checks::SemverArgs),
}

impl Commands {
    fn run(self) {
        match self {
            Commands::RunTests(test_args) => test_args.run(),
            Commands::Clippy(clippy) => clippy.run(),
            Commands::Tidy(tidy) => tidy.run(),
            Commands::SemverChecks(semver) => semver.run(),
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Backend {
    Postgres,
    Sqlite,
    Mysql,
    All,
}

impl Backend {
    const ALL: &'static [Self] = &[Backend::Postgres, Backend::Sqlite, Backend::Mysql];
}

impl Display for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Backend::Postgres => write!(f, "postgres"),
            Backend::Sqlite => write!(f, "sqlite"),
            Backend::Mysql => write!(f, "mysql"),
            Backend::All => write!(f, "all"),
        }
    }
}

fn main() {
    dotenvy::dotenv().ok();
    Commands::parse().run();
}
