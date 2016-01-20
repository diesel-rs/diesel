use diesel::*;
use diesel::query_builder::debug::DebugQueryBuilder;

#[test]
fn test_debug_count_output() {
    use schema::users::dsl::*;
    let sql = debug_sql!(users.count());
    assert_eq!(sql, "SELECT COUNT(*) FROM users");
}

#[test]
fn test_debug_output() {
    use schema::users::dsl::*;
    let command = update(users.filter(id.eq(1))).set(name.eq("new_name"));
    assert_eq!(debug_sql!(command), "UPDATE users SET name = ? WHERE users.id = ?")
}
