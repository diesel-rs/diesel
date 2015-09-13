#[macro_use]
extern crate yaqb;

use yaqb::{QuerySource, Table, Column};

table! {
    users {
        id -> Serial,
        name -> VarChar,
        hair_color -> Nullable<VarChar>,
    }
}

#[test]
fn test_table_macro() {
    use self::users::*;
    use self::users::table as users;

    assert_eq!("users.*", users.select_clause());
    assert_eq!("users", users.from_clause());

    assert_eq!("*", star.name());
    assert_eq!("users.*", star.qualified_name());
    assert_eq!("id", id.name());
    assert_eq!("users.id", id.qualified_name());
    assert_eq!("name", name.name());
    assert_eq!("users.name", name.qualified_name());
    assert_eq!("hair_color", hair_color.name());
    assert_eq!("users.hair_color", hair_color.qualified_name());

    assert_eq!("users", users.name());
    assert_eq!(id, users.primary_key());

    assert_eq!("id, name", (id, name).name());
    assert_eq!("users.id, users.name", (id, name).qualified_name());
}
