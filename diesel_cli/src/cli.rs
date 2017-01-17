use clap::{App, AppSettings, Arg, SubCommand};

pub fn build_cli() -> App<'static, 'static> {
    let database_arg = Arg::with_name("DATABASE_URL")
        .long("database-url")
        .help("Specifies the database URL to connect to. Falls back to \
               the DATABASE_URL environment variable if unspecified.")
        .global(true)
        .takes_value(true);

    let migration_subcommand = SubCommand::with_name("migration")
        .about("A group of commands for generating, running, and reverting \
                migrations.")
        .setting(AppSettings::VersionlessSubcommands)
        .arg(Arg::with_name("MIGRATION_DIRECTORY")
            .long("migration-dir")
            .help("The location of your migration directory. By default this \
                   will look for a directory called `migrations` in the \
                   current directory and its parents.")
            .takes_value(true)
            .global(true)
        ).subcommand(SubCommand::with_name("run")
            .about("Runs all pending migrations")
        ).subcommand(SubCommand::with_name("revert")
            .about("Reverts the latest run migration")
        ).subcommand(SubCommand::with_name("redo")
            .about("Reverts and re-runs the latest migration. Useful \
                    for testing that a migration can in fact be reverted.")
        ).subcommand(SubCommand::with_name("generate")
            .about("Generate a new migration with the given name, and \
                    the current timestamp as the version"
             ).arg(Arg::with_name("MIGRATION_NAME")
                 .help("The name of the migration to create")
                 .required(true)
             ).arg(Arg::with_name("MIGRATION_VERSION")
                 .long("version")
                 .help("The version number to use when generating the migration. \
                        Defaults to the current timestamp, which should suffice \
                        for most use cases.")
                 .takes_value(true)
            )
        ).setting(AppSettings::SubcommandRequiredElseHelp);

    let setup_subcommand = SubCommand::with_name("setup")
        .about("Creates the migrations directory, creates the database \
                specified in your DATABASE_URL, and runs existing migrations.");

    let database_subcommand = SubCommand::with_name("database")
        .about("A group of commands for setting up and resetting your database.")
        .setting(AppSettings::VersionlessSubcommands)
        .subcommand(SubCommand::with_name("setup")
            .about("Creates the database specified in your DATABASE_URL, \
                    and then runs any existing migrations.")
        ).subcommand(SubCommand::with_name("reset")
            .about("Resets your database by dropping the database specified \
                    in your DATABASE_URL and then running `diesel database setup`.")
        ).subcommand(SubCommand::with_name("drop")
            .about("Drops the database specified in your DATABASE_URL.")
            .setting(AppSettings::Hidden)
        ).setting(AppSettings::SubcommandRequiredElseHelp);

    let generate_bash_completion_subcommand = SubCommand::with_name("bash-completion")
        .about("Generate bash completion script for the diesel command.");

    let infer_schema_subcommand = SubCommand::with_name("print-schema")
        .setting(AppSettings::VersionlessSubcommands)
        .about("Print table definitions for database schema.")
        .arg(Arg::with_name("schema")
             .long("schema")
             .short("s")
             .takes_value(true)
             .help("The name of the schema."))
        .arg(Arg::with_name("table-name")
             .index(1)
             .takes_value(true)
             .multiple(true)
             .help("Table names to filter (default whitelist)"))
        .arg(Arg::with_name("whitelist")
             .short("w")
             .long("whitelist")
             .help("Use table list as whitelist")
             .conflicts_with("blacklist"))
        .arg(Arg::with_name("blacklist")
             .short("b")
             .long("blacklist")
             .help("Use table list as blacklist")
             .conflicts_with("whitelist"));

    App::new("diesel")
        .version(env!("CARGO_PKG_VERSION"))
        .setting(AppSettings::VersionlessSubcommands)
        .after_help("You can also run `diesel SUBCOMMAND -h` to get more information about that subcommand.")
        .arg(database_arg)
        .subcommand(migration_subcommand)
        .subcommand(setup_subcommand)
        .subcommand(database_subcommand)
        .subcommand(generate_bash_completion_subcommand)
        .subcommand(infer_schema_subcommand)
        .setting(AppSettings::SubcommandRequiredElseHelp)
}
