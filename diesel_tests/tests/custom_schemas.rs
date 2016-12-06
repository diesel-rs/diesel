use diesel::*;
use schema::connection;

infer_schema!("dotenv:DATABASE_URL", "custom_schema");
use self::custom_schema::users;

#[derive(Insertable)]
#[table_name="users"]
struct NewUser {
    id: i32,
}

#[test]
fn custom_schemas_are_loaded_by_infer_schema() {
    let conn = connection();
    insert(&NewUser { id: 1 }).into(users::table)
        .execute(&conn).unwrap();
    let users = users::table.select(users::id)
        .load(&conn);

    assert_eq!(Ok(vec![1]), users);
}
