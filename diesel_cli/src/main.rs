#[macro_use]
extern crate clap;
extern crate diesel;

use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use diesel::migrations;
use std::env;

fn main() {
    let database_arg = || Arg::with_name("DATABASE_URL")
        .long("database-url")
        .help("Specifies the database URL to connect to. Falls back to \
                   the DATABASE_URL environment variable if unspecified.")
        .takes_value(true);

    let migration_subcommand = SubCommand::with_name("migration")
        .setting(AppSettings::VersionlessSubcommands)
        .subcommand(
            SubCommand::with_name("run")
                .about("Runs all pending migrations")
                .arg(database_arg())
        ).subcommand(
            SubCommand::with_name("revert")
                .about("Reverts the latest run migration")
                .arg(database_arg())
        ).subcommand(
            SubCommand::with_name("redo")
                .about("Reverts and re-runs the latest migration. Useful \
                      for testing that a migration can in fact be reverted.")
                .arg(database_arg())
        ).subcommand(
            SubCommand::with_name("generate")
                .about("Generate a new migration with the given name, and \
                      the current timestamp as the version")
                .arg(Arg::with_name("MIGRATION_NAME")
                     .help("The name of the migration to create")
                     .required(true)
                 )
        ).setting(AppSettings::SubcommandRequiredElseHelp);

    let matches = App::new("diesel")
        .version(env!("CARGO_PKG_VERSION"))
        .setting(AppSettings::VersionlessSubcommands)
        .subcommand(migration_subcommand)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .get_matches();

    match matches.subcommand() {
        ("migration", Some(matches)) => run_migration_command(matches),
        _ => unreachable!(),
    }
}

// FIXME: We can improve the error handling instead of `unwrap` here.
fn run_migration_command(matches: &ArgMatches) {
    match matches.subcommand() {
        ("run", Some(args)) => {
            migrations::run_pending_migrations(&connection(&database_url(args)))
                .unwrap();
        }
        ("revert", Some(args)) => {
            migrations::revert_latest_migration(&connection(&database_url(args)))
                .unwrap();
        }
        ("redo", Some(args)) => {
            let connection = connection(&database_url(args));
            connection.transaction(|| {
                let reverted_version = try!(migrations::revert_latest_migration(&connection));
                migrations::run_migration_with_version(&connection, &reverted_version)
            }).unwrap();
        }
        ("generate", Some(args)) => {
            panic!("Migration generator is not implemented this pass")
        }
        _ => unreachable!("The cli parser should prevent reaching here"),
    }
}

fn database_url(matches: &ArgMatches) -> String {
    matches.value_of("DATABASE_URL")
        .map(|s| s.into())
        .or(env::var("DATABASE_URL").ok())
        .expect("The --database-url argument must be passed, \
                or the DATABASE_URL environment variable must be set.")
}

fn connection(database_url: &str) -> diesel::Connection {
    diesel::Connection::establish(database_url)
        .expect(&format!("Error connecting to {}", database_url))
}
