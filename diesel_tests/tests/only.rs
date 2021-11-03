use crate::schema::*;
use crate::schema_dsl::*;
use diesel::dsl::*;
use diesel::pg::expression::dsl::OnlyDsl;
use diesel::*;

table! {
    users (id) {
        id -> Int8,
        name -> Text,
        table_nr -> Int8,
    }
}

#[derive(Debug, PartialEq, Eq, Queryable, Clone, Insertable, AsChangeset, Selectable)]
#[table_name = "users"]
pub struct NewUser {
    pub name: String,
}

#[test]
fn select_from_only_with_inherited_table() {
    let connection = &mut connection();
    drop_table_cascade(connection, "users");
    create_table(
        "users",
        (
            integer("id").primary_key().auto_increment(),
            string("name").not_null(),
            integer("table_nr").not_null().default("1"),
        ),
    )
    .execute(connection)
    .unwrap();

    connection
        .execute("CREATE TABLE users2 (check (table_nr = 2)) inherits (users)")
        .unwrap();

    connection
        .execute("INSERT INTO users2 (name, table_nr) VALUES ('hello', 2)")
        .unwrap();

    // There is now only one entry in the users2 table, none in the users table.

    let n_users = users::table
        .select(count(users::id))
        .first::<i64>(connection)
        .unwrap();
    assert_eq!(n_users, 1);

    let n_users_in_main_table = users::table
        .only()
        .select(count(users::id))
        .first::<i64>(connection)
        .unwrap();
    assert_eq!(n_users_in_main_table, 0);
}

#[test]
fn select_from_only_filtering() {
    // Test that it's possible to call `.only().filter(..)`
    let connection = &mut connection();
    drop_table_cascade(connection, "users");
    create_table(
        "users",
        (
            integer("id").primary_key().auto_increment(),
            string("name").not_null(),
        ),
    )
    .execute(connection)
    .unwrap();

    diesel::insert_into(users::table)
        .values(users::name.eq("hello"))
        .execute(connection)
        .unwrap();
    diesel::insert_into(users::table)
        .values(users::name.eq("world"))
        .execute(connection)
        .unwrap();
    let results = users::table
        .only()
        .filter(users::name.eq("world"))
        .select(users::name)
        .load::<String>(connection)
        .unwrap();
    assert_eq!(results.len(), 1);
}
