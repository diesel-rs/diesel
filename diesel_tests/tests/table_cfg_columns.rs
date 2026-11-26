use crate::schema::connection;
use diesel::RunQueryDsl;
use diesel::connection::SimpleConnection;
use diesel::query_builder::AsQuery;
use diesel::sqlite::Sqlite;

table! {
    #[sql_name = "gated_columns_users"]
    users (id) {
        id -> Integer,
        #[cfg(all())]
        ldap_dn -> Binary,
        name -> Text,
        password_hash -> Text,
    }
}

#[cfg(not(miri))] // ffi string access
#[diesel_test_helper::test]
fn gated_column_keeps_its_declared_position_in_the_default_selection() {
    let connection = &mut connection();
    connection
        .batch_execute(
            "CREATE TABLE gated_columns_users (id INTEGER PRIMARY KEY, ldap_dn BLOB NOT NULL, name TEXT NOT NULL, password_hash TEXT NOT NULL);
             INSERT INTO gated_columns_users VALUES (1, x'636e3d616c696365', 'alice', 'secret-hash');",
        )
        .unwrap();

    let sql = diesel::debug_query::<Sqlite, _>(&users::table.as_query()).to_string();
    assert_eq!(
        sql,
        "SELECT `gated_columns_users`.`id`, `gated_columns_users`.`ldap_dn`, `gated_columns_users`.`name`, `gated_columns_users`.`password_hash` FROM `gated_columns_users` -- binds: []"
    );

    let row: (i32, Vec<u8>, String, String) = users::table.first(connection).unwrap();
    assert_eq!(
        row,
        (
            1,
            b"cn=alice".to_vec(),
            "alice".to_string(),
            "secret-hash".to_string()
        )
    );
}
