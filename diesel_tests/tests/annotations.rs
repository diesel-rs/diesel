use diesel::*;
use schema::*;

// FIXME: Bring this test back once we can figure out how to allow multiple structs
// on the same table to use `#[belongs_to]` without overlapping the `JoinTo` impls
// #[test]
// fn association_where_struct_name_doesnt_match_table_name() {
//     #[derive(PartialEq, Eq, Debug, Clone, Queryable, Identifiable, Associations)]
//     #[belongs_to(Post)]
//     #[table_name="comments"]
//     struct OtherComment {
//         id: i32,
//         post_id: i32
//     }

//     let connection = connection_with_sean_and_tess_in_users_table();

//     let sean = find_user_by_name("Sean", &connection);
//     let post: Post = insert(&sean.new_post("Hello", None)).into(posts::table)
//         .get_result(&connection).unwrap();
//     insert(&NewComment(post.id, "comment")).into(comments::table)
//         .execute(&connection).unwrap();

//     let comment_text = OtherComment::belonging_to(&post).select(comments::text)
//         .first::<String>(&connection);
//     assert_eq!(Ok("comment".into()), comment_text);
// }

#[test]
#[cfg(not(any(feature="sqlite", feature="mysql")))]
fn association_where_parent_and_child_have_underscores() {
    #[derive(PartialEq, Eq, Debug, Clone, Queryable, Identifiable, Associations)]
    #[has_many(special_comments)]
    #[belongs_to(User)]
    pub struct SpecialPost {
        id: i32,
        user_id: i32,
        title: String
    }

    #[derive(Insertable)]
    #[table_name="special_posts"]
    struct NewSpecialPost {
        user_id: i32,
        title: String
    }

    impl SpecialPost {
        fn new(user_id: i32, title: &str) -> NewSpecialPost {
            NewSpecialPost {
                user_id: user_id,
                title: title.to_owned()
            }
        }
    }

    #[derive(PartialEq, Eq, Debug, Clone, Queryable, Identifiable, Associations)]
    #[belongs_to(SpecialPost)]
    struct SpecialComment {
        id: i32,
        special_post_id: i32,
    }

    impl SpecialComment {
        fn new(special_post_id: i32) -> NewSpecialComment {
            NewSpecialComment {
                special_post_id: special_post_id
            }
        }
    }

    #[derive(Insertable)]
    #[table_name="special_comments"]
    struct NewSpecialComment {
        special_post_id: i32
    }

    let connection = connection_with_sean_and_tess_in_users_table();

    let sean = find_user_by_name("Sean", &connection);
    let new_post = SpecialPost::new(sean.id, "title");
    let special_post: SpecialPost = insert(&new_post).into(special_posts::table)
        .get_result(&connection).unwrap();
    let new_comment = SpecialComment::new(special_post.id);
    insert(&new_comment).into(special_comments::table)
        .execute(&connection).unwrap();

    let comment: SpecialComment = SpecialComment::belonging_to(&special_post)
        .first(&connection).unwrap();

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
    #[has_many(bars)]
    #[derive(Identifiable, Associations)]
    pub struct Foo {
        id: i32,
    }

    #[belongs_to(Foo)]
    #[derive(Identifiable, Associations)]
    pub struct Bar {
        id: i32,
        foo_id: Option<i32>,
    }
}

// This module has no test functions, as it's only to test compilation.
mod multiple_lifetimes_in_insertable_struct_definition {
    #![allow(dead_code)]
    use schema::posts;

    #[derive(Insertable)]
    #[table_name="posts"]
    pub struct MyPost<'a> {
        title: &'a str,
        body: &'a str,
    }
}

mod lifetimes_with_names_other_than_a {
    #![allow(dead_code)]
    use schema::posts;

    #[derive(Insertable)]
    #[table_name="posts"]
    pub struct MyPost<'a, 'b> {
        id: i32,
        title: &'b str,
        body: &'a str,
    }
}

mod custom_foreign_keys_are_respected_on_belongs_to {
    #![allow(dead_code)]

    use schema::User;

    table! { special_posts { id -> Integer, author_id -> Integer, } }

    #[derive(Identifiable, Associations)]
    #[belongs_to(User, foreign_key = "author_id")]
    pub struct SpecialPost {
        id: i32,
        author_id: i32,
    }
}

mod derive_identifiable_with_lifetime {
    #![allow(dead_code)]
    use schema::posts;

    #[derive(Identifiable)]
    pub struct Post<'a> {
        id: &'a i32
    }
}

#[test]
fn derive_identifiable_with_non_standard_pk() {
    use diesel::associations::*;

    #[derive(Identifiable)]
    #[table_name="posts"]
    #[primary_key(foo_id)]
    #[allow(dead_code)]
    struct Foo<'a> {
        id: i32,
        foo_id: &'a str,
        foo: i32,
    }

    let foo1 = Foo { id: 1, foo_id: "hi", foo: 2 };
    let foo2 = Foo { id: 2, foo_id: "there", foo: 3 };
    assert_eq!(&"hi", foo1.id());
    assert_eq!(&"there", foo2.id());
    // Fails to compile if wrong table is generated.
    let _: posts::table = Foo::<'static>::table();
}

#[test]
fn derive_identifiable_with_composite_pk() {
    use diesel::associations::Identifiable;

    #[derive(Identifiable)]
    #[primary_key(foo_id, bar_id)]
    #[table_name="posts"]
    #[allow(dead_code)]
    struct Foo {
        id: i32,
        foo_id: i32,
        bar_id: i32,
        foo: i32,
    }

    let foo1 = Foo { id: 1, foo_id: 2, bar_id: 3, foo: 4 };
    let foo2 = Foo { id: 5, foo_id: 6, bar_id: 7, foo: 8 };
    assert_eq!((&2, &3), foo1.id());
    assert_eq!((&6, &7), foo2.id());
}

#[test]
fn derive_insertable_with_option_for_not_null_field_with_default() {
    #[derive(Insertable)]
    #[table_name="users"]
    struct NewUser {
        id: Option<i32>,
        name: &'static str,
    }

    let conn = connection();
    let data = vec![
        NewUser { id: None, name: "Jim" },
        NewUser { id: Some(123), name: "Bob" },
    ];
    assert_eq!(Ok(2), insert(&data).into(users::table).execute(&conn));

    let users = users::table.load::<User>(&conn).unwrap();
    let jim = users.iter().find(|u| u.name == "Jim");
    let bob = users.iter().find(|u| u.name == "Bob");

    assert!(jim.is_some());
    assert_eq!(Some(&User::new(123, "Bob")), bob);
}
