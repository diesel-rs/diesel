use crate::schema::*;
use diesel::pg::{CopyFormat, CopyHeader};
use diesel::prelude::*;
use std::io::Read;

#[test]
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

#[test]
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

#[test]
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

#[test]
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

#[test]
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

#[test]
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
        .load::<(String, Option<String>)>(conn)
        .unwrap();

    assert_eq!(users[0], ("Sean".to_owned(), None));
    assert_eq!(users[1], ("Tess".to_owned(), Some("green".into())));
}

#[test]
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
        .load::<(String, Option<String>)>(conn)
        .unwrap();

    assert_eq!(users[0], ("Sean".to_owned(), None));
    assert_eq!(users[1], ("Tess".to_owned(), Some("green".into())));
}

#[test]
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
        .load::<(String, Option<String>)>(conn)
        .unwrap();

    assert_eq!(users[0], ("Sean".to_owned(), None));
    assert_eq!(users[1], ("Tess".to_owned(), Some("green".into())));
}

#[test]
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

#[test]
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
    // default is text
    let mut copy = diesel::copy_to(users::table).load_raw(conn).unwrap();
    copy.read_to_string(&mut out).unwrap();
    assert_eq!(out, "1\tSean\t\\N\n2\tTess\t\\N\n");
}

#[test]
fn copy_to_csv_all_options() {
    let conn = &mut connection_with_sean_and_tess_in_users_table();
    let mut out = String::new();
    let mut copy = diesel::copy_to(users::table)
        .with_format(CopyFormat::Csv)
        .with_freeze(true)
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

#[test]
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
