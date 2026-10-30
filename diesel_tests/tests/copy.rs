use crate::schema::*;
use diesel::connection::SimpleConnection;
use diesel::pg::{CopyFormat, CopyHeader};
use diesel::prelude::*;
use diesel::sql_types::Integer;
use std::io::Read;

#[diesel_test_helper::test]
fn copy_from_csv() {
    let conn = &mut connection();

    let user_count_query = users::table.count();
    let users = user_count_query.get_result::<i64>(conn).unwrap();
    assert_eq!(users, 0);

    let count = diesel::copy_from(users::table)
        .from_raw_data(users::table, |copy| {
            writeln!(copy, "1,Sean,").unwrap();
            writeln!(copy, "2,Tess,").unwrap();
            diesel::QueryResult::Ok(())
        })
        .with_format(CopyFormat::Csv)
        .execute(conn)
        .unwrap();

    assert_eq!(count, 2);

    let users = user_count_query.get_result::<i64>(conn).unwrap();
    assert_eq!(users, 2);
}

#[diesel_test_helper::test]
fn copy_from_text() {
    let conn = &mut connection();

    let user_count_query = users::table.count();
    let users = user_count_query.get_result::<i64>(conn).unwrap();
    assert_eq!(users, 0);

    let count = diesel::copy_from(users::table)
        .from_raw_data(users::table, |copy| {
            writeln!(copy, "1\tSean\t").unwrap();
            writeln!(copy, "2\tTess\t").unwrap();
            diesel::QueryResult::Ok(())
        })
        .with_format(CopyFormat::Text)
        .execute(conn)
        .unwrap();

    assert_eq!(count, 2);

    let users = user_count_query.get_result::<i64>(conn).unwrap();
    assert_eq!(users, 2);

    // default is text
    let count = diesel::copy_from(users::table)
        .from_raw_data(users::table, |copy| {
            writeln!(copy, "3\tSean\t").unwrap();
            writeln!(copy, "4\tTess\t").unwrap();
            diesel::QueryResult::Ok(())
        })
        .execute(conn)
        .unwrap();

    assert_eq!(count, 2);

    let users = user_count_query.get_result::<i64>(conn).unwrap();
    assert_eq!(users, 4);
}

#[diesel_test_helper::test]
fn copy_from_allows_to_return_error() {
    // use a connection without transaction here as otherwise
    // we fail the last query
    let conn = &mut connection_without_transaction();

    let user_count_query = users::table.count();
    let users = user_count_query.get_result::<i64>(conn).unwrap();
    assert_eq!(users, 0);

    let res = diesel::copy_from(users::table)
        .from_raw_data(users::table, |copy| {
            writeln!(copy, "1,Sean,").unwrap();
            diesel::QueryResult::Err(diesel::result::Error::RollbackTransaction)
        })
        .with_format(CopyFormat::Csv)
        .execute(conn);

    assert!(res.is_err());

    let users = user_count_query.get_result::<i64>(conn).unwrap();
    assert_eq!(users, 0);
}

#[diesel_test_helper::test]
fn copy_from_with_columns() {
    let conn = &mut connection();

    let user_count_query = users::table.count();
    let users = user_count_query.get_result::<i64>(conn).unwrap();
    assert_eq!(users, 0);

    let count = diesel::copy_from(users::table)
        .from_raw_data((users::name, users::id), |copy| {
            writeln!(copy, "Sean\t1").unwrap();
            writeln!(copy, "Tess\t2").unwrap();
            diesel::QueryResult::Ok(())
        })
        .with_format(CopyFormat::Text)
        .execute(conn)
        .unwrap();

    assert_eq!(count, 2);
    let users = user_count_query.get_result::<i64>(conn).unwrap();
    assert_eq!(users, 2);
}

#[diesel_test_helper::test]
fn copy_from_csv_all_options() {
    let conn = &mut connection();

    let user_count_query = users::table.count();
    let users = user_count_query.get_result::<i64>(conn).unwrap();
    assert_eq!(users, 0);

    let count = diesel::copy_from(users::table)
        .from_raw_data((users::id, users::name, users::hair_color), |copy| {
            // need to send the header here
            // as we set header = match below
            writeln!(copy, "id;name;hair_color").unwrap();
            writeln!(copy, "1;Sean;<!NULL!>").unwrap();
            writeln!(copy, "2;Tess;<!NULL!>").unwrap();
            diesel::QueryResult::Ok(())
        })
        .with_format(CopyFormat::Csv)
        .with_freeze(false)
        .with_delimiter(';')
        .with_null("<!NULL!>")
        .with_quote('"')
        .with_escape('\\')
        .with_header(CopyHeader::Set(true))
        // that option is new in postgres 16,
        // so just skip testing it for now
        //.set_default("default")
        .execute(conn)
        .unwrap();

    assert_eq!(count, 2);
    let users = user_count_query.get_result::<i64>(conn).unwrap();
    assert_eq!(users, 2);
}

#[diesel_test_helper::test]
fn copy_from_from_insertable_struct() {
    let conn = &mut connection();

    #[derive(Insertable)]
    #[diesel(table_name = users)]
    #[diesel(treat_none_as_default_value = false)]
    struct NewUser {
        name: &'static str,
        hair_color: Option<&'static str>,
    }

    let user_count_query = users::table.count();
    let users = user_count_query.get_result::<i64>(conn).unwrap();
    assert_eq!(users, 0);

    let users = vec![
        NewUser {
            name: "Sean",
            hair_color: None,
        },
        NewUser {
            name: "Tess",
            hair_color: Some("green"),
        },
    ];
    let count = diesel::copy_from(users::table)
        .from_insertable(&users)
        .execute(conn)
        .unwrap();
    assert_eq!(count, 2);
    let user_count = user_count_query.get_result::<i64>(conn).unwrap();
    assert_eq!(user_count, 2);
    let users = users::table
        .select((users::name, users::hair_color))
        .order(users::id)
        .load::<(String, Option<String>)>(conn)
        .unwrap();

    assert_eq!(users[0], ("Sean".to_owned(), None));
    assert_eq!(users[1], ("Tess".to_owned(), Some("green".into())));
}

#[diesel_test_helper::test]
fn copy_from_from_insertable_tuple() {
    let conn = &mut connection();

    let user_count_query = users::table.count();
    let users = user_count_query.get_result::<i64>(conn).unwrap();
    assert_eq!(users, 0);

    let users = vec![
        (users::name.eq("Sean"), users::hair_color.eq(None)),
        (users::name.eq("Tess"), users::hair_color.eq(Some("green"))),
    ];
    let count = diesel::copy_from(users::table)
        .from_insertable(&users)
        .execute(conn)
        .unwrap();
    assert_eq!(count, 2);
    let user_count = user_count_query.get_result::<i64>(conn).unwrap();
    assert_eq!(user_count, 2);
    let users = users::table
        .select((users::name, users::hair_color))
        .order(users::id)
        .load::<(String, Option<String>)>(conn)
        .unwrap();

    assert_eq!(users[0], ("Sean".to_owned(), None));
    assert_eq!(users[1], ("Tess".to_owned(), Some("green".into())));
}

#[diesel_test_helper::test]
fn copy_from_from_insertable_vec() {
    let conn = &mut connection();

    let user_count_query = users::table.count();
    let users = user_count_query.get_result::<i64>(conn).unwrap();
    assert_eq!(users, 0);

    let users = vec![
        (users::name.eq("Sean"), users::hair_color.eq(None)),
        (users::name.eq("Tess"), users::hair_color.eq(Some("green"))),
    ];
    let count = diesel::copy_from(users::table)
        .from_insertable(users)
        .execute(conn)
        .unwrap();
    assert_eq!(count, 2);
    let user_count = user_count_query.get_result::<i64>(conn).unwrap();
    assert_eq!(user_count, 2);
    let users = users::table
        .select((users::name, users::hair_color))
        .order(users::id)
        .load::<(String, Option<String>)>(conn)
        .unwrap();

    assert_eq!(users[0], ("Sean".to_owned(), None));
    assert_eq!(users[1], ("Tess".to_owned(), Some("green".into())));
}

#[diesel_test_helper::test]
fn copy_to_csv() {
    let conn = &mut connection_with_sean_and_tess_in_users_table();

    let mut out = String::new();
    let mut copy = diesel::copy_to(users::table)
        .with_format(CopyFormat::Csv)
        .load_raw(conn)
        .unwrap();
    copy.read_to_string(&mut out).unwrap();

    assert_eq!(out, "1,Sean,\n2,Tess,\n");
}

#[diesel_test_helper::test]
fn copy_to_text() {
    let conn = &mut connection_with_sean_and_tess_in_users_table();
    {
        let mut out = String::new();
        let mut copy = diesel::copy_to(users::table)
            .with_format(CopyFormat::Text)
            .load_raw(conn)
            .unwrap();
        copy.read_to_string(&mut out).unwrap();
        assert_eq!(out, "1\tSean\t\\N\n2\tTess\t\\N\n");
    }

    let mut out = String::new();
    let mut copy = diesel::copy_to(users::table).load_raw(conn).unwrap();
    copy.read_to_string(&mut out).unwrap();
    assert_eq!(out, "1\tSean\t\\N\n2\tTess\t\\N\n");
}

#[diesel_test_helper::test]
fn copy_to_csv_all_options() {
    let conn = &mut connection_with_sean_and_tess_in_users_table();
    let mut out = String::new();
    let mut copy = diesel::copy_to(users::table)
        .with_format(CopyFormat::Csv)
        .with_delimiter(';')
        .with_quote('"')
        .with_escape('\\')
        .with_null("<!NULL!>")
        .with_header(true)
        .load_raw(conn)
        .unwrap();

    copy.read_to_string(&mut out).unwrap();
    assert_eq!(
        out,
        "id;name;hair_color\n1;Sean;<!NULL!>\n2;Tess;<!NULL!>\n"
    );
}

#[diesel_test_helper::test]
fn copy_to_queryable() {
    let conn = &mut connection_with_sean_and_tess_in_users_table();

    #[derive(Queryable, Selectable)]
    #[diesel(table_name = users)]
    struct User {
        name: String,
        hair_color: Option<String>,
    }

    let out = diesel::copy_to(users::table)
        .load::<User, _>(conn)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    assert_eq!(out[0].name, "Sean");
    assert_eq!(out[0].hair_color, None);
    assert_eq!(out[1].name, "Tess");
    assert_eq!(out[1].hair_color, None);

    // some query afterwards
    let name = users::table
        .select(users::name)
        .filter(users::name.eq("Sean"))
        .get_result::<String>(conn)
        .unwrap();
    assert_eq!(name, "Sean");
}

#[diesel_test_helper::test]
fn copy_with_null_injection() {
    let conn = &mut connection_with_sean_and_tess_in_users_table();

    let mut buf = diesel::copy_to(users::table)
        .with_format(CopyFormat::Csv)
        .with_null("abc ' efg")
        .load_raw(conn)
        .unwrap();
    let mut out = String::new();
    buf.read_to_string(&mut out).unwrap();
    assert_eq!(out, "1,Sean,abc ' efg\n2,Tess,abc ' efg\n");
}

#[diesel_test_helper::test]
fn copy_with_delimiter_injection() {
    let conn = &mut connection_with_sean_and_tess_in_users_table();
    let mut buf = diesel::copy_to(users::table)
        .with_format(CopyFormat::Csv)
        .with_delimiter('\'')
        .load_raw(conn)
        .unwrap();
    let mut out = String::new();
    buf.read_to_string(&mut out).unwrap();
    assert_eq!(out, "1'Sean'\n2'Tess'\n");
}

#[diesel_test_helper::test]
fn copy_with_delimiter_injection2() {
    let conn = &mut connection_with_sean_and_tess_in_users_table();
    let mut buf = diesel::copy_to(users::table)
        .with_format(CopyFormat::Csv)
        .with_delimiter('\\')
        .load_raw(conn)
        .unwrap();
    let mut out = String::new();
    buf.read_to_string(&mut out).unwrap();
    assert_eq!(out, "1\\Sean\\\n2\\Tess\\\n");
}

#[diesel_test_helper::test]
fn copy_with_escape_injection() {
    let conn = &mut connection_with_sean_and_tess_in_users_table();

    let mut buf = diesel::copy_to(users::table)
        .with_format(CopyFormat::Csv)
        .with_escape('\'')
        .load_raw(conn)
        .unwrap();
    let mut out = String::new();
    buf.read_to_string(&mut out).unwrap();
    assert_eq!(out, "1,Sean,\n2,Tess,\n");
}

#[diesel_test_helper::test]
fn copy_with_quote_injection() {
    let conn = &mut connection_with_sean_and_tess_in_users_table();
    let mut buf = diesel::copy_to(users::table)
        .with_format(CopyFormat::Csv)
        .with_quote('\'')
        .load_raw(conn)
        .unwrap();
    let mut out = String::new();
    buf.read_to_string(&mut out).unwrap();
    assert_eq!(out, "1,Sean,\n2,Tess,\n");
}

#[diesel_test_helper::test]
fn cached_query_after_copy_to() {
    let conn = &mut connection_with_sean_and_tess_in_users_table();
    let q = || diesel::select(1.into_sql::<diesel::sql_types::Integer>());
    assert_eq!(q().get_result::<i32>(conn).unwrap(), 1);
    let rows: Vec<User> = diesel::copy_to(users::table)
        .load::<User, _>(conn)
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        q().get_result::<i32>(conn).map_err(|e| e.to_string()),
        Ok(1)
    );
}

#[diesel_test_helper::test]
fn execute_after_copy_to_does_not_hang() {
    let conn = &mut connection_with_sean_and_tess_in_users_table();
    let _rows: Vec<User> = diesel::copy_to(users::table)
        .load::<User, _>(conn)
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    let r = diesel::update(users::table)
        .set(users::name.eq("x"))
        .execute(conn)
        .unwrap();
    assert_eq!(r, 2);
}

#[diesel_test_helper::test]
fn copy_to_load_dropped_early() {
    let conn = &mut connection();
    // add a temporary copy of the users table to prevent polluting the main table
    // (which causes other tests to fail as the order is not deterministic anymore)
    diesel::sql_query("CREATE TEMPORARY TABLE users (LIKE public.users INCLUDING ALL)")
        .execute(conn)
        .unwrap();

    diesel::sql_query(
        "INSERT INTO users (id, name) SELECT g, repeat('x', 200) FROM generate_series(1, $1) g",
    )
    .bind::<Integer, _>(50_000)
    .execute(conn)
    .unwrap();

    // Prepare and cache the statement used after the COPY: an uncached statement
    // goes through `PQprepare`, which silently leaves an unfinished COPY OUT itself
    assert_eq!(
        diesel::select(1.into_sql::<diesel::sql_types::Integer>())
            .get_result::<i32>(conn)
            .map_err(|e| e.to_string()),
        Ok(1)
    );

    let first = diesel::copy_to(users::table)
        .load::<User, _>(conn)
        .unwrap()
        .next();
    assert!(first.is_some());

    assert_eq!(
        diesel::select(1.into_sql::<diesel::sql_types::Integer>())
            .get_result::<i32>(conn)
            .map_err(|e| e.to_string()),
        Ok(1)
    );
}

#[diesel_test_helper::test]
fn copy_to_load_raw_dropped_early() {
    let conn = &mut connection();
    // add a temporary copy of the users table to prevent polluting the main table
    // (which causes other tests to fail as the order is not deterministic anymore)
    diesel::sql_query("CREATE TEMPORARY TABLE users (LIKE public.users INCLUDING ALL)")
        .execute(conn)
        .unwrap();

    diesel::sql_query(
        "INSERT INTO users (id, name) SELECT g, repeat('x', 200) FROM generate_series(1, $1) g",
    )
    .bind::<Integer, _>(50_000)
    .execute(conn)
    .unwrap();

    // Prepare and cache the statement used after the COPY (see `copy_to_load_dropped_early`)
    assert_eq!(
        diesel::select(1.into_sql::<diesel::sql_types::Integer>())
            .get_result::<i32>(conn)
            .map_err(|e| e.to_string()),
        Ok(1)
    );

    {
        let mut raw = diesel::copy_to(users::table).load_raw(conn).unwrap();
        let mut buf = [0; 16];
        raw.read_exact(&mut buf).unwrap();
    }

    assert_eq!(
        diesel::select(1.into_sql::<diesel::sql_types::Integer>())
            .get_result::<i32>(conn)
            .map_err(|e| e.to_string()),
        Ok(1)
    );
}

#[diesel_test_helper::test]
fn copy_to_small_load_dropped_early() {
    // The whole COPY is already buffered by libpq when the iterator is dropped
    let conn = &mut connection_with_sean_and_tess_in_users_table();
    let q = || users::table.count();
    assert_eq!(
        q().get_result::<i64>(conn).map_err(|e| e.to_string()),
        Ok(2)
    );

    let first = diesel::copy_to(users::table)
        .load::<User, _>(conn)
        .unwrap()
        .next();
    assert!(first.is_some());

    assert_eq!(
        q().get_result::<i64>(conn).map_err(|e| e.to_string()),
        Ok(2)
    );
}

#[diesel_test_helper::test]
fn copy_to_load_raw_dropped_unread() {
    let conn = &mut connection_with_sean_and_tess_in_users_table();
    let q = || users::table.count();
    assert_eq!(
        q().get_result::<i64>(conn).map_err(|e| e.to_string()),
        Ok(2)
    );

    drop(diesel::copy_to(users::table).load_raw(conn).unwrap());

    assert_eq!(
        q().get_result::<i64>(conn).map_err(|e| e.to_string()),
        Ok(2)
    );
}

#[diesel_test_helper::test]
fn execute_after_copy_to_dropped_early_does_not_hang() {
    let mut conn = connection();
    // Use a session-local `users` table: if the connection hangs, its open
    // transaction must not hold row locks that block other tests
    diesel::sql_query("CREATE TEMPORARY TABLE users (LIKE public.users INCLUDING ALL)")
        .execute(&mut conn)
        .unwrap();
    diesel::sql_query("INSERT INTO users (id, name) VALUES (1, 'Sean'), (2, 'Tess')")
        .execute(&mut conn)
        .unwrap();
    let q = || diesel::update(users::table).set(users::name.eq("x"));
    assert_eq!(q().execute(&mut conn).map_err(|e| e.to_string()), Ok(2));

    // Run on a separate thread, a connection left in COPY OUT spins forever here
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let first = diesel::copy_to(users::table)
            .load::<User, _>(&mut conn)
            .unwrap()
            .next();
        assert!(first.is_some());
        let _ = tx.send(q().execute(&mut conn).map_err(|e| e.to_string()));
    });
    let res = rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("execute after a dropped COPY TO did not return within 10s");
    assert_eq!(res, Ok(2));
}

#[diesel_test_helper::test]
fn copy_to_fill_buf_after_eof() {
    use std::io::BufRead;

    let conn = &mut connection_with_sean_and_tess_in_users_table();

    {
        let mut raw = diesel::copy_to(users::table).load_raw(conn).unwrap();
        let mut out = Vec::new();
        raw.read_to_end(&mut out).unwrap();
        // `BufRead` allows to call `fill_buf` again after EOF
        assert_eq!(
            raw.fill_buf().map(|b| b.len()).map_err(|e| e.to_string()),
            Ok(0)
        );
        assert_eq!(
            raw.fill_buf().map(|b| b.len()).map_err(|e| e.to_string()),
            Ok(0)
        );
    }

    assert_eq!(
        diesel::select(1.into_sql::<diesel::sql_types::Integer>())
            .get_result::<i32>(conn)
            .map_err(|e| e.to_string()),
        Ok(1)
    );
}

#[diesel_test_helper::test]
fn copy_to_reports_error_during_copy() {
    let conn = &mut connection();
    // add a temporary copy of the users table to prevent polluting the main table
    // (which causes other tests to fail as the order is not deterministic anymore)
    diesel::sql_query("CREATE TEMPORARY TABLE users (LIKE public.users INCLUDING ALL)")
        .execute(conn)
        .unwrap();
    diesel::sql_query(
        "INSERT INTO users (id, name) SELECT g, repeat('x', 200) FROM generate_series(1, $1) g",
    )
    .bind::<Integer, _>(100_000)
    .execute(conn)
    .unwrap();

    // The error aborts the (test) transaction, so run the COPY in a savepoint
    let res = conn.transaction::<(), diesel::result::Error, _>(|conn| {
        diesel::sql_query("SET LOCAL statement_timeout = 300").execute(conn)?;
        let mut raw = diesel::copy_to(users::table).load_raw(conn)?;
        let mut head = [0; 16];
        raw.read_exact(&mut head).unwrap();
        // The server blocks on the full socket buffer and hits the timeout
        std::thread::sleep(std::time::Duration::from_secs(1));
        let mut rest = Vec::new();
        let err = raw.read_to_end(&mut rest).unwrap_err();
        Err(diesel::result::Error::DeserializationError(Box::new(err)))
    });

    let err = res.unwrap_err().to_string();
    assert!(err.contains("statement timeout"), "{err}");
    assert_eq!(
        diesel::select(1.into_sql::<diesel::sql_types::Integer>())
            .get_result::<i32>(conn)
            .map_err(|e| e.to_string()),
        Ok(1)
    );
}

#[diesel_test_helper::test]
fn copy_to_load_ends_after_error_during_copy() {
    let conn = &mut connection();
    // see `copy_to_reports_error_during_copy`
    diesel::sql_query("CREATE TEMPORARY TABLE users (LIKE public.users INCLUDING ALL)")
        .execute(conn)
        .unwrap();
    // '€' has no LATIN1 equivalent, so the server fails in the middle of the COPY
    diesel::sql_query(
        "INSERT INTO users (id, name) \
         SELECT g, CASE WHEN g = 5 THEN '€' ELSE 'x' END FROM generate_series(1, 10) g",
    )
    .execute(conn)
    .unwrap();

    // The error aborts the (test) transaction, so run the COPY in a savepoint
    let mut ok = 0;
    let mut errors = Vec::new();
    let _ = conn.transaction::<(), diesel::result::Error, _>(|conn| {
        diesel::sql_query("SET LOCAL client_encoding = 'LATIN1'").execute(conn)?;
        // `take` bounds the loop: the iterator must end on its own after the error
        for row in diesel::copy_to(users::table)
            .load::<User, _>(conn)?
            .take(10)
        {
            match row {
                Ok(_) => ok += 1,
                Err(e) => errors.push(e.to_string()),
            }
        }
        Err(diesel::result::Error::RollbackTransaction)
    });

    assert_eq!(ok, 4);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("no equivalent in encoding"),
        "{errors:?}"
    );
    assert_eq!(
        diesel::select(1.into_sql::<diesel::sql_types::Integer>())
            .get_result::<i32>(conn)
            .map_err(|e| e.to_string()),
        Ok(1)
    );
}

#[diesel_test_helper::test]
fn copy_to_null_does_not_null_rest_of_row() {
    diesel::table! {
        test_nullable (id) {
            id -> Integer,
            a -> Nullable<Integer>,
            b -> Nullable<Integer>,
        }
    }

    #[derive(Queryable, Selectable, Debug, PartialEq)]
    #[diesel(table_name = test_nullable)]
    struct Row {
        id: i32,
        a: Option<i32>,
        b: Option<i32>,
    }

    let conn = &mut connection();
    diesel::sql_query("CREATE TEMP TABLE test_nullable (id int4 PRIMARY KEY, a int4, b int4)")
        .execute(conn)
        .unwrap();
    diesel::sql_query("INSERT INTO test_nullable VALUES (1, NULL, 42)")
        .execute(conn)
        .unwrap();
    let rows: Vec<Row> = diesel::copy_to(test_nullable::table)
        .load::<Row, _>(conn)
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        rows,
        vec![Row {
            id: 1,
            a: None,
            b: Some(42)
        }]
    );
}

#[diesel_test_helper::test]
fn copy_to_load_null_combinations() {
    diesel::table! {
        test_null_combinations (id) {
            id -> Integer,
            a -> Nullable<Integer>,
            b -> Nullable<Text>,
            c -> Nullable<Binary>,
            d -> Nullable<BigInt>,
        }
    }

    #[derive(Queryable, Selectable, Insertable, Debug, PartialEq)]
    #[diesel(table_name = test_null_combinations)]
    struct Row {
        id: i32,
        a: Option<i32>,
        b: Option<String>,
        c: Option<Vec<u8>>,
        d: Option<i64>,
    }

    let conn = &mut connection();
    diesel::sql_query(
        "CREATE TEMP TABLE test_null_combinations \
         (id int4 PRIMARY KEY, a int4, b text, c bytea, d int8)",
    )
    .execute(conn)
    .unwrap();
    // every NULL / non-NULL combination, with empty (length 0) values next to NULL (length -1)
    let rows = (0..32)
        .map(|i| {
            let empty = i & 16 != 0;
            Row {
                id: i,
                a: (i & 1 != 0).then_some(-7),
                b: (i & 2 != 0).then(|| if empty { "" } else { "txt" }.to_owned()),
                c: (i & 4 != 0).then(|| if empty { vec![] } else { vec![0, 1, 255] }),
                d: (i & 8 != 0).then_some(i64::MIN),
            }
        })
        .collect::<Vec<_>>();
    diesel::insert_into(test_null_combinations::table)
        .values(&rows)
        .execute(conn)
        .unwrap();

    let mut copied = diesel::copy_to(test_null_combinations::table)
        .load::<Row, _>(conn)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    copied.sort_by_key(|r| r.id);
    assert_eq!(copied, rows);
}

#[diesel_test_helper::test]
fn copy_to_load_ends_after_connection_loss() {
    let conn = &mut connection();
    // see `copy_to_reports_error_during_copy`
    diesel::sql_query("CREATE TEMPORARY TABLE users (LIKE public.users INCLUDING ALL)")
        .execute(conn)
        .unwrap();
    diesel::sql_query(
        "INSERT INTO users (id, name) SELECT g, repeat('x', 200) FROM generate_series(1, $1) g",
    )
    .bind::<Integer, _>(100_000)
    .execute(conn)
    .unwrap();
    let pid = diesel::select(diesel::dsl::sql::<Integer>("pg_backend_pid()"))
        .get_result::<i32>(conn)
        .unwrap();

    let mut rows = diesel::copy_to(users::table).load::<User, _>(conn).unwrap();
    assert!(rows.next().unwrap().is_ok());

    let admin = &mut connection_without_transaction();
    diesel::sql_query("SELECT pg_terminate_backend($1)")
        .bind::<Integer, _>(pid)
        .execute(admin)
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(300));

    // Skipping errors must not loop forever: the iterator ends after the first error
    // (`take` bounds the loop, the table has 100_000 rows)
    let mut errors = 0;
    let read = rows
        .by_ref()
        .take(200_000)
        .inspect(|r| errors += usize::from(r.is_err()))
        .filter_map(Result::ok)
        .count();
    assert!(read < 100_000, "{read}");
    assert_eq!(errors, 1);
    assert!(rows.next().is_none());
}

#[diesel_test_helper::test]
fn copy_to_load_raw_reports_a_failed_copy_once() {
    use diesel::connection::SimpleConnection;
    use std::io::BufRead;
    diesel::table! {
        copy_to_failing_rows (id) {
            id -> Integer,
            value -> Text,
        }
    }

    let conn = &mut connection();
    conn.batch_execute(
        "CREATE TEMPORARY TABLE copy_to_failing_rows (id INTEGER NOT NULL, value TEXT NOT NULL); \
         INSERT INTO copy_to_failing_rows VALUES (1, 'a'), (2, 'b'), (3, '€'), (4, 'd'); \
         SET LOCAL client_encoding = 'LATIN1';",
    )
    .unwrap();

    let mut raw = diesel::copy_to(copy_to_failing_rows::table)
        .load_raw(conn)
        .unwrap();
    let mut out = Vec::new();
    let error = raw.read_to_end(&mut out).expect_err("the COPY to fail");
    assert!(error.to_string().contains("LATIN1"), "{error}");
    assert_eq!(out, b"1\ta\n2\tb\n");
    // A failed `COPY` is at its end: reading on reports the end, not another error
    assert_eq!(raw.fill_buf().map_err(|e| e.to_string()), Ok(&[][..]));
    assert_eq!(raw.fill_buf().map_err(|e| e.to_string()), Ok(&[][..]));
}

#[diesel_test_helper::test]
fn execute_after_panicking_copy_from_does_not_hang() {
    // cannot use a transaction as otherwise the transaction is in an error state
    // after the copy is cancelled
    let conn = &mut connection_without_transaction();
    conn.batch_execute(
        "CREATE TEMPORARY TABLE users(id SERIAL PRIMARY KEY, name TEXT NOT NULL, hair_color TEXT); \
         INSERT INTO users(name) VALUES ('John'), ('Jane');",
    )
    .unwrap();

    let p = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        diesel::copy_from(users::table)
            .from_raw_data(users::table, |_w| -> QueryResult<()> {
                panic!("callback panic")
            })
            .execute(&mut *conn)
    }));
    assert!(p.is_err());
    let r = diesel::update(users::table)
        .set(users::name.eq("x"))
        .execute(conn);
    assert_eq!(Ok(2), r);
}
