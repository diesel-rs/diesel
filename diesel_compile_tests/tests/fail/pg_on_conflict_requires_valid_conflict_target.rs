extern crate diesel;

use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

table! {
    posts {
        id -> Integer,
        title -> VarChar,
    }
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser(#[diesel(column_name = name)] &'static str);

#[declare_sql_function]
extern "SQL" {
    fn lower(x: diesel::sql_types::Text) -> diesel::sql_types::Text;
}

fn main() {
    use self::users::dsl::*;
    let mut connection = PgConnection::establish("postgres://localhost").unwrap();

    let valid_insert = insert_into(users)
        .values(&NewUser("Sean"))
        .on_conflict(id)
        .do_nothing()
        .execute(&mut connection);
    // Sanity check, no error

    let column_from_other_table = insert_into(users)
        .values(&NewUser("Sean"))
        .on_conflict(posts::id);

    let expression_using_column_from_other_table = insert_into(users)
        .values(&NewUser("Sean"))
        .on_conflict(lower(posts::title));

    let random_non_expression = insert_into(users)
        .values(&NewUser("Sean"))
        .on_conflict("id");
}
