use crate::schema::*;
use diesel::*;

#[test]
fn insert_from_table() {
    use crate::schema::posts::dsl::*;
    let conn = &mut connection_with_sean_and_tess_in_users_table();
    insert_into(posts)
        .values(users::table)
        .into_columns((user_id, title, body))
        .execute(conn)
        .unwrap();

    let data = posts
        .select((user_id, title, body))
        .order(user_id)
        .load(conn);
    let expected = vec![
        (1, String::from("Sean"), None::<String>),
        (2, String::from("Tess"), None),
    ];
    assert_eq!(Ok(expected), data);
}

#[test]
fn insert_from_table_reference() {
    use crate::schema::posts::dsl::*;
    let conn = &mut connection_with_sean_and_tess_in_users_table();
    insert_into(posts)
        .values(&users::table)
        .into_columns((user_id, title, body))
        .execute(conn)
        .unwrap();

    let data = posts
        .select((user_id, title, body))
        .order(user_id)
        .load(conn);
    let expected = vec![
        (1, String::from("Sean"), None::<String>),
        (2, String::from("Tess"), None),
    ];
    assert_eq!(Ok(expected), data);
}

#[test]
fn insert_from_select() {
    use crate::schema::posts::dsl::*;
    use crate::schema::users::dsl::{id, name, users};

    let conn = &mut connection_with_sean_and_tess_in_users_table();
    users
        .select((id, name.concat(" says hi")))
        .insert_into(posts)
        .into_columns((user_id, title))
        .execute(conn)
        .unwrap();

    let data = posts
        .select(title)
        .order(title)
        .load::<String>(conn)
        .unwrap();
    let expected = vec!["Sean says hi", "Tess says hi"];
    assert_eq!(expected, data);
}

#[test]
fn insert_from_select_reference() {
    use crate::schema::posts::dsl::*;
    use crate::schema::users::dsl::{id, name, users};

    let conn = &mut connection_with_sean_and_tess_in_users_table();
    let select = users.select((id, name.concat(" says hi")));
    insert_into(posts)
        .values(&select)
        .into_columns((user_id, title))
        .execute(conn)
        .unwrap();

    let data = posts
        .select(title)
        .order(title)
        .load::<String>(conn)
        .unwrap();
    let expected = vec!["Sean says hi", "Tess says hi"];
    assert_eq!(expected, data);
}

#[test]
fn insert_from_boxed() {
    use crate::schema::posts::dsl::*;
    use crate::schema::users::dsl::{id, name, users};

    let conn = &mut connection_with_sean_and_tess_in_users_table();
    users
        .select((id, name.concat(" says hi")))
        .into_boxed()
        .insert_into(posts)
        .into_columns((user_id, title))
        .execute(conn)
        .unwrap();

    let data = posts
        .select(title)
        .order(title)
        .load::<String>(conn)
        .unwrap();
    let expected = vec!["Sean says hi", "Tess says hi"];
    assert_eq!(expected, data);
}

#[test]
fn insert_from_boxed_reference() {
    use crate::schema::posts::dsl::*;
    use crate::schema::users::dsl::{id, name, users};

    let conn = &mut connection_with_sean_and_tess_in_users_table();
    let select = users.select((id, name.concat(" says hi"))).into_boxed();
    insert_into(posts)
        .values(&select)
        .into_columns((user_id, title))
        .execute(conn)
        .unwrap();

    let data = posts
        .select(title)
        .order(title)
        .load::<String>(conn)
        .unwrap();
    let expected = vec!["Sean says hi", "Tess says hi"];
    assert_eq!(expected, data);
}

#[test]
#[cfg(feature = "sqlite")]
fn insert_or_ignore_with_select() {
    use crate::schema::posts::dsl::*;
    use crate::schema::users::dsl::{id, name, users};

    let conn = &mut connection_with_sean_and_tess_in_users_table();
    sql_query("CREATE UNIQUE INDEX foo ON posts (user_id)")
        .execute(conn)
        .unwrap();

    insert_or_ignore_into(posts)
        .values(users.select((id, name.concat(" says hi"))))
        .into_columns((user_id, title))
        .execute(conn)
        .unwrap();
    insert_or_ignore_into(posts)
        .values(users.select((id, name.concat(" says bye"))))
        .into_columns((user_id, title))
        .execute(conn)
        .unwrap();

    let data = posts.select(title).load::<String>(conn).unwrap();
    let expected = vec!["Sean says hi", "Tess says hi"];
    assert_eq!(expected, data);
}

#[test]
#[cfg(feature = "sqlite")]
fn insert_or_replace_with_select() {
    use crate::schema::posts::dsl::*;
    use crate::schema::users::dsl::{id, name, users};

    let conn = &mut connection_with_sean_and_tess_in_users_table();
    sql_query("CREATE UNIQUE INDEX foo ON posts (user_id)")
        .execute(conn)
        .unwrap();

    replace_into(posts)
        .values(users.select((id, name.concat(" says hi"))))
        .into_columns((user_id, title))
        .execute(conn)
        .unwrap();
    replace_into(posts)
        .values(users.select((id, name.concat(" says bye"))))
        .into_columns((user_id, title))
        .execute(conn)
        .unwrap();

    let data = posts.select(title).load::<String>(conn).unwrap();
    let expected = vec!["Sean says bye", "Tess says bye"];
    assert_eq!(expected, data);
}

#[test]
#[cfg(feature = "mysql")]
// We can't share the test with SQLite because it modifies
// schema, but we can at least make sure the query is *syntactically* valid.
fn insert_or_ignore_with_select() {
    use crate::schema::posts::dsl::*;
    use crate::schema::users::dsl::{id, name, users};

    let conn = &mut connection_with_sean_and_tess_in_users_table();

    insert_or_ignore_into(posts)
        .values(users.select((id, name.concat(" says hi"))))
        .into_columns((user_id, title))
        .execute(conn)
        .unwrap();

    let data = posts.select(title).load::<String>(conn).unwrap();
    let expected = vec!["Sean says hi", "Tess says hi"];
    assert_eq!(expected, data);
}

#[test]
#[cfg(feature = "mysql")]
// We can't share the test with SQLite because it modifies
// schema, but we can at least make sure the query is *syntactically* valid.
fn insert_or_replace_with_select() {
    use crate::schema::posts::dsl::*;
    use crate::schema::users::dsl::{id, name, users};

    let conn = &mut connection_with_sean_and_tess_in_users_table();

    replace_into(posts)
        .values(users.select((id, name.concat(" says hi"))))
        .into_columns((user_id, title))
        .execute(conn)
        .unwrap();

    let data = posts.select(title).load::<String>(conn).unwrap();
    let expected = vec!["Sean says hi", "Tess says hi"];
    assert_eq!(expected, data);
}

#[test]
fn on_conflict_do_nothing_with_select() {
    use crate::schema::posts::dsl::*;
    use crate::schema::users::dsl::{id, name, users};

    let mut conn = if cfg!(feature = "mysql") {
        let mut conn = connection_without_transaction();
        sql_query(
            "CREATE TEMPORARY TABLE posts (\
                 id INTEGER PRIMARY KEY AUTO_INCREMENT,\
                 user_id INTEGER NOT NULL,\
                 title VARCHAR(200) NOT NULL,\
                 body TEXT\
             )",
        )
        .execute(&mut conn)
        .unwrap();
        sql_query("CREATE UNIQUE INDEX  index_on_title ON posts (title)")
            .execute(&mut conn)
            .unwrap();
        conn.begin_test_transaction().unwrap();
        insert_sean_and_tess_into_users_table(&mut conn);
        conn
    } else {
        let mut conn = connection_with_sean_and_tess_in_users_table();
        sql_query("CREATE UNIQUE INDEX  index_on_title ON posts (title)")
            .execute(&mut conn)
            .unwrap();
        conn
    };

    let conn = &mut conn;

    let query = users
        .select((id, name.concat(" says hi")))
        .filter(id.ge(0)) // Sqlite needs a where clause
        .insert_into(posts)
        .into_columns((user_id, title))
        .on_conflict_do_nothing();

    let inserted_rows = query.execute(conn).unwrap();
    assert_eq!(2, inserted_rows);
    let inserted_rows = query.execute(conn).unwrap();
    if cfg!(feature = "mysql") {
        assert_eq!(2, inserted_rows);
    } else {
        assert_eq!(0, inserted_rows);
    }

    let data = posts
        .select(title)
        .order(title)
        .load::<String>(conn)
        .unwrap();
    let expected = vec!["Sean says hi", "Tess says hi"];
    assert_eq!(expected, data);
}

#[test]
fn on_conflict_do_update_with_select() {
    use crate::schema::posts::dsl::*;
    use crate::schema::users::dsl::{id, name, users};

    let mut conn = if cfg!(feature = "mysql") {
        let mut conn = connection_without_transaction();
        sql_query(
            "CREATE TEMPORARY TABLE posts (\
                 id INTEGER PRIMARY KEY AUTO_INCREMENT,\
                 user_id INTEGER NOT NULL,\
                 title VARCHAR(200) NOT NULL,\
                 body TEXT\
             )",
        )
        .execute(&mut conn)
        .unwrap();
        sql_query("CREATE UNIQUE INDEX  index_on_title ON posts (title)")
            .execute(&mut conn)
            .unwrap();
        conn.begin_test_transaction().unwrap();
        insert_sean_and_tess_into_users_table(&mut conn);
        conn
    } else {
        let mut conn = connection_with_sean_and_tess_in_users_table();
        sql_query("CREATE UNIQUE INDEX  index_on_title ON posts (title)")
            .execute(&mut conn)
            .unwrap();
        conn
    };

    let conn = &mut conn;

    #[cfg(any(feature = "postgres", feature = "sqlite"))]
    let target = title;
    #[cfg(feature = "mysql")]
    let target = diesel::dsl::DuplicatedKeys;

    let query = users
        .select((id, name.concat(" says hi")))
        .filter(id.ge(0)) // exists because sqlite needs a where clause
        .insert_into(posts)
        .into_columns((user_id, title))
        .on_conflict(target)
        .do_update()
        .set(body.eq("updated"));

    query.execute(conn).unwrap();

    insert_into(users)
        .values(name.eq("Ruby"))
        .execute(conn)
        .unwrap();

    query.execute(conn).unwrap();

    let data = posts
        .select((title, body))
        .order_by(title)
        .load(conn)
        .unwrap();
    let expected = vec![
        (String::from("Ruby says hi"), None),
        (String::from("Sean says hi"), Some(String::from("updated"))),
        (String::from("Tess says hi"), Some(String::from("updated"))),
    ];
    assert_eq!(expected, data);
}

#[test]
fn on_conflict_do_update_with_boxed_select() {
    use crate::schema::posts::dsl::*;
    use crate::schema::users::dsl::{id, name, users};

    let mut conn = if cfg!(feature = "mysql") {
        let mut conn = connection_without_transaction();
        sql_query(
            "CREATE TEMPORARY TABLE posts (\
                 id INTEGER PRIMARY KEY AUTO_INCREMENT,\
                 user_id INTEGER NOT NULL,\
                 title VARCHAR(200) NOT NULL,\
                 body TEXT\
             )",
        )
        .execute(&mut conn)
        .unwrap();
        sql_query("CREATE UNIQUE INDEX  index_on_title ON posts (title)")
            .execute(&mut conn)
            .unwrap();
        conn.begin_test_transaction().unwrap();
        insert_sean_and_tess_into_users_table(&mut conn);
        conn
    } else {
        let mut conn = connection_with_sean_and_tess_in_users_table();
        sql_query("CREATE UNIQUE INDEX  index_on_title ON posts (title)")
            .execute(&mut conn)
            .unwrap();
        conn
    };

    let conn = &mut conn;

    #[cfg(any(feature = "postgres", feature = "sqlite"))]
    let target = title;
    #[cfg(feature = "mysql")]
    let target = diesel::dsl::DuplicatedKeys;

    users
        .select((id, name.concat(" says hi")))
        .order(id)
        .into_boxed()
        .insert_into(posts)
        .into_columns((user_id, title))
        .on_conflict(target)
        .do_update()
        .set(body.eq("updated"))
        .execute(conn)
        .unwrap();

    insert_into(users)
        .values(name.eq("Ruby"))
        .execute(conn)
        .unwrap();

    users
        .select((id, name.concat(" says hi")))
        .order(id)
        .into_boxed()
        .insert_into(posts)
        .into_columns((user_id, title))
        .on_conflict(target)
        .do_update()
        .set(body.eq("updated"))
        .execute(conn)
        .unwrap();

    let data = posts.select((title, body)).load(conn).unwrap();
    let expected = vec![
        (String::from("Sean says hi"), Some(String::from("updated"))),
        (String::from("Tess says hi"), Some(String::from("updated"))),
        (String::from("Ruby says hi"), None),
    ];
    assert_eq!(expected, data);
}
