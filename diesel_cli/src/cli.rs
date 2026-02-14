use std::io::stdout;
use std::path::PathBuf;

use crate::database::DatabaseArgs;
use crate::migrations::MigrationArgs;
use crate::print_schema::PrintSchemaArgs;
use clap::CommandFactory;
use clap::{ArgAction, Parser, Subcommand};
use clap_complete::{Shell, generate};

#[derive(Parser, Debug)]
#[command(version = cli_long_version(), about, long_about = None, after_help = "You can also run `diesel SUBCOMMAND -h` to get more information about that subcommand.")]
pub struct Cli {
    /// Specifies the database URL to connect to. Falls back to the DATABASE_URL environment variable if unspecified.
    #[arg(long = "database-url", global = true)]
    pub database_url: Option<String>,

    /// The location of the configuration file to use. Falls back to the
    /// `DIESEL_CONFIG_FILE` environment variable if unspecified. Defaults
    /// to `diesel.toml` in your project root. See
    /// diesel.rs/guides/configuring-diesel-cli for documentation on this file.
    #[arg(id = "CONFIG_FILE", long = "config-file", global = true)]
    pub config_file: Option<PathBuf>,

    /// Require that the schema file is up to date.
    ///
    /// When `print_schema.file` is specified in your config file, this
    /// flag will cause Diesel CLI to error if any command would result in
    /// changes to that file. It is recommended that you use this flag when
    /// running migrations in CI or production.
    #[arg(id = "LOCKED_SCHEMA", long = "locked-schema", global = true, action = ArgAction::SetTrue)]
    pub locked_schema: bool,

    /// The location of your migration directory. By default this
    /// will look for a directory called `migrations` in the
    /// current directory and its parents.
    #[arg(id = "MIGRATION_DIRECTORY", long = "migration-dir", global = true)]
    pub migration_dir: Option<std::path::PathBuf>,

    #[command(subcommand)]
    pub command: DieselCliCommand,
}

#[derive(Subcommand, Debug)]
pub enum DieselCliCommand {
    /// A group of commands for generating, running, and reverting migrations.
    Migration(MigrationArgs),

    /// Creates the migrations directory, creates the database
    /// specified in your DATABASE_URL, and runs existing migrations.
    Setup {
        /// Don't generate the default migration.
        #[arg(id = "NO_DEFAULT_MIGRATION", long = "no-default-migration", action = ArgAction::SetTrue)]
        no_default_migration: bool,
    },

    /// A group of commands for setting up and resetting your database.
    #[command(alias = "db")]
    Database(DatabaseArgs),

    /// Generate shell completion scripts for the diesel command.
    Completions {
        #[arg(id = "SHELL", index = 1, required = true)]
        shell: Shell,
    },

    /// Print table definitions for database schema.
    PrintSchema(PrintSchemaArgs),
}

#[tracing::instrument]
pub fn generate_completions_command(shell: &Shell) {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    generate(*shell, &mut cmd, name, &mut stdout());
}

fn cli_long_version() -> String {
    format!(
        "\n Version: {}\n Supported Backends: {}",
        clap::crate_version!(),
        supported_backends()
    )
}

fn supported_backends() -> String {
    let features = &[
        #[cfg(feature = "postgres")]
        "postgres",
        #[cfg(feature = "mysql")]
        "mysql",
        #[cfg(feature = "sqlite")]
        "sqlite",
    ];

    features.join(" ")
}
