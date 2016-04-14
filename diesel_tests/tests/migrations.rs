#[cfg(feature = "postgres")]
use diesel::types::Bool;
#[cfg(feature = "sqlite")]
use diesel::types::BigInt;
use diesel::{select, LoadDsl};
use diesel::result::QueryResult;
use diesel::connection::Connection;
use diesel::expression::dsl::sql;
use diesel::migrations::revert_migration;
use schema::*;

#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
pub fn table_exists(conn: &TestConnection) -> bool {
    select(sql::<BigInt>("count(*) \
             FROM sqlite_master \
             WHERE type = 'table' \
             AND name = 'DUMMY';"))
        .get_result::<i64>(conn)
        .unwrap() == 1
}

#[cfg(all(not(feature = "sqlite"), feature = "postgres"))]
pub fn table_exists(conn: &TestConnection) -> bool {
    select(sql::<Bool>("EXISTS \
            (SELECT 1 \
             FROM information_schema.tables \
             WHERE table_name = 'dummy')"))
        .get_result(conn)
        .unwrap()
}

create_migrations_module!("migrations_test");

#[test]
fn test_migrations_macro() {
    let connection = connection_without_transaction();
    migrations::run(&connection);
    let exists = table_exists(&connection);
    assert!(exists);
    if exists {
        revert_migration(&connection,
                         &migrations::get_migrations()[0],
                         &mut ::std::io::stdout());
    }
}
