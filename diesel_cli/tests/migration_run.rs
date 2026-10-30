use crate::support::{database, project};
use diesel::dsl::sql;
use diesel::sql_types::{Bool, Text};
use diesel::{RunQueryDsl, select};
use std::path::Path;

#[test]
fn migration_run_runs_pending_migrations() {
    let p = project("migration_run").folder("migrations").build();
    let db = database(&p.database_url());

    // Make sure the project is setup
    p.command("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        Some("DROP TABLE users"),
        None,
    );

    assert!(!db.table_exists("users"));

    let result = p.command("migration").arg("run").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(db.table_exists("users"));
}

fn stored_version(db: &database::Database) -> String {
    select(sql::<Text>(
        "(SELECT version FROM __diesel_schema_migrations)",
    ))
    .get_result(&mut db.conn())
    .unwrap()
}

#[test]
fn migration_commands_respect_include_name_in_version() {
    for (setting, underscore_version) in [
        ("", "202610251934420000"),
        ("include_name_in_version = false", "202610251934420000"),
        (
            "include_name_in_version = true",
            "202610251934420000helloworld",
        ),
    ] {
        for (name, expected_version) in [
            (
                "2026-10-25-193442-0000-hello-world",
                "202610251934420000helloworld",
            ),
            ("2026-10-25-193442-0000_hello-world", underscore_version),
            ("2026-10-25-193442-0000_hello_world", underscore_version),
        ] {
            let p = project("migration_version_config")
                .folder("migrations")
                .file(
                    "diesel.toml",
                    &format!("[migrations_directory]\ndir = \"migrations\"\n{setting}\n"),
                )
                .build();
            let db = database(&p.database_url());
            p.create_migration(
                name,
                "CREATE TABLE hello (id INTEGER PRIMARY KEY)",
                Some("DROP TABLE hello"),
                None,
            );

            for (command, args) in [
                ("setup", &["--no-default-migration"][..]),
                ("migration", &["run"]),
                ("migration", &["redo"]),
                ("database", &["reset", "--no-default-migration"]),
            ] {
                let result = p.command(command).args(args.iter().copied()).run();
                assert!(result.is_success(), "Result was unsuccessful {:?}", result);
                assert_eq!(stored_version(&db), expected_version);
            }

            let result = p.command("migration").arg("pending").run();
            assert!(result.is_success(), "Result was unsuccessful {:?}", result);
            assert_eq!(result.stdout(), "false\n");

            let result = p.command("migration").arg("list").run();
            assert!(result.is_success(), "Result was unsuccessful {:?}", result);
            assert!(result.stdout().contains(&format!("[X] {name}")));

            let result = p.command("migration").arg("revert").run();
            assert!(result.is_success(), "Result was unsuccessful {:?}", result);
            assert!(!db.table_exists("hello"));

            let result = p.command("migration").arg("run").run();
            assert!(result.is_success(), "Result was unsuccessful {:?}", result);
            assert!(db.table_exists("hello"));
            assert_eq!(stored_version(&db), expected_version);
        }
    }
}

#[test]
fn migration_version_is_truncated_to_50_characters() {
    for character in ["a", "\u{e9}"] {
        for suffix_length in [32, 33, 60] {
            let p = project("migration_version_length")
                .folder("migrations")
                .file(
                    "diesel.toml",
                    "[migrations_directory]\ndir = \"migrations\"\ninclude_name_in_version = true\n",
                )
                .build();
            let db = database(&p.database_url());
            let result = p.command("setup").arg("--no-default-migration").run();
            assert!(result.is_success(), "Result was unsuccessful {:?}", result);

            let suffix = character.repeat(suffix_length);
            p.create_migration(
                &format!("2026-10-25-193442-0000_{suffix}"),
                "CREATE TABLE hello (id INTEGER PRIMARY KEY)",
                Some("DROP TABLE hello"),
                Some("run_in_transaction = false"),
            );
            let result = p.command("migration").arg("run").run();
            assert!(result.is_success(), "Result was unsuccessful {:?}", result);
            assert!(db.table_exists("hello"));
            assert_eq!(
                stored_version(&db),
                format!("202610251934420000{}", character.repeat(32))
            );
        }
    }
}

#[test]
fn migration_version_collisions_fail_before_running_sql() {
    let p = project("migration_version_collision")
        .folder("migrations")
        .file(
            "diesel.toml",
            "[migrations_directory]\ndir = \"migrations\"\ninclude_name_in_version = true\n",
        )
        .build();
    let db = database(&p.database_url());
    let result = p.command("setup").arg("--no-default-migration").run();
    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    p.create_migration(
        "2026-10-25-193441-0000_earlier",
        "CREATE TABLE earlier (id INTEGER PRIMARY KEY)",
        Some("DROP TABLE earlier"),
        Some("run_in_transaction = false"),
    );
    // The 18 timestamp digits plus this prefix fill all 50 version characters,
    // so the table names are truncated away and both versions are identical.
    let shared_prefix = "a".repeat(32);
    for table in ["first_table", "second_table"] {
        p.create_migration(
            &format!("2026-10-25-193442-0000_{shared_prefix}_{table}"),
            &format!("CREATE TABLE {table} (id INTEGER PRIMARY KEY)"),
            Some(&format!("DROP TABLE {table}")),
            Some("run_in_transaction = false"),
        );
    }

    let result = p.command("migration").arg("run").run();
    assert!(!result.is_success(), "Result was successful {:?}", result);
    assert!(result.stderr().contains("shared by multiple migrations"));
    for table in ["earlier", "first_table", "second_table"] {
        assert!(!db.table_exists(table));
    }
    let has_versions = select(sql::<Bool>(
        "EXISTS (SELECT 1 FROM __diesel_schema_migrations)",
    ))
    .get_result::<bool>(&mut db.conn())
    .unwrap();
    assert!(!has_versions);
}

#[test]
fn migration_version_format_preserves_initial_setup_output() {
    for include_name in [false, true] {
        let p = project("migration_initial_setup_output")
            .folder("migrations")
            .file(
                "diesel.toml",
                &format!(
                    "[migrations_directory]\ndir = \"migrations\"\ninclude_name_in_version = {include_name}\n"
                ),
            )
            .build();
        let db = database(&p.database_url());
        let result = p.command("setup").arg("--no-default-migration").run();
        assert!(result.is_success(), "Result was unsuccessful {:?}", result);
        p.create_migration(
            "00000000000000_diesel_initial_setup",
            "CREATE TABLE initial_setup (id INTEGER PRIMARY KEY)",
            Some("DROP TABLE initial_setup"),
            None,
        );

        let result = p.command("migration").arg("run").run();
        assert!(result.is_success(), "Result was unsuccessful {:?}", result);
        assert!(
            result.stdout().is_empty(),
            "Unexpected stdout {}",
            result.stdout()
        );
        assert!(db.table_exists("initial_setup"));

        let result = p.command("migration").arg("revert").run();
        assert!(result.is_success(), "Result was unsuccessful {:?}", result);
        assert!(
            result.stdout().is_empty(),
            "Unexpected stdout {}",
            result.stdout()
        );
        assert!(!db.table_exists("initial_setup"));
    }
}

#[test]
fn migration_run_inserts_run_on_timestamps() {
    let p = project("migration_run_on_timestamps")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    // Make sure the project is setup.
    p.command("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        Some("DROP TABLE users"),
        None,
    );

    let migrations_done: bool = select(sql::<Bool>(
        "EXISTS (SELECT * FROM __diesel_schema_migrations WHERE version >= '1')",
    ))
    .get_result(&mut db.conn())
    .unwrap();
    assert!(!migrations_done, "Migrations table should be empty");

    let result = p.command("migration").arg("run").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(db.table_exists("users"));

    // By running a query that compares timestamps, we are also checking
    // that the auto-inserted values for the "run_on" column are valid.

    #[cfg(feature = "sqlite")]
    fn valid_run_on_timestamp(db: &database::Database) -> bool {
        select(sql::<Bool>(
            "EXISTS (SELECT 1 FROM __diesel_schema_migrations \
             WHERE run_on < DATETIME('now', '+1 hour'))",
        ))
        .get_result(&mut db.conn())
        .unwrap()
    }

    #[cfg(feature = "postgres")]
    fn valid_run_on_timestamp(db: &database::Database) -> bool {
        select(sql::<Bool>(
            "EXISTS (SELECT 1 FROM __diesel_schema_migrations \
             WHERE run_on < NOW() + INTERVAL '1 hour')",
        ))
        .get_result(&mut db.conn())
        .unwrap()
    }

    #[cfg(feature = "mysql")]
    fn valid_run_on_timestamp(db: &database::Database) -> bool {
        select(sql::<Bool>(
            "EXISTS (SELECT 1 FROM __diesel_schema_migrations \
             WHERE run_on < NOW() + INTERVAL 1 HOUR)",
        ))
        .get_result(&mut db.conn())
        .unwrap()
    }

    #[cfg(feature = "mariadb")]
    fn valid_run_on_timestamp(db: &database::Database) -> bool {
        select(sql::<Bool>(
            "EXISTS (SELECT 1 FROM __diesel_schema_migrations \
             WHERE run_on < NOW() + INTERVAL 1 HOUR)",
        ))
        .get_result(&mut db.conn())
        .unwrap()
    }

    assert!(
        valid_run_on_timestamp(&db),
        "Running a migration did not insert an updated run_on value"
    );
}

#[test]
fn empty_migrations_are_not_valid() {
    let p = project("migration_run_empty").folder("migrations").build();

    p.command("setup").run();

    p.create_migration("12345_empty_migration", "", None, None);

    let result = p.command("migration").arg("run").run();

    assert!(!result.is_success());
    assert!(result.stderr().contains(
        "Failed to run 12345_empty_migration with: Attempted to run an empty migration."
    ));
}

#[test]
fn error_migrations_fails() {
    let p = project("run_error_migrations_fails")
        .folder("migrations")
        .build();

    p.command("setup").run();

    p.create_migration(
        "run_error_migrations_fails",
        "CREATE TABLE users (id INTEGER PRIMARY KEY}",
        Some("DROP TABLE users"),
        None,
    );

    let result = p.command("migration").arg("run").run();

    assert!(!result.is_success());
    assert!(
        result
            .stderr()
            .contains("Failed to run run_error_migrations_fails with: ")
    );
}

#[test]
#[cfg(feature = "postgres")]
fn error_migrations_when_use_invalid_database_url() {
    let p = project("error_migrations_when_use_invalid_database_url")
        .folder("migrations")
        .build();

    // Make sure the project is setup
    p.command("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        Some("DROP TABLE users"),
        None,
    );

    let result = p
        .command_without_database_url("migration")
        .arg("run")
        .arg("--database-url")
        .arg("postgres://localhost/lemmy")
        .run();

    assert!(!result.is_success());
    assert!(result.stderr().contains("Could not connect to database"));
}

#[test]
fn any_pending_migrations_works() {
    let p = project("any_pending_migrations_one")
        .folder("migrations")
        .build();

    p.command("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        Some("DROP TABLE users"),
        None,
    );

    let result = p.command("migration").arg("pending").run();

    assert!(result.stdout().contains("true\n"));
}

#[test]
fn any_pending_migrations_after_running() {
    let p = project("any_pending_migrations")
        .folder("migrations")
        .build();

    p.command("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        Some("DROP TABLE users"),
        None,
    );

    p.command("migration").arg("run").run();

    let result = p.command("migration").arg("pending").run();

    assert!(result.stdout().contains("false\n"));
}

#[test]
fn any_pending_migrations_after_running_and_creating() {
    let p = project("any_pending_migrations_run_then_create")
        .folder("migrations")
        .build();

    p.command("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        Some("DROP TABLE users"),
        None,
    );

    p.command("migration").arg("run").run();

    p.create_migration(
        "123456_create_posts_table",
        "CREATE TABLE posts (id INTEGER PRIMARY KEY)",
        Some("DROP TABLE posts"),
        None,
    );

    let result = p.command("migration").arg("pending").run();

    assert!(result.stdout().contains("true\n"));
}

#[test]
fn migration_run_runs_pending_migrations_custom_database_url_1() {
    let p = project("migration_run_custom_db_url_1")
        .folder("migrations")
        .build();
    let db_url = p.database_url();
    let db = database(&db_url);

    // Make sure the project is setup
    p.command("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        Some("DROP TABLE users"),
        None,
    );

    assert!(!db.table_exists("users"));

    let result = p
        .command_without_database_url("migration")
        .arg("run")
        .arg("--database-url")
        .arg(db_url)
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(db.table_exists("users"));
}

#[test]
fn migration_run_runs_pending_migrations_custom_database_url_2() {
    let p = project("migration_run_custom_db_url_2")
        .folder("migrations")
        .build();
    let db_url = p.database_url();
    let db = database(&db_url);

    // Make sure the project is setup
    p.command("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        Some("DROP TABLE users"),
        None,
    );

    assert!(!db.table_exists("users"));

    let result = p
        .command_without_database_url("migration")
        .arg("--database-url")
        .arg(db_url)
        .arg("run")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(db.table_exists("users"));
}

#[test]
fn migration_run_runs_pending_migrations_custom_migration_dir_1() {
    let p = project("migration_run_custom_migration_dir_1")
        .folder("custom_migrations")
        .build();
    let db_url = p.database_url();
    let db = database(&db_url);

    // Make sure the project is setup
    p.command("setup").run();

    p.create_migration_in_directory(
        "custom_migrations",
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        Some("DROP TABLE users"),
        None,
    );

    assert!(!db.table_exists("users"));

    let result = p
        .command("migration")
        .arg("run")
        .arg("--migration-dir")
        .arg(p.migration_dir_in_directory("custom_migrations"))
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(db.table_exists("users"));
}

#[test]
fn migration_run_runs_pending_migrations_custom_migration_dir_2() {
    let p = project("migration_run_custom_migration_dir_2")
        .folder("custom_migrations")
        .build();
    let db_url = p.database_url();
    let db = database(&db_url);

    // Make sure the project is setup
    p.command("setup").run();

    p.create_migration_in_directory(
        "custom_migrations",
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        Some("DROP TABLE users"),
        None,
    );

    assert!(!db.table_exists("users"));

    let result = p
        .command("migration")
        .arg("--migration-dir")
        .arg(p.migration_dir_in_directory("custom_migrations"))
        .arg("run")
        .run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(db.table_exists("users"));
}

#[test]
fn migration_run_updates_schema_if_config_present() {
    let p = project("migration_run_updates_schema_if_config_present")
        .folder("migrations")
        .file(
            "diesel.toml",
            r#"
            [print_schema]
            file = "src/my_schema.rs"
            "#,
        )
        .build();

    // Make sure the project is setup
    p.command("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        Some("DROP TABLE users"),
        None,
    );

    assert!(!p.has_file("src/my_schema.rs"));

    let result = p.command("migration").arg("run").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(p.has_file("src/my_schema.rs"));
}

#[test]
fn migration_run_ignores_schema_if_config_present_with_no_schema_cli_flag() {
    let p = project("migration_run_ignores_schema_with_cli_flag")
        .folder("migrations")
        .file(
            "diesel.toml",
            r#"
            [print_schema]
            file = "src/my_schema.rs"
            "#,
        )
        .build();

    // Make sure the project is setup
    p.command("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        Some("DROP TABLE users"),
        None,
    );

    assert!(!p.has_file("src/my_schema.rs"));

    let result = p.command("migration").arg("run").arg("--no-schema").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    // Schema still doesn't exist.
    assert!(!p.has_file("src/my_schema.rs"));
}

#[test]
fn migrations_can_be_run_with_no_config_file() {
    let p = project("migration_run_no_config_file")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    p.command("database").arg("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        Some("DROP TABLE users"),
        None,
    );

    assert!(!db.table_exists("users"));

    let result = p.command("migration").arg("run").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(db.table_exists("users"));
}

#[test]
fn migrations_can_be_run_with_no_cargo_toml() {
    let p = project("migration_run_no_cargo_toml")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    let cargo_toml_path = Path::new("Cargo.toml");
    p.delete_single_file(cargo_toml_path);

    p.command("database").arg("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        Some("DROP TABLE users"),
        None,
    );

    assert!(!db.table_exists("users"));

    let result = p.command("migration").arg("run").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(db.table_exists("users"));
}

#[test]
fn migrations_can_be_run_with_no_down() {
    let p = project("migrations_can_be_run_with_no_down")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    let cargo_toml_path = Path::new("Cargo.toml");
    p.delete_single_file(cargo_toml_path);

    p.command("database").arg("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        None,
        None,
    );

    assert!(!db.table_exists("users"));

    let result = p.command("migration").arg("run").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(db.table_exists("users"));
}

#[test]
fn verify_schema_errors_if_schema_file_would_change() {
    let p = project("migration_run_verify_schema_errors")
        .folder("migrations")
        .file(
            "diesel.toml",
            r#"
            [print_schema]
            file = "src/my_schema.rs"
            "#,
        )
        .build();

    // Make sure the project is setup
    p.command("setup").run();

    p.create_migration(
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        Some("DROP TABLE users"),
        None,
    );

    assert!(!p.has_file("src/my_schema.rs"));

    let result = p.command("migration").arg("run").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(p.has_file("src/my_schema.rs"));

    p.create_migration(
        "12346_create_posts_table",
        "CREATE TABLE posts (id INTEGER PRIMARY KEY)",
        Some("DROP TABLE posts"),
        None,
    );

    let result = p
        .command("migration")
        .arg("run")
        .arg("--locked-schema")
        .run();

    assert!(
        !result.is_success(),
        "Result was successful, expected to fail {:?}",
        result
    );
    assert!(
        result
            .stderr()
            .contains("Command would result in changes to")
            && result.stderr().contains("src/my_schema.rs"),
        "Unexpected stderr {}",
        result.stderr()
    );
    assert!(p.has_file("src/my_schema.rs"));
}

#[test]
fn migration_run_runs_pending_migrations_custom_migrations_dir_from_diesel_toml() {
    let p = project("migration_run_custom_migration_dir_from_diesel_toml")
        .folder("custom_migrations")
        .file(
            "diesel.toml",
            r#"
            [migrations_directory]
            dir = "custom_migrations"
            "#,
        )
        .build();

    let db_url = p.database_url();
    let db = database(&db_url);

    // Make sure the project is setup
    p.command("setup").run();

    p.create_migration_in_directory(
        "custom_migrations",
        "12345_create_users_table",
        "CREATE TABLE users (id INTEGER PRIMARY KEY)",
        Some("DROP TABLE users"),
        None,
    );

    assert!(!db.table_exists("users"));

    let result = p.command("migration").arg("run").run();

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(
        result.stdout().contains("Running migration 12345"),
        "Unexpected stdout {}",
        result.stdout()
    );
    assert!(db.table_exists("users"));
}

#[cfg(not(any(feature = "mysql", feature = "mariadb")))] // mysql does not support DDL + Transactions
#[test]
fn migration_run_without_transaction() {
    let p = project("migration_run_without_transaction")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    // Make sure the project is setup
    p.command("setup").run();

    p.create_migration(
        "2023-05-08-210424_without_transaction",
        "BEGIN TRANSACTION;CREATE TABLE customers ( id INTEGER PRIMARY KEY );COMMIT TRANSACTION;",
        Some("BEGIN TRANSACTION;DROP TABLE customers; COMMIT TRANSACTION;"),
        Some("run_in_transaction = false"),
    );

    let result = p.command("migration").arg("run").run();
    assert!(db.table_exists("customers"));

    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(result.stdout() == "Running migration 2023-05-08-210424_without_transaction\n");
}

#[test]
#[cfg(feature = "postgres")]
fn reset_search_path_after_migration() {
    let p: crate::support::Project = project("print_schema_after_search_path_change")
        .folder("migrations")
        .build();
    let _db = database(&p.database_url());

    p.command("setup").run();

    p.create_migration(
        "12345_set_search_path",
        "CREATE SCHEMA IF NOT EXISTS custom_schema; SET search_path TO custom_schema;",
        Some("SET search_path TO public; DROP SCHEMA IF EXISTS custom_schema CASCADE;"),
        None,
    );

    p.create_migration(
        "12346_create_users",
        "CREATE TABLE users (id SERIAL PRIMARY KEY, name VARCHAR NOT NULL);",
        Some("DROP TABLE users;"),
        None,
    );

    let result = p.command("migration").arg("run").run();
    assert!(result.is_success(), "Migration failed: {:?}", result);
}
