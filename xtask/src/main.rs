use std::fmt::Display;

use clap::{Parser, ValueEnum};

mod tests;

#[derive(Debug, Parser)]
enum Commands {
    /// Run all tests for diesel
    ///
    /// Requires `cargo-nextest` to be installed
    RunTests(tests::TestArgs),
}

impl Commands {
    fn run(self) {
        match self {
            Commands::RunTests(test_args) => test_args.run(),
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
