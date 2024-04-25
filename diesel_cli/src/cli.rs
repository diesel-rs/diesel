use clap::{
    builder::{EnumValueParser, PossibleValuesParser},
    Arg, ArgAction, Command,
};
use clap_complete::Shell;

use crate::print_schema;

fn position_sensitive_flag(arg: Arg) -> Arg {
    arg.num_args(0)
        .value_parser(clap::value_parser!(bool))
        .default_missing_value("true")
        .default_value("false")
}

pub fn build_cli() -> Command {
    let database_arg = Arg::new("DATABASE_URL")
        .long("database-url")
        .help(
            "Specifies the database URL to connect to. Falls back to \
             the DATABASE_URL environment variable if unspecified.",
        )
        .global(true)
        .num_args(1);

    let migration_subcommand = Command::new("migration")
        .about(
            "A group of commands for generating, running, and reverting \
             migrations.",
        )
        .arg(migration_dir_arg())
        .subcommand(Command::new("run").about("Runs all pending migrations."))
        .subcommand(
            Command::new("revert")
                .about("Reverts the specified migrations.")
                .arg(
                    Arg::new("REVERT_ALL")
                        .long("all")
                        .short('a')
                        .help("Reverts previously run migration files.")
                        .action(ArgAction::SetTrue)
                        .conflicts_with("REVERT_NUMBER"),
                )
                .arg(
                    Arg::new("REVERT_NUMBER")
                        .long("number")
                        .short('n')
                        .help("Reverts the last `n` migration files.")
                        .long_help(
                            "When this option is specified the last `n` migration files \
                             will be reverted. By default revert the last one.",
                        )
                        .default_value("1")
                        .num_args(1)
                        .value_parser(clap::value_parser!(u64))
                        .conflicts_with("REVERT_ALL"),
                ),
        )
        .subcommand(
            Command::new("redo")
                .about(
                    "Reverts and re-runs the latest migration. Useful \
                     for testing that a migration can in fact be reverted.",
                )
                .arg(
                    Arg::new("REDO_ALL")
                        .long("all")
                        .short('a')
                        .help("Reverts and re-runs all migrations.")
                        .long_help(
                            "When this option is specified all migrations \
                             will be reverted and re-runs. Useful for testing \
                             that your migrations can be reverted and applied.",
                        )
                        .action(ArgAction::SetTrue)
                        .conflicts_with("REDO_NUMBER"),
                )
                .arg(
                    Arg::new("REDO_NUMBER")
                        .long("number")
                        .short('n')
                        .help("Redo the last `n` migration files.")
                        .long_help(
                            "When this option is specified the last `n` migration files \
                             will be reverted and re-runs. By default redo the last migration.",
                        )
                        .default_value("1")
                        .num_args(1)
                        .value_parser(clap::value_parser!(u64))
                        .conflicts_with("REDO_ALL"),
                ),
        )
        .subcommand(
            Command::new("list")
                .about("Lists all available migrations, marking those that have been applied."),
        )
        .subcommand(
            Command::new("pending").about("Returns true if there are any pending migrations."),
        )
        .subcommand(
            Command::new("generate")
                .about(
                    "Generate a new migration with the given name, and \
                     the current timestamp as the version.",
                )
                .arg(
                    Arg::new("MIGRATION_NAME")
                        .index(1)
                        .num_args(1)
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
                        .num_args(1),
                )
                .arg(
                    Arg::new("MIGRATION_NO_DOWN_FILE")
                        .short('u') // only Up
                        .long("no-down")
                        .help(
                            "Don't generate a down.sql file. \
                            You won't be able to run migration `revert` or `redo`.",
                        )
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("MIGRATION_FORMAT")
                        .long("format")
                        .value_parser(PossibleValuesParser::new(["sql"]))
                        .num_args(1)
                        .default_value("sql")
                        .help("The format of the migration to be generated."),
                )
                .arg(
                    Arg::new("SCHEMA_RS")
                        .long("diff-schema")
                        .help(
                            "Populate the generated migrations \
                             based on the current difference between \
                             your `schema.rs` file and the specified \
                             database. \n\n\
                             The generated migrations are not expected to \
                             be perfect. Be sure to check whether they meet \
                             your expectations. Adjust the generated output \
                             if that's not the case.",
                        )
                        .default_missing_value("NOT_SET")
                        .num_args(0..=1)
                        .require_equals(true),
                )
                .arg(
                    Arg::new("sqlite-integer-primary-key-is-bigint")
                        .long("sqlite-integer-primary-key-is-bigint")
                        .requires("SCHEMA_RS")
                        .action(ArgAction::SetTrue)
                        .help(
                            "For SQLite 3.37 and above, detect `INTEGER PRIMARY KEY` columns as `BigInt`, \
                             when the table isn't declared with `WITHOUT ROWID`.\n\
                             See https://www.sqlite.org/lang_createtable.html#rowid for more information.\n\
                             Only used with the `--diff-schema` argument."
                        ),
                )
                .arg(
                    Arg::new("table-name")
                        .index(2)
                        .num_args(1..)
                        .action(clap::ArgAction::Append)
                        .help("Table names to filter."),
                )
                .arg(
                    position_sensitive_flag(Arg::new("only-tables"))
                        .short('o')
                        .long("only-tables")
                        .action(ArgAction::Append)
                        .help("Only include tables from table-name that matches regexp."),
                )
                .arg(
                    position_sensitive_flag(Arg::new("except-tables"))
                        .short('e')
                        .long("except-tables")
                        .action(ArgAction::Append)
                        .help("Exclude tables from table-name that matches regex."),
                )
                .arg(
                    Arg::new("schema-key")
                        .long("schema-key")
                        .action(clap::ArgAction::Append)
                        .help("select schema key from diesel.toml, use 'default' for print_schema without key."),
                ),
        )
        .subcommand_required(true)
        .arg_required_else_help(true);

    let setup_subcommand = Command::new("setup").arg(migration_dir_arg()).about(
        "Creates the migrations directory, creates the database \
             specified in your DATABASE_URL, and runs existing migrations.",
    );

    let database_subcommand = Command::new("database")
        .alias("db")
        .arg(migration_dir_arg())
        .about("A group of commands for setting up and resetting your database.")
        .subcommand(Command::new("setup").about(
            "Creates the database specified in your DATABASE_URL, \
             and then runs any existing migrations.",
        ))
        .subcommand(Command::new("reset").about(
            "Resets your database by dropping the database specified \
             in your DATABASE_URL and then running `diesel database setup`.",
        ))
        .subcommand(
            Command::new("drop")
                .about("Drops the database specified in your DATABASE_URL.")
                .hide(true),
        )
        .subcommand_required(true)
        .arg_required_else_help(true);

    let generate_completions_subcommand = Command::new("completions")
        .about("Generate shell completion scripts for the diesel command.")
        .arg(
            Arg::new("SHELL")
                .index(1)
                .required(true)
                .value_parser(EnumValueParser::<Shell>::new()),
        );

    let infer_schema_subcommand = Command::new("print-schema")
        .about("Print table definitions for database schema.")
        .arg(
            Arg::new("schema")
                .long("schema")
                .short('s')
                .num_args(1)
                .help("The name of the schema."),
        )
        .arg(
            Arg::new("table-name")
                .index(1)
                .num_args(1..)
                .action(clap::ArgAction::Append)
                .help("Table names to filter."),
        )
        .arg(
            position_sensitive_flag(Arg::new("only-tables"))
                .short('o')
                .long("only-tables")
                .action(ArgAction::Append)
                .help("Only include tables from table-name that matches regexp.")
        )
        .arg(
            position_sensitive_flag(Arg::new("except-tables"))
                .short('e')
                .long("except-tables")
                .action(ArgAction::Append)
                .help("Exclude tables from table-name that matches regex.")
        )
        .arg(
            position_sensitive_flag(Arg::new("with-docs"))
                .long("with-docs")
                .action(ArgAction::Append)
                .help("Render documentation comments for tables and columns."),
        )
        .arg(
            Arg::new("with-docs-config")
                .long("with-docs-config")
                .help("Render documentation comments for tables and columns.")
                .num_args(1)
                .action(ArgAction::Append)
                .value_parser(PossibleValuesParser::new(print_schema::DocConfig::VARIANTS_STR)),
        )
        .arg(
            Arg::new("column-sorting")
                .long("column-sorting")
                .help("Sort order for table columns.")
                .num_args(1)
                .action(ArgAction::Append)
                .value_parser(PossibleValuesParser::new(["ordinal_position", "name"])),
        )
        .arg(
            Arg::new("patch-file")
                .long("patch-file")
                .num_args(1)
                .action(ArgAction::Append)
                .value_parser(clap::value_parser!(std::path::PathBuf))
                .help("A unified diff file to be applied to the final schema."),
        )
        .arg(
            Arg::new("import-types")
                .long("import-types")
                .num_args(1..)
                .action(ArgAction::Append)
                .action(clap::ArgAction::Append)
                .number_of_values(1)
                .help("A list of types to import for every table, separated by commas."),
        )
        .arg(
            position_sensitive_flag(Arg::new("generate-custom-type-definitions"))
                .long("no-generate-missing-sql-type-definitions")
                .action(ArgAction::Append)
                .help("Generate SQL type definitions for types not provided by diesel"),
        )
        .arg(
            Arg::new("except-custom-type-definitions")
                .action(ArgAction::Append)
                .long("except-custom-type-definitions")
                .num_args(1..)
                .action(ArgAction::Append)
                .help("A list of regexes to filter the custom types definitions generated")
        )
        .arg(
            Arg::new("custom-type-derives")
                .long("custom-type-derives")
                .num_args(1..)
                .action(clap::ArgAction::Append)
                .number_of_values(1)
                .help("A list of derives to implement for every automatically generated SqlType in the schema, separated by commas."),
        )
        .arg(
            Arg::new("schema-key")
                .long("schema-key")
                .action(ArgAction::Append)
                .default_values(["default"])
                .help("select schema key from diesel.toml, use 'default' for print_schema without key."),
        ).arg(
        position_sensitive_flag(Arg::new("sqlite-integer-primary-key-is-bigint"))
            .long("sqlite-integer-primary-key-is-bigint")
            .action(ArgAction::Append)
            .help(
                "For SQLite 3.37 and above, detect `INTEGER PRIMARY KEY` columns as `BigInt`, \
                     when the table isn't declared with `WITHOUT ROWID`.\n\
                     See https://www.sqlite.org/lang_createtable.html#rowid for more information."
            ),
    );

    let config_arg = Arg::new("CONFIG_FILE")
        .value_parser(clap::value_parser!(std::path::PathBuf))
        .long("config-file")
        .help(
            "The location of the configuration file to use. Falls back to the \
             `DIESEL_CONFIG_FILE` environment variable if unspecified. Defaults \
             to `diesel.toml` in your project root. See \
             diesel.rs/guides/configuring-diesel-cli for documentation on this file.",
        )
        .global(true)
        .num_args(1);

    let locked_schema_arg = Arg::new("LOCKED_SCHEMA")
        .long("locked-schema")
        .help("Require that the schema file is up to date.")
        .long_help(
            "When `print_schema.file` is specified in your config file, this \
             flag will cause Diesel CLI to error if any command would result in \
             changes to that file. It is recommended that you use this flag when \
             running migrations in CI or production.",
        )
        .action(ArgAction::SetTrue)
        .global(true);

    Command::new("diesel")
        .version(clap::crate_version!())
        .long_version(
            clap::builder::Str::from(
                format!(
                    "\n Version: {}\n Supported Backends: {}",
                    clap::crate_version!(),
                    super::supported_backends()
                )
            )
        )
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
        .subcommand_required(true)
        .arg_required_else_help(true)
}

fn migration_dir_arg() -> Arg {
    Arg::new("MIGRATION_DIRECTORY")
        .long("migration-dir")
        .help(
            "The location of your migration directory. By default this \
             will look for a directory called `migrations` in the \
             current directory and its parents.",
        )
        .num_args(1)
        .value_parser(clap::value_parser!(std::path::PathBuf))
        .global(true)
}
