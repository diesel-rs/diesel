use schema::connection_without_transaction;
use diesel::*;
use diesel::expression::dsl::sql;

table! {
    auto_time {
        id -> Integer,
        n -> Integer,
        updated_at -> Timestamp,
    }
}

#[test]
#[cfg(feature = "postgres")]
fn managing_updated_at_for_table() {
    use self::auto_time::columns::*;
    use self::auto_time::table as auto_time;
    use diesel::pg::types::date_and_time::PgTimestamp;

    // transactions have frozen time, so we can't use them
    let connection = connection_without_transaction();
    connection.execute("CREATE TABLE auto_time (
        id SERIAL PRIMARY KEY,
        n INTEGER,
        updated_at TIMESTAMP
    );").unwrap();
    connection.execute("SELECT diesel_manage_updated_at('auto_time');").unwrap();

    connection.execute("INSERT INTO auto_time (n) VALUES (2), (1), (5);").unwrap();
    let result = select(sql("COUNT(*) FROM auto_time WHERE updated_at IS NULL"))
        .get_result::<i64>(&connection);
    assert_eq!(Ok(3), result);

    connection.execute("UPDATE auto_time SET n = n + 1 WHERE true;").unwrap();
    let result = select(sql("COUNT(*) FROM auto_time WHERE updated_at IS NULL"))
        .get_result::<i64>(&connection);
    assert_eq!(Ok(0), result);

    let query = auto_time.find(2).select(updated_at);
    let old_time: PgTimestamp = query.first(&connection).unwrap();
    update(auto_time.find(2)).set(n.eq(0)).execute(&connection).unwrap();
    let new_time: PgTimestamp = query.first(&connection).unwrap();
    assert!(old_time < new_time);

    // clean up because we aren't in a transaction
    connection.execute("DROP TABLE auto_time;").unwrap();
}
