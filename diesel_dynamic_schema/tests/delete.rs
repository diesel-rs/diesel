use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::*;

#[test]
fn delete_one_record() {
    let connection = &mut crate::establish_connection();
    crate::create_user_table(connection);
    sql_query("INSERT INTO users (id, name, hair_color) VALUES (42, 'Sean', 'black'), (43, 'Tess', 'black')")
        .execute(connection)
        .unwrap();

    let users = diesel_dynamic_schema::table("users");
    let id = users.column::<Integer, _>("id");

    diesel::delete(users.filter(id.eq(42)))
        .execute(connection)
        .unwrap();

    let name = users.column::<Text, _>("name");
    let names = users.select(name).load::<String>(connection);
    assert_eq!(Ok(vec!["Tess".into()]), names);
}

// #[test]
// fn truncate_table() {
//     let connection = &mut crate::establish_connection();
//     crate::create_user_table(connection);
//     sql_query("INSERT INTO users (id, name, hair_color) VALUES (42, 'Sean', 'black'), (43, 'Tess', 'black')")
//         .execute(connection)
//         .unwrap();

//     let users = diesel_dynamic_schema::table("users");

//     diesel::delete(users).execute(connection).unwrap();

//     let name = users.column::<Text, _>("name");
//     let names = users.select(name).load::<String>(connection);
//     assert_eq!(Ok(Vec::new()), names);
// }
