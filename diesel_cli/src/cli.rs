use crate::validators::num::*;
use clap::{App, AppSettings, Arg};
use clap_complete::Shell;

fn str_as_char(str: &str) -> char {
    str.chars().next().unwrap()
}

pub fn build_cli() -> App<'static> {
    let database_arg = Arg::new("DATABASE_URL")
        .long("database-url")
        .help(
            "Specifies the database URL to connect to. Falls back to \
             the DATABASE_URL environment variable if unspecified.",
        )
        .global(true)
        .takes_value(true);

    let migration_subcommand = App::new("migration")
        .about(
            "A group of commands for generating, running, and reverting \
             migrations.",
        )
        .arg(migration_dir_arg())
        .subcommand(App::new("run").about("Runs all pending migrations."))
        .subcommand(
            App::new("revert")
                .about("Reverts the specified migrations.")
                .arg(
                    Arg::new("REVERT_ALL")
                        .long("all")
                        .short(str_as_char("a"))
                        .help("Reverts previously run migration files.")
                        .takes_value(false)
                        .conflicts_with("REVERT_NUMBER"),
                )
                .arg(
                    Arg::new("REVERT_NUMBER")
                        .long("number")
                        .short(str_as_char("n"))
                        .help("Reverts the last `n` migration files.")
                        .long_help(
                            "When this option is specified the last `n` migration files \
                             will be reverted. By default revert the last one.",
                        )
                        .default_value("1")
                        .takes_value(true)
                        .validator(is_positive_int)
                        .conflicts_with("REVERT_ALL"),
                ),
        )
        .subcommand(
            App::new("redo")
                .about(
                    "Reverts and re-runs the latest migration. Useful \
                     for testing that a migration can in fact be reverted.",
                )
                .arg(
                    Arg::new("REDO_ALL")
                        .long("all")
                        .short(str_as_char("a"))
                        .help("Reverts and re-runs all migrations.")
                        .long_help(
                            "When this option is specified all migrations \
                             will be reverted and re-runs. Useful for testing \
                             that your migrations can be reverted and applied.",
                        )
                        .takes_value(false)
                        .conflicts_with("REDO_NUMBER"),
                )
                .arg(
                    Arg::new("REDO_NUMBER")
                        .long("number")
                        .short(str_as_char("n"))
                        .help("Redo the last `n` migration files.")
                        .long_help(
                            "When this option is specified the last `n` migration files \
                             will be reverted and re-runs. By default redo the last migration.",
                        )
                        .default_value("1")
                        .takes_value(true)
                        .validator(is_positive_int)
                        .conflicts_with("REDO_ALL"),
                ),
        )
        .subcommand(
            App::new("list")
                .about("Lists all available migrations, marking those that have been applied."),
        )
        .subcommand(App::new("pending").about("Returns true if there are any pending migrations."))
        .subcommand(
            App::new("generate")
                .about(
                    "Generate a new migration with the given name, and \
                     the current timestamp as the version.",
                )
                .arg(
                    Arg::new("MIGRATION_NAME")
                        .help("The name of the migration to create.")
                        .required(true),
                )
                .arg(
                    Arg::new("MIGRATION_VERSION")
                        .long("version")
                        .help(
                            "The version number to use when generating the migration. \
                             Defaults to the current timestamp, which should suffice \
                             for most use cases.",
                        )
                        .takes_value(true),
                )
                .arg(
                    Arg::new("MIGRATION_FORMAT")
                        .long("format")
                        .possible_values(&["sql", "barrel"])
                        .default_value("sql")
                        .takes_value(true)
                        .help("The format of the migration to be generated."),
                ),
        )
        .setting(AppSettings::SubcommandRequiredElseHelp);

    let setup_subcommand = App::new("setup").arg(migration_dir_arg()).about(
        "Creates the migrations directory, creates the database \
             specified in your DATABASE_URL, and runs existing migrations.",
    );

    let database_subcommand = App::new("database")
        .alias("db")
        .arg(migration_dir_arg())
        .about("A group of commands for setting up and resetting your database.")
        .subcommand(App::new("setup").about(
            "Creates the database specified in your DATABASE_URL, \
             and then runs any existing migrations.",
        ))
        .subcommand(App::new("reset").about(
            "Resets your database by dropping the database specified \
             in your DATABASE_URL and then running `diesel database setup`.",
        ))
        .subcommand(
            App::new("drop")
                .about("Drops the database specified in your DATABASE_URL.")
                .setting(AppSettings::Hidden),
        )
        .setting(AppSettings::SubcommandRequiredElseHelp);

    let generate_completions_subcommand = App::new("completions")
        .about("Generate shell completion scripts for the diesel command.")
        .arg(
            Arg::new("SHELL")
                .index(1)
                .required(true)
                .possible_values(Shell::possible_values()),
        );

    let infer_schema_subcommand = App::new("print-schema")
        .about("Print table definitions for database schema.")
        .arg(
            Arg::new("schema")
                .long("schema")
                .short(str_as_char("s"))
                .takes_value(true)
                .help("The name of the schema."),
        )
        .arg(
            Arg::new("table-name")
                .index(1)
                .takes_value(true)
                .multiple_values(true)
                .multiple_occurrences(true)
                .help("Table names to filter (default only-tables if not empty)."),
        )
        .arg(
            Arg::new("only-tables")
                .short(str_as_char("o"))
                .long("only-tables")
                .help("Only include tables from table-name that matches regexp.")
                .conflicts_with("except-tables"),
        )
        .arg(
            Arg::new("except-tables")
                .short(str_as_char("e"))
                .long("except-tables")
                .help("Exclude tables from table-name that matches regex.")
                .conflicts_with("only-tables"),
        )
        .arg(
            Arg::new("with-docs")
                .long("with-docs")
                .help("Render documentation comments for tables and columns."),
        )
        .arg(
            Arg::new("column-sorting")
                .long("column-sorting")
                .help("Sort order for table columns.")
                .takes_value(true)
                .possible_values(&["ordinal_position", "name"]),
        )
        .arg(
            Arg::new("patch-file")
                .long("patch-file")
                .takes_value(true)
                .help("A unified diff file to be applied to the final schema."),
        )
        .arg(
            Arg::new("import-types")
                .long("import-types")
                .takes_value(true)
                .multiple_values(true)
                .multiple_occurrences(true)
                .number_of_values(1)
                .help("A list of types to import for every table, separated by commas."),
        )
        .arg(
            Arg::new("generate-custom-type-definitions")
                .long("no-generate-missing-sql-type-definitions")
                .help("Generate SQL type definitions for types not provided by diesel"),
        );

    let config_arg = Arg::new("CONFIG_FILE")
        .long("config-file")
        .help(
            "The location of the configuration file to use. Falls back to the \
             `DIESEL_CONFIG_FILE` environment variable if unspecified. Defaults \
             to `diesel.toml` in your project root. See \
             diesel.rs/guides/configuring-diesel-cli for documentation on this file.",
        )
        .global(true)
        .takes_value(true);

    let locked_schema_arg = Arg::new("LOCKED_SCHEMA")
        .long("locked-schema")
        .help("Require that the schema file is up to date.")
        .long_help(
            "When `print_schema.file` is specified in your config file, this \
             flag will cause Diesel CLI to error if any command would result in \
             changes to that file. It is recommended that you use this flag when \
             running migrations in CI or production.",
        )
        .global(true);

    App::new("diesel")
        .version(env!("CARGO_PKG_VERSION"))
        .after_help(
            "You can also run `diesel SUBCOMMAND -h` to get more information about that subcommand.",
        )
        .arg(database_arg)
        .arg(config_arg)
        .arg(locked_schema_arg)
        .subcommand(migration_subcommand)
        .subcommand(setup_subcommand)
        .subcommand(database_subcommand)
        .subcommand(generate_completions_subcommand)
        .subcommand(infer_schema_subcommand)
        .setting(AppSettings::SubcommandRequiredElseHelp)
}

fn migration_dir_arg<'a>() -> Arg<'a> {
    Arg::new("MIGRATION_DIRECTORY")
        .long("migration-dir")
        .help(
            "The location of your migration directory. By default this \
             will look for a directory called `migrations` in the \
             current directory and its parents.",
        )
        .takes_value(true)
        .global(true)
}
