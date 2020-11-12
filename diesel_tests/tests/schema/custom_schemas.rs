use crate::schema::connection;
use diesel::*;

include!("pg_custom_schema.rs");
use self::custom_schema::users;

#[derive(Insertable)]
#[diesel(table_name = users)]
struct NewUser {
    id: i32,
}

#[test]
fn custom_schemas_are_loaded_by_infer_schema() {
    let conn = &mut connection();
    insert_into(users::table)
        .values(&NewUser { id: 1 })
        .execute(conn)
        .unwrap();
    let users = users::table.select(users::id).load(conn);

    assert_eq!(Ok(vec![1]), users);
}
