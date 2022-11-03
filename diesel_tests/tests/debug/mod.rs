use crate::schema::TestBackend;
use diesel::*;

#[test]
fn test_debug_count_output() {
    use crate::schema::users::dsl::*;
    let sql = debug_query::<TestBackend, _>(&users.count()).to_string();
    if cfg!(feature = "postgres") {
        assert_eq!(sql, r#"SELECT COUNT(*) FROM "users" -- binds: []"#);
    } else {
        assert_eq!(sql, "SELECT COUNT(*) FROM `users` -- binds: []");
    }
}

#[test]
fn test_debug_output() {
    use crate::schema::users::dsl::*;
    let command = update(users.filter(id.eq(1))).set(name.eq("new_name"));
    let sql = debug_query::<TestBackend, _>(&command).to_string();
    if cfg!(feature = "postgres") {
        assert_eq!(
            sql,
            r#"UPDATE "users" SET "name" = $1 WHERE ("users"."id" = $2) -- binds: ["new_name", 1]"#
        )
    } else {
        assert_eq!(
            sql,
            r#"UPDATE `users` SET `name` = ? WHERE (`users`.`id` = ?) -- binds: ["new_name", 1]"#
        )
    }
}

#[test]
fn test_debug_batch_insert() {
    use crate::schema::users::dsl::*;

    let values = vec![
        (name.eq("Sean"), hair_color.eq(Some("black"))),
        (name.eq("Tess"), hair_color.eq(None::<&str>)),
    ];
    let borrowed_command = insert_into(users).values(&values);
    let borrowed_sql_display = debug_query::<TestBackend, _>(&borrowed_command).to_string();
    let borrowed_sql_debug = format!("{:?}", debug_query::<TestBackend, _>(&borrowed_command));

    let owned_command = insert_into(users).values(values);
    let owned_sql_display = debug_query::<TestBackend, _>(&owned_command).to_string();
    let owned_sql_debug = format!("{:?}", debug_query::<TestBackend, _>(&owned_command));

    if cfg!(feature = "postgres") {
        assert_eq!(
            borrowed_sql_display,
            r#"INSERT INTO "users" ("name", "hair_color") VALUES ($1, $2), ($3, $4) -- binds: ["Sean", Some("black"), "Tess", None]"#
        );
        assert_eq!(
            borrowed_sql_debug,
            r#"Query { sql: "INSERT INTO \"users\" (\"name\", \"hair_color\") VALUES ($1, $2), ($3, $4)", binds: ["Sean", Some("black"), "Tess", None] }"#
        );

        assert_eq!(
            owned_sql_display,
            r#"INSERT INTO "users" ("name", "hair_color") VALUES ($1, $2), ($3, $4) -- binds: ["Sean", Some("black"), "Tess", None]"#
        );
        assert_eq!(
            owned_sql_debug,
            r#"Query { sql: "INSERT INTO \"users\" (\"name\", \"hair_color\") VALUES ($1, $2), ($3, $4)", binds: ["Sean", Some("black"), "Tess", None] }"#
        );
    } else {
        assert_eq!(
            borrowed_sql_display,
            r#"INSERT INTO `users` (`name`, `hair_color`) VALUES (?, ?), (?, ?) -- binds: ["Sean", Some("black"), "Tess", None]"#
        );
        assert_eq!(
            borrowed_sql_debug,
            r#"Query { sql: "INSERT INTO `users` (`name`, `hair_color`) VALUES (?, ?), (?, ?)", binds: ["Sean", Some("black"), "Tess", None] }"#
        );

        assert_eq!(
            owned_sql_display,
            r#"INSERT INTO `users` (`name`, `hair_color`) VALUES (?, ?), (?, ?) -- binds: ["Sean", Some("black"), "Tess", None]"#
        );
        assert_eq!(
            owned_sql_debug,
            r#"Query { sql: "INSERT INTO `users` (`name`, `hair_color`) VALUES (?, ?), (?, ?)", binds: ["Sean", Some("black"), "Tess", None] }"#
        );
    }
}

#[test]
#[cfg(feature = "sqlite")]
fn test_insert_with_default() {
    // This test ensures that we've implemented `debug_query` for batch insert
    // containing a default value on sqlite
    // This requires a separate impl because it's more than one sql statement that
    // is executed

    use crate::schema::users::dsl::*;

    let values = vec![
        (Some(name.eq("Sean")), hair_color.eq(Some("black"))),
        (Some(name.eq("Tess")), hair_color.eq(None::<&str>)),
    ];
    let borrowed_command = insert_into(users).values(&values);
    let borrowed_sql_display = debug_query::<TestBackend, _>(&borrowed_command).to_string();
    let borrowed_sql_debug = format!("{:?}", debug_query::<TestBackend, _>(&borrowed_command));

    let owned_command = insert_into(users).values(values);
    let owned_sql_display = debug_query::<TestBackend, _>(&owned_command).to_string();
    let owned_sql_debug = format!("{:?}", debug_query::<TestBackend, _>(&owned_command));

    assert_eq!(
        borrowed_sql_display,
        r#"BEGIN;
INSERT INTO `users` (`name`, `hair_color`) VALUES (?, ?) -- binds: ["Sean", Some("black")]
INSERT INTO `users` (`name`, `hair_color`) VALUES (?, ?) -- binds: ["Tess", None]
COMMIT;
"#
    );
    assert_eq!(
        borrowed_sql_debug,
        r#"Query { sql: ["BEGIN", "INSERT INTO `users` (`name`, `hair_color`) VALUES (?, ?) -- binds: [\"Sean\", Some(\"black\")]", "INSERT INTO `users` (`name`, `hair_color`) VALUES (?, ?) -- binds: [\"Tess\", None]", "COMMIT"], binds: [] }"#
    );

    assert_eq!(
        owned_sql_display,
        r#"BEGIN;
INSERT INTO `users` (`name`, `hair_color`) VALUES (?, ?) -- binds: ["Sean", Some("black")]
INSERT INTO `users` (`name`, `hair_color`) VALUES (?, ?) -- binds: ["Tess", None]
COMMIT;
"#
    );
    assert_eq!(
        owned_sql_debug,
        r#"Query { sql: ["BEGIN", "INSERT INTO `users` (`name`, `hair_color`) VALUES (?, ?) -- binds: [\"Sean\", Some(\"black\")]", "INSERT INTO `users` (`name`, `hair_color`) VALUES (?, ?) -- binds: [\"Tess\", None]", "COMMIT"], binds: [] }"#
    );
}

#[test]
#[cfg(feature = "postgres")]
fn test_upsert() {
    // this test ensures we get the right debug string for upserts
    use crate::schema::users::dsl::*;

    let values = vec![
        (name.eq("Sean"), hair_color.eq(Some("black"))),
        (name.eq("Tess"), hair_color.eq(None::<&str>)),
    ];

    let upsert_command_single_where = insert_into(users)
        .values(&values)
        .on_conflict(hair_color)
        .filter_target(hair_color.eq("black"))
        .do_nothing();
    let upsert_single_where_sql_display =
        debug_query::<TestBackend, _>(&upsert_command_single_where).to_string();

    assert_eq!(
        upsert_single_where_sql_display,
        r#"INSERT INTO "users" ("name", "hair_color") VALUES ($1, $2), ($3, $4) ON CONFLICT ("hair_color") WHERE ("users"."hair_color" = $5) DO NOTHING -- binds: ["Sean", Some("black"), "Tess", None, "black"]"#
    );

    let upsert_command_second_where = insert_into(users)
        .values(&values)
        .on_conflict(hair_color)
        .filter_target(hair_color.eq("black"))
        .filter_target(name.eq("Sean"))
        .do_nothing();

    let upsert_second_where_sql_display =
        debug_query::<TestBackend, _>(&upsert_command_second_where).to_string();

    assert_eq!(
        upsert_second_where_sql_display,
        r#"INSERT INTO "users" ("name", "hair_color") VALUES ($1, $2), ($3, $4) ON CONFLICT ("hair_color") WHERE (("users"."hair_color" = $5) AND ("users"."name" = $6)) DO NOTHING -- binds: ["Sean", Some("black"), "Tess", None, "black", "Sean"]"#
    );
}
