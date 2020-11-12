use crate::schema::*;
use diesel::sql_types::Text;
use diesel::*;

#[test]
fn association_where_struct_name_doesnt_match_table_name() {
    #[derive(PartialEq, Eq, Debug, Clone, Queryable, Identifiable, Associations)]
    #[diesel(belongs_to(Post))]
    #[diesel(table_name = comments)]
    struct OtherComment {
        id: i32,
        post_id: i32,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    let sean = find_user_by_name("Sean", connection);
    insert_into(posts::table)
        .values(&sean.new_post("Hello", None))
        .execute(connection)
        .unwrap();
    let post = posts::table.first::<Post>(connection).unwrap();
    insert_into(comments::table)
        .values(&NewComment(post.id, "comment"))
        .execute(connection)
        .unwrap();

    let comment_text = OtherComment::belonging_to(&post)
        .select(comments::text)
        .first::<String>(connection);
    assert_eq!(Ok("comment".into()), comment_text);
}

#[test]
#[cfg(not(any(feature = "sqlite", feature = "mysql")))]
fn association_where_parent_and_child_have_underscores() {
    #[derive(PartialEq, Eq, Debug, Clone, Queryable, Identifiable, Associations)]
    #[diesel(belongs_to(User))]
    pub struct SpecialPost {
        id: i32,
        user_id: i32,
        title: String,
    }

    #[derive(Insertable)]
    #[diesel(table_name = special_posts)]
    struct NewSpecialPost {
        user_id: i32,
        title: String,
    }

    impl SpecialPost {
        fn new(user_id: i32, title: &str) -> NewSpecialPost {
            NewSpecialPost {
                user_id: user_id,
                title: title.to_owned(),
            }
        }
    }

    #[derive(PartialEq, Eq, Debug, Clone, Queryable, Identifiable, Associations)]
    #[diesel(belongs_to(SpecialPost))]
    struct SpecialComment {
        id: i32,
        special_post_id: i32,
    }

    impl SpecialComment {
        fn new(special_post_id: i32) -> NewSpecialComment {
            NewSpecialComment {
                special_post_id: special_post_id,
            }
        }
    }

    #[derive(Insertable)]
    #[diesel(table_name = special_comments)]
    struct NewSpecialComment {
        special_post_id: i32,
    }

    let connection = &mut connection_with_sean_and_tess_in_users_table();

    let sean = find_user_by_name("Sean", connection);
    let new_post = SpecialPost::new(sean.id, "title");
    let special_post: SpecialPost = insert_into(special_posts::table)
        .values(&new_post)
        .get_result(connection)
        .unwrap();
    let new_comment = SpecialComment::new(special_post.id);
    insert_into(special_comments::table)
        .values(&new_comment)
        .execute(connection)
        .unwrap();

    let comment: SpecialComment = SpecialComment::belonging_to(&special_post)
        .first(connection)
        .unwrap();

    assert_eq!(special_post.id, comment.special_post_id);
}

// This module has no test functions, as it's only to test compilation.
mod associations_can_have_nullable_foreign_keys {
    #![allow(dead_code)]

    table! {
        foos{
            id -> Integer,
        }
    }

    table! {
        bars {
            id -> Integer,
            foo_id -> Nullable<Integer>,
        }
    }
    // This test has no assertions, as it is for compilation purposes only.
    #[derive(Identifiable)]
    pub struct Foo {
        id: i32,
    }

    #[derive(Identifiable, Associations)]
    #[diesel(belongs_to(Foo))]
    pub struct Bar {
        id: i32,
        foo_id: Option<i32>,
    }
}

// This module has no test functions, as it's only to test compilation.
mod multiple_lifetimes_in_insertable_struct_definition {
    #![allow(dead_code)]
    use crate::schema::posts;

    #[derive(Insertable)]
    #[diesel(table_name = posts)]
    pub struct MyPost<'a> {
        title: &'a str,
        body: &'a str,
    }
}

mod lifetimes_with_names_other_than_a {
    #![allow(dead_code)]
    use crate::schema::posts;

    #[derive(Insertable)]
    #[diesel(table_name = posts)]
    pub struct MyPost<'a, 'b> {
        id: i32,
        title: &'b str,
        body: &'a str,
    }
}

mod insertable_with_cow {
    #![allow(dead_code)]
    use crate::schema::posts;
    use std::borrow::Cow;

    #[derive(Insertable)]
    #[diesel(table_name = posts)]
    pub struct MyPost<'a> {
        id: i32,
        title: Cow<'a, str>,
        body: Cow<'a, str>,
    }
}

mod custom_foreign_keys_are_respected_on_belongs_to {
    #![allow(dead_code)]

    use crate::schema::User;

    table! { special_posts { id -> Integer, author_id -> Integer, } }

    #[derive(Identifiable, Associations)]
    #[diesel(belongs_to(User, foreign_key = author_id))]
    pub struct SpecialPost {
        id: i32,
        author_id: i32,
    }
}

mod derive_identifiable_with_lifetime {
    #![allow(dead_code)]
    use crate::schema::posts;

    #[derive(Identifiable)]
    pub struct Post<'a> {
        id: &'a i32,
    }
}

#[test]
fn derive_identifiable_with_non_standard_pk() {
    use diesel::associations::*;

    #[derive(Identifiable)]
    #[diesel(table_name = posts)]
    #[diesel(primary_key(foo_id))]
    #[allow(dead_code)]
    struct Foo<'a> {
        id: i32,
        foo_id: &'a str,
        foo: i32,
    }

    let foo1 = Foo {
        id: 1,
        foo_id: "hi",
        foo: 2,
    };
    let foo2 = Foo {
        id: 2,
        foo_id: "there",
        foo: 3,
    };
    assert_eq!(&"hi", foo1.id());
    assert_eq!(&"there", foo2.id());
    // Fails to compile if wrong table is generated.
    let _: posts::table = Foo::<'static>::table();
}

#[test]
fn derive_identifiable_with_composite_pk() {
    use diesel::associations::Identifiable;

    #[derive(Identifiable)]
    #[diesel(primary_key(foo_id, bar_id))]
    #[diesel(table_name = posts)]
    #[allow(dead_code)]
    struct Foo {
        id: i32,
        foo_id: i32,
        bar_id: i32,
        foo: i32,
    }

    let foo1 = Foo {
        id: 1,
        foo_id: 2,
        bar_id: 3,
        foo: 4,
    };
    let foo2 = Foo {
        id: 5,
        foo_id: 6,
        bar_id: 7,
        foo: 8,
    };
    assert_eq!((&2, &3), foo1.id());
    assert_eq!((&6, &7), foo2.id());
}

#[test]
fn derive_insertable_with_option_for_not_null_field_with_default() {
    #[derive(Insertable)]
    #[diesel(table_name = users)]
    struct NewUser {
        id: Option<i32>,
        name: &'static str,
    }

    let conn = &mut connection();
    let data = vec![
        NewUser {
            id: None,
            name: "Jim",
        },
        NewUser {
            id: Some(123),
            name: "Bob",
        },
    ];
    assert_eq!(Ok(2), insert_into(users::table).values(&data).execute(conn));

    let users = users::table.load::<User>(conn).unwrap();
    let jim = users.iter().find(|u| u.name == "Jim");
    let bob = users.iter().find(|u| u.name == "Bob");

    assert!(jim.is_some());
    assert_eq!(Some(&User::new(123, "Bob")), bob);
}

sql_function!(fn nextval(a: Text) -> Integer);

#[test]
#[cfg(feature = "postgres")]
fn derive_insertable_with_field_that_cannot_convert_expression_to_nullable() {
    #[derive(Insertable)]
    #[diesel(table_name = users)]
    struct NewUser {
        id: nextval::HelperType<&'static str>,
        name: &'static str,
    }

    let conn = &mut connection();
    let data = NewUser {
        id: nextval("users_id_seq"),
        name: "Jim",
    };
    assert_eq!(Ok(1), insert_into(users::table).values(&data).execute(conn));

    let users = users::table.load::<User>(conn).unwrap();
    let jim = users.iter().find(|u| u.name == "Jim");

    assert!(jim.is_some());
}

#[test]
fn nested_queryable_derives() {
    #[derive(Queryable, Debug, PartialEq)]
    struct UserAndPost {
        user: User,
        post: Post,
    }

    let conn = &mut connection_with_sean_and_tess_in_users_table();
    let sean = find_user_by_name("Sean", conn);
    insert_into(posts::table)
        .values(&sean.new_post("Hi", None))
        .execute(conn)
        .unwrap();
    let post = posts::table.first(conn).unwrap();

    let expected = UserAndPost { user: sean, post };
    let actual = users::table.inner_join(posts::table).get_result(conn);

    assert_eq!(Ok(expected), actual);
}
