use std::io::stdout;
use std::path::PathBuf;

use crate::database::DatabaseArgs;
use crate::migrations::MigrationArgs;
use crate::print_schema::PrintSchemaArgs;
use clap::CommandFactory;
use clap::{ArgAction, Parser, Subcommand};
use clap_complete::{Shell, generate};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Specifies the database URL to connect to. Falls back to the DATABASE_URL environment variable if unspecified.
    #[arg(long = "database-url", global = true)]
    pub database_url: Option<String>,

    /// The location of the configuration file to use. Falls back to the
    /// `DIESEL_CONFIG_FILE` environment variable if unspecified. Defaults
    /// to `diesel.toml` in your project root. See
    /// diesel.rs/guides/configuring-diesel-cli for documentation on this file.
    #[arg(id = "CONFIG_FILE", long = "config-file", global = true, value_parser = clap::value_parser!(std::path::PathBuf))]
    pub config_file: Option<PathBuf>,

    /// When `print_schema.file` is specified in your config file, this
    /// flag will cause Diesel CLI to error if any command would result in
    /// changes to that file. It is recommended that you use this flag when
    /// running migrations in CI or production.
    #[arg(id = "LOCKED_SCHEMA", long = "locked-schema", global = true, action = ArgAction::SetTrue)]
    pub locked_schema: bool,

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
        /// The location of your migration directory. By default this
        /// will look for a directory called `migrations` in the
        /// current directory and its parents.
        #[arg(id = "MIGRATION_DIRECTORY", long = "migration-dir", value_parser = clap::value_parser!(std::path::PathBuf), global = true)]
        migration_dir: Option<std::path::PathBuf>,

        /// Don't generate the default migration.
        #[arg(id = "NO_DEFAULT_MIGRATION", long = "no-default-migration", action = ArgAction::SetTrue)]
        no_default_migration: bool,
    },

    /// A group of commands for setting up and resetting your database.
    #[command(alias = "db")]
    Database(DatabaseArgs),

    /// Generate shell completion scripts for the diesel command.
    Completions {
        #[arg(id = "SHELL", index = 1, required = true, value_parser)]
        shell: Shell,
    },

    PrintSchema(PrintSchemaArgs),
}

#[tracing::instrument]
pub fn generate_completions_command(shell: &Shell) {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    generate(*shell, &mut cmd, name, &mut stdout());
}
