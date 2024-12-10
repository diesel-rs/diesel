use super::schema::*;
use diesel::*;

#[test]
fn simple_distinct() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection();
    diesel::sql_query("INSERT INTO users (name) VALUES ('Sean'), ('Sean'), ('Tess')")
        .execute(connection)
        .unwrap();

    let source = users.select(name).distinct().order(name);
    let expected_data = vec!["Sean".to_string(), "Tess".to_string()];
    let data: Vec<String> = source.load(connection).unwrap();

    assert_eq!(expected_data, data);
}

#[cfg(feature = "postgres")]
#[test]
fn distinct_on() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection();
    diesel::sql_query(
            "INSERT INTO users (name, hair_color) VALUES ('Sean', 'black'), ('Sean', NULL), ('Tess', NULL), ('Tess', NULL)",
        ).execute(connection)
        .unwrap();

    let source = users
        .select((name, hair_color))
        .order(name)
        .distinct_on(name);
    let mut expected_data = vec![
        ("Sean".to_string(), Some("black".to_string())),
        ("Tess".to_string(), None),
    ];
    let data: Vec<_> = source.load(connection).unwrap();

    assert_eq!(expected_data, data);

    let source = users
        .select((name, hair_color))
        .order(name.asc())
        .distinct_on(name);
    let data: Vec<_> = source.load(connection).unwrap();

    assert_eq!(expected_data, data);

    let source = users
        .select((name, hair_color))
        .order(name.desc())
        .distinct_on(name);
    let data: Vec<_> = source.load(connection).unwrap();

    expected_data.reverse();
    assert_eq!(expected_data, data);
}

#[cfg(feature = "postgres")]
#[test]
fn distinct_on_select_by() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection();
    diesel::sql_query(
            "INSERT INTO users (name, hair_color) VALUES ('Sean', 'black'), ('Sean', NULL), ('Tess', NULL), ('Tess', NULL)",
        ).execute(connection)
        .unwrap();

    let source = users
        .select(NewUser::as_select())
        .order(name)
        .distinct_on(name);
    let expected_data = vec![
        NewUser::new("Sean", Some("black")),
        NewUser::new("Tess", None),
    ];
    let data: Vec<_> = source.load(connection).unwrap();

    assert_eq!(expected_data, data);
}

#[cfg(feature = "postgres")]
#[test]
fn distinct_on_select_order_by_two_columns() {
    use diesel::sql_types::Integer;

    use crate::schema::users::dsl::*;

    let connection = &mut connection();
    diesel::sql_query(
            "INSERT INTO users (name, hair_color) VALUES ('Sean', 'black'), ('Sean', 'aqua'), ('Tess', 'bronze'), ('Tess', 'champagne')",
        ).execute(connection)
        .unwrap();

    let source = users
        .select((name, hair_color))
        .order((name, hair_color.desc()))
        .distinct_on(name);
    let expected_data = vec![
        NewUser::new("Sean", Some("black")),
        NewUser::new("Tess", Some("champagne")),
    ];
    let data: Vec<_> = source.load(connection).unwrap();

    assert_eq!(expected_data, data);

    let source = users
        .select((name, hair_color))
        .order((name.asc(), hair_color.desc()))
        .distinct_on(name);
    let data: Vec<_> = source.load(connection).unwrap();

    assert_eq!(expected_data, data);

    let source = users
        .select((name, hair_color))
        .order((name.desc(), hair_color.desc()))
        .distinct_on(name);
    let expected_data = vec![
        NewUser::new("Tess", Some("champagne")),
        NewUser::new("Sean", Some("black")),
    ];
    let data: Vec<_> = source.load(connection).unwrap();

    assert_eq!(expected_data, data);

    let source = users
        .select((name, hair_color))
        .order((name.desc(), hair_color))
        .distinct_on(name);
    let expected_data = vec![
        NewUser::new("Tess", Some("bronze")),
        NewUser::new("Sean", Some("aqua")),
    ];
    let data: Vec<_> = source.load(connection).unwrap();

    assert_eq!(expected_data, data);

    let source = users
        .select((name, hair_color))
        .order(dsl::sql::<Integer>("name DESC, hair_color"))
        .distinct_on(dsl::sql("name"));
    let expected_data = vec![
        NewUser::new("Tess", Some("bronze")),
        NewUser::new("Sean", Some("aqua")),
    ];
    let data: Vec<_> = source.load(connection).unwrap();

    assert_eq!(expected_data, data);
}

#[cfg(feature = "postgres")]
#[test]
fn distinct_of_multiple_columns() {
    use crate::schema::posts;
    use crate::schema::users;

    let mut connection = connection_with_sean_and_tess_in_users_table();

    let sean = find_user_by_name("Sean", &mut connection);
    let tess = find_user_by_name("Tess", &mut connection);

    let new_posts = vec![
        NewPost::new(sean.id, "1", Some("1")),
        NewPost::new(sean.id, "2", Some("2")),
        NewPost::new(sean.id, "3", Some("1")),
        NewPost::new(sean.id, "4", Some("2")),
        NewPost::new(tess.id, "5", Some("1")),
        NewPost::new(tess.id, "6", Some("2")),
        NewPost::new(tess.id, "7", Some("1")),
        NewPost::new(tess.id, "8", Some("2")),
    ];
    insert_into(posts::table)
        .values(&new_posts)
        .execute(&mut connection)
        .unwrap();
    let posts = posts::table
        .order(posts::title)
        .load::<Post>(&mut connection)
        .unwrap();

    // one order by
    // one distinct on
    let data = posts::table
        .order(posts::body)
        .distinct_on(posts::body)
        .load(&mut connection);
    let expected = vec![(posts[0].clone()), (posts[7].clone())];

    assert_eq!(Ok(expected), data);

    // multi order by
    // one distinct on
    let data = posts::table
        .inner_join(users::table)
        .order((users::id, posts::body, posts::title))
        .distinct_on(users::id)
        .load(&mut connection);
    let expected = vec![
        (posts[0].clone(), sean.clone()),
        (posts[4].clone(), tess.clone()),
    ];

    assert_eq!(Ok(expected), data);

    // one order by
    // multi distinct on
    let data = posts::table
        .inner_join(users::table)
        .order(users::id)
        .distinct_on((users::id, posts::body))
        .load::<(Post, User)>(&mut connection);

    assert!(data.is_ok(), "{:?}", data.unwrap_err());
    let data = data.unwrap();
    assert_eq!(data.len(), 4);
    assert_eq!(data[0].1, sean.clone());
    assert_eq!(data[1].1, sean.clone());
    assert_eq!(data[2].1, tess.clone());
    assert_eq!(data[3].1, tess.clone());
    // post id's are non-deterministic
    assert_eq!(data[0].0.body, Some("1".into()));
    assert_eq!(data[1].0.body, Some("2".into()));
    assert_eq!(data[2].0.body, Some("1".into()));
    assert_eq!(data[3].0.body, Some("2".into()));

    // multi order by
    // multi distinct on
    // order by > distinct on
    let data = posts::table
        .inner_join(users::table)
        .order((users::id, posts::body, posts::title))
        .distinct_on((users::id, posts::body))
        .load(&mut connection);
    let expected = vec![
        (posts[0].clone(), sean.clone()),
        (posts[1].clone(), sean.clone()),
        (posts[4].clone(), tess.clone()),
        (posts[5].clone(), tess.clone()),
    ];

    assert_eq!(Ok(expected), data);

    // multi order by
    // multi distinct on
    // order by < distinct on
    let data = posts::table
        .inner_join(users::table)
        .order((users::id, posts::body))
        .distinct_on((users::id, posts::body, posts::title))
        .load(&mut connection);
    let expected = vec![
        (posts[0].clone(), sean.clone()),
        (posts[2].clone(), sean.clone()),
        (posts[1].clone(), sean.clone()),
        (posts[3].clone(), sean.clone()),
        (posts[4].clone(), tess.clone()),
        (posts[6].clone(), tess.clone()),
        (posts[5].clone(), tess.clone()),
        (posts[7].clone(), tess.clone()),
    ];

    assert_eq!(Ok(expected), data);

    // multi order by
    // multi distinct on
    // including asc and desc
    let data = posts::table
        .inner_join(users::table)
        .order((users::id.asc(), posts::body.desc(), posts::title))
        .distinct_on((users::id, posts::body))
        .load(&mut connection);
    let expected = vec![
        (posts[1].clone(), sean.clone()),
        (posts[0].clone(), sean.clone()),
        (posts[5].clone(), tess.clone()),
        (posts[4].clone(), tess.clone()),
    ];

    assert_eq!(Ok(expected), data);

    // with arbitrary expressions

    let data = posts::table
        .left_join(users::table)
        .order((users::id.nullable(), posts::body.nullable().desc()))
        .distinct_on((users::id.nullable(), posts::body.nullable()))
        .load::<(Post, Option<User>)>(&mut connection);

    assert!(data.is_ok(), "{:?}", data.unwrap_err());
    let data = data.unwrap();
    assert_eq!(data.len(), 4);
    assert_eq!(data[0].1, Some(sean.clone()));
    assert_eq!(data[1].1, Some(sean.clone()));
    assert_eq!(data[2].1, Some(tess.clone()));
    assert_eq!(data[3].1, Some(tess.clone()));
    // post id's are non-deterministic
    assert_eq!(data[0].0.body, Some("2".into()));
    assert_eq!(data[1].0.body, Some("1".into()));
    assert_eq!(data[2].0.body, Some("2".into()));
    assert_eq!(data[3].0.body, Some("1".into()));

    let data = posts::table
        .left_join(users::table)
        .order((users::id.nullable(), posts::body.nullable().desc()))
        .distinct_on(users::id.nullable())
        .load(&mut connection);

    let expected = vec![
        (posts[1].clone(), Some(sean.clone())),
        (posts[7].clone(), Some(tess.clone())),
    ];

    assert_eq!(Ok(expected), data);

    let data = posts::table
        .left_join(users::table)
        .order(users::id.nullable())
        .distinct_on((users::id.nullable(), posts::body.nullable()))
        .load(&mut connection);

    let expected = vec![
        (posts[0].clone(), Some(sean.clone())),
        (posts[1].clone(), Some(sean)),
        (posts[4].clone(), Some(tess.clone())),
        (posts[7].clone(), Some(tess)),
    ];
    assert_eq!(Ok(expected), data);
}
