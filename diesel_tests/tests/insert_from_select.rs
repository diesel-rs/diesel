use crate::schema::*;
use diesel::*;

#[test]
fn insert_from_table() {
    use crate::schema::posts::dsl::*;
    let mut conn = connection_with_sean_and_tess_in_users_table();
    insert_into(posts)
        .values(users::table)
        .into_columns((user_id, title, body))
        .execute(&mut conn)
        .unwrap();

    let data = posts.select((user_id, title, body)).load(&mut conn);
    let expected = vec![
        (1, String::from("Sean"), None::<String>),
        (2, String::from("Tess"), None),
    ];
    assert_eq!(Ok(expected), data);
}

#[test]
fn insert_from_table_reference() {
    use crate::schema::posts::dsl::*;
    let mut conn = connection_with_sean_and_tess_in_users_table();
    insert_into(posts)
        .values(&users::table)
        .into_columns((user_id, title, body))
        .execute(&mut conn)
        .unwrap();

    let data = posts.select((user_id, title, body)).load(&mut conn);
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

    let mut conn = connection_with_sean_and_tess_in_users_table();
    users
        .select((id, name.concat(" says hi")))
        .insert_into(posts)
        .into_columns((user_id, title))
        .execute(&mut conn)
        .unwrap();

    let data = posts.select(title).load::<String>(&mut conn).unwrap();
    let expected = vec!["Sean says hi", "Tess says hi"];
    assert_eq!(expected, data);
}

#[test]
fn insert_from_select_reference() {
    use crate::schema::posts::dsl::*;
    use crate::schema::users::dsl::{id, name, users};

    let mut conn = connection_with_sean_and_tess_in_users_table();
    let select = users.select((id, name.concat(" says hi")));
    insert_into(posts)
        .values(&select)
        .into_columns((user_id, title))
        .execute(&mut conn)
        .unwrap();

    let data = posts.select(title).load::<String>(&mut conn).unwrap();
    let expected = vec!["Sean says hi", "Tess says hi"];
    assert_eq!(expected, data);
}

#[test]
fn insert_from_boxed() {
    use crate::schema::posts::dsl::*;
    use crate::schema::users::dsl::{id, name, users};

    let mut conn = connection_with_sean_and_tess_in_users_table();
    users
        .select((id, name.concat(" says hi")))
        .into_boxed()
        .insert_into(posts)
        .into_columns((user_id, title))
        .execute(&mut conn)
        .unwrap();

    let data = posts.select(title).load::<String>(&mut conn).unwrap();
    let expected = vec!["Sean says hi", "Tess says hi"];
    assert_eq!(expected, data);
}

#[test]
fn insert_from_boxed_reference() {
    use crate::schema::posts::dsl::*;
    use crate::schema::users::dsl::{id, name, users};

    let mut conn = connection_with_sean_and_tess_in_users_table();
    let select = users.select((id, name.concat(" says hi"))).into_boxed();
    insert_into(posts)
        .values(&select)
        .into_columns((user_id, title))
        .execute(&mut conn)
        .unwrap();

    let data = posts.select(title).load::<String>(&mut conn).unwrap();
    let expected = vec!["Sean says hi", "Tess says hi"];
    assert_eq!(expected, data);
}

#[test]
#[cfg(feature = "sqlite")]
fn insert_or_ignore_with_select() {
    use crate::schema::posts::dsl::*;
    use crate::schema::users::dsl::{id, name, users};

    let mut conn = connection_with_sean_and_tess_in_users_table();
    sql_query("CREATE UNIQUE INDEX foo ON posts (user_id)")
        .execute(&mut conn)
        .unwrap();

    insert_or_ignore_into(posts)
        .values(users.select((id, name.concat(" says hi"))))
        .into_columns((user_id, title))
        .execute(&mut conn)
        .unwrap();
    insert_or_ignore_into(posts)
        .values(users.select((id, name.concat(" says bye"))))
        .into_columns((user_id, title))
        .execute(&mut conn)
        .unwrap();

    let data = posts.select(title).load::<String>(&mut conn).unwrap();
    let expected = vec!["Sean says hi", "Tess says hi"];
    assert_eq!(expected, data);
}

#[test]
#[cfg(feature = "sqlite")]
fn insert_or_replace_with_select() {
    use crate::schema::posts::dsl::*;
    use crate::schema::users::dsl::{id, name, users};

    let mut conn = connection_with_sean_and_tess_in_users_table();
    sql_query("CREATE UNIQUE INDEX foo ON posts (user_id)")
        .execute(&mut conn)
        .unwrap();

    replace_into(posts)
        .values(users.select((id, name.concat(" says hi"))))
        .into_columns((user_id, title))
        .execute(&mut conn)
        .unwrap();
    replace_into(posts)
        .values(users.select((id, name.concat(" says bye"))))
        .into_columns((user_id, title))
        .execute(&mut conn)
        .unwrap();

    let data = posts.select(title).load::<String>(&mut conn).unwrap();
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

    let mut conn = connection_with_sean_and_tess_in_users_table();

    insert_or_ignore_into(posts)
        .values(users.select((id, name.concat(" says hi"))))
        .into_columns((user_id, title))
        .execute(&mut conn)
        .unwrap();

    let data = posts.select(title).load::<String>(&mut conn).unwrap();
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

    let mut conn = connection_with_sean_and_tess_in_users_table();

    replace_into(posts)
        .values(users.select((id, name.concat(" says hi"))))
        .into_columns((user_id, title))
        .execute(&mut conn)
        .unwrap();

    let data = posts.select(title).load::<String>(&mut conn).unwrap();
    let expected = vec!["Sean says hi", "Tess says hi"];
    assert_eq!(expected, data);
}

#[test]
#[cfg(any(feature = "postgres", feature = "sqlite"))]
fn on_conflict_do_nothing_with_select() {
    use crate::schema::posts::dsl::*;
    use crate::schema::users::dsl::{id, name, users};

    let mut conn = connection_with_sean_and_tess_in_users_table();

    sql_query("CREATE UNIQUE INDEX index_on_title ON posts (title)")
        .execute(&mut conn)
        .unwrap();
    let query = users
        .select((id, name.concat(" says hi")))
        .filter(id.ge(0)) // Sqlite needs a where claues
        .insert_into(posts)
        .into_columns((user_id, title))
        .on_conflict_do_nothing();

    let inserted_rows = query.execute(&mut conn).unwrap();
    assert_eq!(2, inserted_rows);
    let inserted_rows = query.execute(&mut conn).unwrap();
    assert_eq!(0, inserted_rows);

    let data = posts.select(title).load::<String>(&mut conn).unwrap();
    let expected = vec!["Sean says hi", "Tess says hi"];
    assert_eq!(expected, data);
}

#[test]
#[cfg(any(feature = "postgres", feature = "sqlite"))]
fn on_conflict_do_update_with_select() {
    use crate::schema::posts::dsl::*;
    use crate::schema::users::dsl::{id, name, users};

    let mut conn = connection_with_sean_and_tess_in_users_table();

    sql_query("CREATE UNIQUE INDEX index_on_title ON posts (title)")
        .execute(&mut conn)
        .unwrap();
    let query = users
        .select((id, name.concat(" says hi")))
        .filter(id.ge(0)) // exists because sqlite needs a where clause
        .insert_into(posts)
        .into_columns((user_id, title))
        .on_conflict(title)
        .do_update()
        .set(body.eq("updated"));

    query.execute(&mut conn).unwrap();

    insert_into(users)
        .values(name.eq("Ruby"))
        .execute(&mut conn)
        .unwrap();

    query.execute(&mut conn).unwrap();

    let data = posts.select((title, body)).load(&mut conn).unwrap();
    let expected = vec![
        (String::from("Sean says hi"), Some(String::from("updated"))),
        (String::from("Tess says hi"), Some(String::from("updated"))),
        (String::from("Ruby says hi"), None),
    ];
    assert_eq!(expected, data);
}

#[test]
#[cfg(all(feature = "postgres", feature = "sqlite"))]
fn on_conflict_do_update_with_boxed_select() {
    use schema::posts::dsl::*;
    use schema::users::dsl::{id, name, users};

    let mut conn = connection_with_sean_and_tess_in_users_table();

    sql_query("CREATE UNIQUE INDEX index_on_title ON posts (title)")
        .execute(&mut conn)
        .unwrap();

    users
        .select((id, name.concat(" says hi")))
        .into_boxed()
        .insert_into(posts)
        .into_columns((user_id, title))
        .on_conflict(title)
        .do_update()
        .set(body.eq("updated"))
        .execute(&mut conn)
        .unwrap();

    insert_into(users)
        .values(name.eq("Ruby"))
        .execute(&mut conn)
        .unwrap();

    users
        .select((id, name.concat(" says hi")))
        .into_boxed()
        .insert_into(posts)
        .into_columns((user_id, title))
        .on_conflict(title)
        .do_update()
        .set(body.eq("updated"))
        .execute(&mut conn)
        .unwrap();

    let data = posts.select((title, body)).load(&mut conn).unwrap();
    let expected = vec![
        (String::from("Sean says hi"), Some(String::from("updated"))),
        (String::from("Tess says hi"), Some(String::from("updated"))),
        (String::from("Ruby says hi"), None),
    ];
    assert_eq!(expected, data);
}
