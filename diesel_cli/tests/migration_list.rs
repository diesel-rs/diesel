
use chrono::Utc;
use std::thread::sleep;
use std::time::Duration;

use support::{database, project};
use diesel::migrations::TIMESTAMP_FORMAT;

#[test]
fn migration_list_lists_pending_applied_migrations() {
    let p = project("migration_list_pending_applied")
        .folder("migrations")
        .build();
    let db = database(&p.database_url());

    p.command("setup").run();

    p.create_migration("12345_create_users_table",
                       "CREATE TABLE users ( id INTEGER )",
                       "DROP TABLE users");

    assert!(!db.table_exists("users"));

    // finds unapplied migration
    let result = p.command("migration")
        .arg("list")
        .run();
    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(result.stdout().contains("[ ] 12345_create_users_table"));

    let result = p.command("migration")
        .arg("run")
        .run();
    assert!(result.is_success());
    assert!(db.table_exists("users"));

    // finds applied migration
    let result = p.command("migration")
        .arg("list")
        .run();
    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(result.stdout().contains("[X] 12345_create_users_table"));
}

fn assert_tags_in_order(output: &str, tags: &[&str]) {
    let matches: Vec<_> = tags.iter().map(|s| {
        let index = output.find(s).expect(&format!("tag {:?} not found in output: {:?}", s, output));
        (index, s)
    }).collect();
    for window in matches.as_slice().windows(2) {
        assert!(window[0].0 < window[1].0, "expected {:?} before {:?}", window[0].1, window[1].1);
    }
}

#[test]
fn migration_list_lists_migrations_ordered_by_timestamp() {
    let p = project("migration_list_ordered")
        .folder("migrations")
        .build();

    p.command("setup").run();

    let tag1 = format!("{}_initial", Utc::now().format(TIMESTAMP_FORMAT));
    p.create_migration(&tag1, "", "");

    let result = p.command("migration")
        .arg("list")
        .run();
    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    assert!(result.stdout().contains(&format!("[ ] {}", &tag1)));

    sleep(Duration::from_millis(1100));

    let tag2 = format!("{}_alter", Utc::now().format(TIMESTAMP_FORMAT));
    p.create_migration(&tag2, "", "");

    let result = p.command("migration")
        .arg("list")
        .run();
    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    let output = result.stdout();
    assert_tags_in_order(output, &[&tag1, &tag2]);
}

#[test]
fn migration_list_orders_unknown_timestamps_last() {
    let p = project("migration_list_custom")
        .folder("migrations")
        .build();

    p.command("setup").run();

    let tag1 = format!("{}_migration1", Utc::now().format(TIMESTAMP_FORMAT));
    p.create_migration(&tag1, "", "");

    let tag4 = "abc_migration4";
    p.create_migration(&tag4, "", "");

    let tag5 = "zzz_migration5";
    p.create_migration(&tag5, "", "");

    sleep(Duration::from_millis(1100));

    let tag2 = format!("{}_migration2", Utc::now().format(TIMESTAMP_FORMAT));
    p.create_migration(&tag2, "", "");

    sleep(Duration::from_millis(1100));

    let tag3 = format!("{}_migration3", Utc::now().format(TIMESTAMP_FORMAT));
    p.create_migration(&tag3, "", "");

    let result = p.command("migration")
        .arg("list")
        .run();
    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    let output = result.stdout();
    assert_tags_in_order(output, &[&tag1, &tag2, &tag3, &tag4, &tag5]);
}

#[test]
fn migration_list_orders_nontimestamp_versions_lexicographically() {
    let p = project("migration_list_nontimestamp_versions")
        .folder("migrations")
        .build();

    p.command("setup").run();

    let tag4 = "a_migration";
    p.create_migration(&tag4, "", "");

    let tag6 = "bc_migration";
    p.create_migration(&tag6, "", "");

    let tag5 = "aa_migration";
    p.create_migration(&tag5, "", "");

    let tag2 = "!wow_migration";
    p.create_migration(&tag2, "", "");

    let tag3 = "7letters";
    p.create_migration(&tag3, "", "");

    let tag1 = format!("{}_stamped_migration", Utc::now().format(TIMESTAMP_FORMAT));
    p.create_migration(&tag1, "", "");

    let result = p.command("migration")
        .arg("list")
        .run();
    assert!(result.is_success(), "Result was unsuccessful {:?}", result);
    let output = result.stdout();
    assert_tags_in_order(output, &[&tag1, &tag2, &tag3, &tag4, &tag5, &tag6]);
}
