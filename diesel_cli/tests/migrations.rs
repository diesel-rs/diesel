use regex::Regex;

use support::project;

#[test]
fn migration_generate_creates_a_migration_with_the_proper_name() {
    let p = project("migration_name")
        .folder("migrations")
        .build();
    let result = p.command("migration")
        .arg("generate")
        .arg("hello")
        .run();

    let expected_stdout = Regex::new("\
Creating migrations/\\d{14}_hello/up.sql
Creating migrations/\\d{14}_hello/down.sql\
        ").unwrap();
    assert!(result.is_success());
    assert!(result.stdout().contains(&expected_stdout));

    let migrations = p.migrations();
    assert_eq!(1, migrations.len());

    let migration = &migrations[0];
    assert_eq!("hello", migration.name());
    assert!(migration.path().join("up.sql").exists());
    assert!(migration.path().join("down.sql").exists());
}
