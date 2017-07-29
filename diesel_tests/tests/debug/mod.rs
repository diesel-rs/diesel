use schema::TestBackend;
use diesel::*;

#[test]
fn test_debug_count_output() {
    use schema::users::dsl::*;
    let sql = debug_sql::<TestBackend, _>(&users.count());
    if cfg!(feature = "postgres") {
        assert_eq!(sql, r#"SELECT COUNT(*) FROM "users""#);
    } else {
        assert_eq!(sql, "SELECT COUNT(*) FROM `users`");
    }
}

#[test]
fn test_debug_output() {
    use schema::users::dsl::*;
    let command = update(users.filter(id.eq(1))).set(name.eq("new_name"));
    let sql = debug_sql::<TestBackend, _>(&command);
    if cfg!(feature = "postgres") {
        assert_eq!(sql, r#"UPDATE "users" SET "name" = $1 WHERE "users"."id" = $2"#)
    } else {
        assert_eq!(sql, "UPDATE `users` SET `name` = ? WHERE `users`.`id` = ?")
    }
}
