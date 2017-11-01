use schema::*;
use diesel::*;

macro_rules! assert_sets_eq {
    ($set1:expr, $set2:expr) => {
        let set1 = {$set1};
        let set2 = {$set2};
        let s1r : Vec<_> = set1.iter().filter(|&si| !set2.contains(si)).collect();
        assert!(s1r.len() == 0, format!("left set contains items not found in right set: {:?}", s1r));
        let s2r : Vec<_> = set2.iter().filter(|&si| !set1.contains(si)).collect();
        assert!(s2r.len() == 0, format!("right set contains items not found in left set: {:?}", s2r));
    };
}

#[test]
fn filter_by_int_equality() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    let sean_id = find_user_by_name("Sean", &connection).id;
    let tess_id = find_user_by_name("Tess", &connection).id;
    let unused_id = sean_id + tess_id;

    let sean = User::new(sean_id, "Sean");
    let tess = User::new(tess_id, "Tess");
    assert_eq!(Ok(sean), users.filter(id.eq(sean_id)).first(&connection));
    assert_eq!(Ok(tess), users.filter(id.eq(tess_id)).first(&connection));
    assert_eq!(
        Err(NotFound),
        users.filter(id.eq(unused_id)).first::<User>(&connection)
    );
}

#[test]
fn filter_by_string_equality() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();

    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    assert_eq!(Ok(sean), users.filter(name.eq("Sean")).first(&connection));
    assert_eq!(Ok(tess), users.filter(name.eq("Tess")).first(&connection));
    assert_eq!(
        Err(NotFound),
        users.filter(name.eq("Jim")).first::<User>(&connection)
    );
}

#[test]
fn filter_by_equality_on_nullable_columns() {
    use schema::users::dsl::*;

    let connection = connection();
    let data = vec![
        NewUser::new("Sean", Some("black")),
        NewUser::new("Tess", Some("brown")),
        NewUser::new("Jim", Some("black")),
    ];
    insert_into(users)
        .values(&data)
        .execute(&connection)
        .unwrap();

    let data = users.order(id).load::<User>(&connection).unwrap();
    let sean = data[0].clone();
    let tess = data[1].clone();
    let jim = data[2].clone();

    let source = users.filter(hair_color.eq("black"));
    assert_sets_eq!(vec![sean, jim], source.load(&connection).unwrap());

    let source = users.filter(hair_color.eq("brown"));
    assert_eq!(vec![tess], source.load(&connection).unwrap());
}

#[test]
fn filter_by_is_not_null_on_nullable_columns() {
    use schema::users::dsl::*;

    let connection = connection();
    let data = vec![
        NewUser::new("Derek", Some("red")),
        NewUser::new("Gordon", None),
    ];
    insert_into(users)
        .values(&data)
        .execute(&connection)
        .unwrap();
    let data = users.order(id).load::<User>(&connection).unwrap();
    let derek = data[0].clone();

    let source = users.filter(hair_color.is_not_null());
    assert_eq!(vec![derek], source.load(&connection).unwrap());
}

#[test]
fn filter_by_is_null_on_nullable_columns() {
    use schema::users::dsl::*;

    let connection = connection();
    let data = vec![
        NewUser::new("Derek", Some("red")),
        NewUser::new("Gordon", None),
    ];
    insert_into(users)
        .values(&data)
        .execute(&connection)
        .unwrap();
    let data = users.order(id).load::<User>(&connection).unwrap();
    let gordon = data[1].clone();

    let source = users.filter(hair_color.is_null());
    assert_eq!(vec![gordon], source.load(&connection).unwrap());
}

#[test]
fn filter_after_joining() {
    use schema::users::name;

    let connection = connection_with_sean_and_tess_in_users_table();
    connection
        .execute(
            "INSERT INTO posts (id, title, user_id) VALUES
                       (1, 'Hello', 1), (2, 'World', 2)",
        )
        .unwrap();

    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    let seans_post = Post::new(1, 1, "Hello", None);
    let tess_post = Post::new(2, 2, "World", None);
    let source = users::table.inner_join(posts::table);
    assert_eq!(
        Ok((sean, seans_post)),
        source.filter(name.eq("Sean")).first(&connection)
    );
    assert_eq!(
        Ok((tess, tess_post)),
        source.filter(name.eq("Tess")).first(&connection)
    );
    assert_eq!(
        Err(NotFound),
        source
            .filter(name.eq("Jim"))
            .first::<(User, Post)>(&connection)
    );
}

#[test]
fn select_then_filter() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();

    let source = users.select(name);
    assert_eq!(
        Ok("Sean".to_string()),
        source.filter(name.eq("Sean")).first(&connection)
    );
    assert_eq!(
        Ok("Tess".to_string()),
        source.filter(name.eq("Tess")).first(&connection)
    );
    assert_eq!(
        Err(NotFound),
        source.filter(name.eq("Jim")).first::<String>(&connection)
    );
}

#[test]
fn filter_then_select() {
    use schema::users::dsl::*;

    let connection = connection();
    let data = vec![NewUser::new("Sean", None), NewUser::new("Tess", None)];
    insert_into(users)
        .values(&data)
        .execute(&connection)
        .unwrap();

    assert_eq!(
        Ok("Sean".to_string()),
        users
            .filter(name.eq("Sean"))
            .select(name)
            .first(&connection)
    );
    assert_eq!(
        Ok("Tess".to_string()),
        users
            .filter(name.eq("Tess"))
            .select(name)
            .first(&connection)
    );
    assert_eq!(
        Err(NotFound),
        users
            .filter(name.eq("Jim"))
            .select(name)
            .first::<String>(&connection)
    );
}

#[test]
fn filter_on_multiple_columns() {
    use schema::users::dsl::*;

    let connection = connection();
    let data: &[_] = &[
        NewUser::new("Sean", Some("black")),
        NewUser::new("Sean", Some("brown")),
        NewUser::new("Sean", None),
        NewUser::new("Tess", Some("black")),
        NewUser::new("Tess", Some("brown")),
    ];
    insert_into(users)
        .values(data)
        .execute(&connection)
        .unwrap();
    let data = users.order(id).load::<User>(&connection).unwrap();
    let black_haired_sean = data[0].clone();
    let brown_haired_sean = data[1].clone();
    let black_haired_tess = data[3].clone();
    let brown_haired_tess = data[4].clone();

    let source = users.filter(name.eq("Sean").and(hair_color.eq("black")));
    assert_eq!(vec![black_haired_sean], source.load(&connection).unwrap());

    let source = users.filter(name.eq("Sean").and(hair_color.eq("brown")));
    assert_eq!(vec![brown_haired_sean], source.load(&connection).unwrap());

    let source = users.filter(name.eq("Tess").and(hair_color.eq("black")));
    assert_eq!(vec![black_haired_tess], source.load(&connection).unwrap());

    let source = users.filter(name.eq("Tess").and(hair_color.eq("brown")));
    assert_eq!(vec![brown_haired_tess], source.load(&connection).unwrap());
}

#[test]
fn filter_called_twice_means_same_thing_as_and() {
    use schema::users::dsl::*;

    let connection = connection();
    let data: &[_] = &[
        NewUser::new("Sean", Some("black")),
        NewUser::new("Sean", Some("brown")),
        NewUser::new("Sean", None),
        NewUser::new("Tess", Some("black")),
        NewUser::new("Tess", Some("brown")),
    ];
    insert_into(users)
        .values(data)
        .execute(&connection)
        .unwrap();
    let data = users.order(id).load::<User>(&connection).unwrap();
    let black_haired_sean = data[0].clone();
    let brown_haired_sean = data[1].clone();
    let black_haired_tess = data[3].clone();
    let brown_haired_tess = data[4].clone();

    let source = users.filter(name.eq("Sean")).filter(hair_color.eq("black"));
    assert_eq!(vec![black_haired_sean], source.load(&connection).unwrap());

    let source = users.filter(name.eq("Sean")).filter(hair_color.eq("brown"));
    assert_eq!(vec![brown_haired_sean], source.load(&connection).unwrap());

    let source = users.filter(name.eq("Tess")).filter(hair_color.eq("black"));
    assert_eq!(vec![black_haired_tess], source.load(&connection).unwrap());

    let source = users.filter(name.eq("Tess")).filter(hair_color.eq("brown"));
    assert_eq!(vec![brown_haired_tess], source.load(&connection).unwrap());
}

table! {
    points (x) {
        x -> Integer,
        y -> Integer,
    }
}

#[test]
fn filter_on_column_equality() {
    use self::points::dsl::*;

    let connection = connection();
    connection
        .execute("INSERT INTO points (x, y) VALUES (1, 1), (1, 2), (2, 2)")
        .unwrap();

    let expected_data = vec![(1, 1), (2, 2)];
    let query = points.order(x).filter(x.eq(y));
    let data: Vec<_> = query.load(&connection).unwrap();
    assert_sets_eq!(expected_data, data);
}

#[test]
fn filter_with_or() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    insert_into(users)
        .values(&NewUser::new("Jim", None))
        .execute(&connection)
        .unwrap();

    let expected_users = vec![User::new(1, "Sean"), User::new(2, "Tess")];
    let data: Vec<_> = users
        .order(id)
        .filter(name.eq("Sean").or(name.eq("Tess")))
        .load(&connection)
        .unwrap();

    assert_sets_eq!(expected_users, data);
}

#[test]
fn or_doesnt_mess_with_precidence_of_previous_statements() {
    use schema::users::dsl::*;

    let connection = connection_with_sean_and_tess_in_users_table();
    let f = false.into_sql::<types::Bool>();
    let count = users
        .filter(f)
        .filter(f.or(true))
        .count()
        .first(&connection);

    assert_eq!(Ok(0), count);

    let count = users
        .filter(f.or(f).and(f.or(true)))
        .count()
        .first(&connection);

    assert_eq!(Ok(0), count);
}

#[test]
fn not_does_not_affect_expressions_other_than_those_passed_to_it() {
    use schema::users::dsl::*;
    use diesel::dsl::not;

    let connection = connection_with_sean_and_tess_in_users_table();
    let count = users
        .filter(not(name.eq("Tess")))
        .filter(id.eq(1))
        .count()
        .get_result(&connection);

    assert_eq!(Ok(1), count);
}

#[test]
fn not_affects_arguments_passed_when_they_contain_higher_operator_precedence() {
    use schema::users::dsl::*;
    use diesel::dsl::not;

    let connection = connection_with_sean_and_tess_in_users_table();
    let count = users
        .filter(not(name.eq("Tess").and(id.eq(1))))
        .count()
        .get_result(&connection);

    assert_eq!(Ok(2), count);
}

use diesel::types::VarChar;
sql_function!(lower, lower_t, (x: VarChar) -> VarChar);

#[test]
fn filter_by_boxed_predicate() {
    fn by_name(
        name: &str,
    ) -> Box<BoxableExpression<users::table, TestBackend, SqlType = types::Bool>> {
        Box::new(lower(users::name).eq(name.to_string()))
    }

    let connection = connection_with_sean_and_tess_in_users_table();
    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    let queried_sean = users::table.filter(by_name("sean")).first(&connection);
    let queried_tess = users::table.filter(by_name("tess")).first(&connection);

    assert_eq!(Ok(sean), queried_sean);
    assert_eq!(Ok(tess), queried_tess);
}

#[test]
fn filter_subselect_referencing_outer_table() {
    use diesel::dsl::exists;

    let conn = connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", &conn);

    insert_into(posts::table)
        .values(&vec![
            sean.new_post("Hello", None),
            sean.new_post("Hello 2", None),
        ])
        .execute(&conn)
        .unwrap();

    let expected = Ok(vec![sean]);
    let users_with_published_posts = users::table
        .filter(exists(posts::table.filter(posts::user_id.eq(users::id))))
        .load(&conn);
    assert_eq!(expected, users_with_published_posts);

    let users_with_published_posts = users::table
        .filter(
            users::id.eq_any(
                posts::table
                    .select(posts::user_id)
                    .filter(posts::user_id.eq(users::id)),
            ),
        )
        .load(&conn);
    assert_eq!(expected, users_with_published_posts);
}

#[test]
fn filter_subselect_with_boxed_query() {
    use schema::users::dsl::*;

    let conn = connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", &conn);

    let subselect = users.filter(name.eq("Sean")).select(id).into_boxed();

    let expected = Ok(vec![sean]);
    let data = users.filter(id.eq_any(subselect)).load(&conn);
    assert_eq!(expected, data);
}

#[test]
#[cfg(feature = "postgres")]
fn filter_subselect_with_pg_any() {
    use diesel::dsl::any;

    let conn = connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", &conn);

    insert_into(posts::table)
        .values(&vec![
            sean.new_post("Hello", None),
            sean.new_post("Hello 2", None),
        ])
        .execute(&conn)
        .unwrap();

    let users_with_published_posts = users::table
        .filter(
            users::id.eq(any(
                posts::table
                    .select(posts::user_id)
                    .filter(posts::user_id.eq(users::id)),
            )),
        )
        .load(&conn);
    assert_eq!(Ok(vec![sean]), users_with_published_posts);
}

#[test]
#[cfg(feature = "sqlite")]
fn filter_by_string_equality_case_insensitive() {
    use schema::users::dsl::*;
    use diesel::sqlite::expression::*;

    let connection = connection_with_sean_and_tess_in_users_table();

    let sean = User::new(1, "Sean");
    let tess = User::new(2, "Tess");
    assert_eq!(
        Ok(sean),
        users
            .filter(name.eq("sean").collate(CollationNoCase))
            .first(&connection)
    );
    assert_eq!(
        Ok(tess),
        users
            .filter(name.collate(CollationRTrim).eq("Tess "))
            .first(&connection)
    );
}
