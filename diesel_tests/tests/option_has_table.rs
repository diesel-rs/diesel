use diesel::associations::HasTable;
use diesel::prelude::*;
use std::rc::Rc;
use std::sync::Arc;

table! {
    users (id) {
        id -> Integer,
        name -> Text,
    }
}

#[derive(Identifiable, Queryable, PartialEq, Debug)]
#[diesel(table_name = users)]
struct User {
    id: i32,
    name: String,
}

#[test]
fn option_user_implements_has_table() {
    fn assert_has_table<T: HasTable<Table = users::table>>() {
        // If this compiles, T::Table is users::table
    }

    assert_has_table::<User>();
    assert_has_table::<Option<User>>();
    assert_has_table::<Option<&User>>();
    assert_has_table::<Box<User>>();
    assert_has_table::<Rc<User>>();
    assert_has_table::<Arc<User>>();
}
