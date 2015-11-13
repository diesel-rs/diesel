use schema::*;
use yaqb::*;
use yaqb::persistable::InsertableColumns;

mod mine {
    table! {
        users {
            id -> Serial,
            name -> VarChar,
            hair_color -> Nullable<VarChar>,
        }
    }
}

#[test]
fn test_table_macro() {
    use self::mine::users::*;
    use self::mine::users::table as users;

    assert_eq!("*", star.name());
    assert_eq!("users.*", star.qualified_name());
    assert_eq!("id", id.name());
    assert_eq!("users.id", id.qualified_name());
    assert_eq!("name", name.name());
    assert_eq!("users.name", name.qualified_name());
    assert_eq!("hair_color", hair_color.name());
    assert_eq!("users.hair_color", hair_color.qualified_name());

    assert_eq!("users", users.name());
    assert_eq!("id", users.primary_key().name());

    assert_eq!("id, name", (id, name).names());
}

sql_function!(my_lower, (x: VarChar) -> VarChar);

#[test]
fn test_sql_function() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    connection.execute("CREATE FUNCTION my_lower(varchar) RETURNS varchar
        AS $$ SELECT LOWER($1) $$
        LANGUAGE SQL").unwrap();
    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");

    assert_eq!(vec![sean], users.filter(my_lower(name).eq("sean"))
        .load(&connection).unwrap().collect::<Vec<_>>());
    assert_eq!(vec![tess], users.filter(my_lower(name).eq("tess"))
        .load(&connection).unwrap().collect::<Vec<_>>());
}
