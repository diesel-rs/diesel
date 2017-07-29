use schema::TestBackend;
use diesel::*;

#[test]
fn test_debug_count_output() {
    use schema::users::dsl::*;
    let sql = debug_query::<TestBackend, _>(&users.count()).to_string();
    if cfg!(feature = "postgres") {
        assert_eq!(sql, r#"SELECT COUNT(*) FROM "users" -- binds: []"#);
    } else {
        assert_eq!(sql, "SELECT COUNT(*) FROM `users` -- binds: []");
    }
}

#[test]
fn test_debug_output() {
    use schema::users::dsl::*;
    let command = update(users.filter(id.eq(1))).set(name.eq("new_name"));
    let sql = debug_query::<TestBackend, _>(&command).to_string();
    if cfg!(feature = "postgres") {
        assert_eq!(sql, r#"UPDATE "users" SET "name" = $1 WHERE "users"."id" = $2 -- binds: ["new_name", 1]"#)
    } else {
        assert_eq!(sql, r#"UPDATE `users` SET `name` = ? WHERE `users`.`id` = ? -- binds: ["new_name", 1]"#)
    }
}
