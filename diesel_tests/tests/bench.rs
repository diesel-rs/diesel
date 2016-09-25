#![feature(custom_derive, plugin, custom_attribute, test)]
#![plugin(diesel_codegen, dotenv_macros)]

#![allow(non_snake_case)]

#[macro_use]
extern crate diesel;
extern crate test;

mod schema;

use self::test::Bencher;
use self::schema::{
    users, posts, comments,
    NewUser, NewPost, NewComment,
    User, Post, Comment,
    TestConnection, batch_insert,
};
use diesel::*;

#[cfg(not(feature = "sqlite"))]
fn connection() -> TestConnection {
    let conn = schema::connection();
    conn.execute("TRUNCATE TABLE users").unwrap();
    conn.execute("TRUNCATE TABLE posts").unwrap();
    conn.execute("TRUNCATE TABLE comments").unwrap();
    conn
}

#[cfg(feature = "sqlite")]
fn connection() -> TestConnection {
    schema::connection()
}

macro_rules! bench_trivial_query {
    ($n:expr, $name:ident, $name_boxed:ident) => {
        #[bench]
        fn $name(b: &mut Bencher) {
            let conn = connection();

            let data: Vec<_> = (0..$n).map(|i| {
                NewUser::new(&format!("User {}", i), None)
            }).collect();
            batch_insert(&data, users::table, &conn);

            b.iter(|| {
                users::table.load::<User>(&conn).unwrap()
            })
        }

        #[bench]
        fn $name_boxed(b: &mut Bencher) {
            let conn = connection();

            let data: Vec<_> = (0..$n).map(|i| {
                NewUser::new(&format!("User {}", i), None)
            }).collect();
            batch_insert(&data, users::table, &conn);

            b.iter(|| {
                users::table.into_boxed().load::<User>(&conn).unwrap()
            })
        }
    };
}

// bench_trivial_query!(0,
//     bench_trivial_query_selecting______0_rows, bench_trivial_query_selecting______0_rows_boxed);
bench_trivial_query!(1,
    bench_trivial_query_selecting______1_row, bench_trivial_query_selecting______1_row_boxed);
bench_trivial_query!(10,
    bench_trivial_query_selecting_____10_rows, bench_trivial_query_selecting_____10_rows_boxed);
bench_trivial_query!(100,
    bench_trivial_query_selecting____100_rows, bench_trivial_query_selecting____100_rows_boxed);
bench_trivial_query!(1_000,
    bench_trivial_query_selecting__1_000_rows, bench_trivial_query_selecting__1_000_rows_boxed);
bench_trivial_query!(10_000,
    bench_trivial_query_selecting_10_000_rows, bench_trivial_query_selecting_10_000_rows_boxed);

macro_rules! bench_medium_complex_query {
    ($n:expr, $name:ident, $name_boxed:ident) => {
        #[bench]
        fn $name(b: &mut Bencher) {
            let conn = connection();

            let data: Vec<_> = (0..$n).map(|i| {
                let hair_color = if i % 2 == 0 { "black" } else { "brown" };
                NewUser::new(&format!("User {}", i), Some(hair_color))
            }).collect();
            batch_insert(&data, users::table, &conn);

            b.iter(|| {
                use schema::users::dsl::*;
                let target = users.left_outer_join(posts::table)
                    .filter(hair_color.eq("black"))
                    .order(name.desc());
                target.load::<(User, Option<Post>)>(&conn).unwrap()
            })
        }

        #[bench]
        fn $name_boxed(b: &mut Bencher) {
            let conn = connection();

            let data: Vec<_> = (0..$n).map(|i| {
                let hair_color = if i % 2 == 0 { "black" } else { "brown" };
                NewUser::new(&format!("User {}", i), Some(hair_color))
            }).collect();
            batch_insert(&data, users::table, &conn);

            b.iter(|| {
                use schema::users::dsl::*;
                let target = users.left_outer_join(posts::table)
                    .filter(hair_color.eq("black"))
                    .order(name.desc())
                    .into_boxed();
                target.load::<(User, Option<Post>)>(&conn).unwrap()
            })
        }
    };
}

// bench_medium_complex_query!(0,
//     bench_medium_complex_query_selecting______0_rows, bench_medium_complex_query_selecting______0_rows_boxed);
bench_medium_complex_query!(1,
    bench_medium_complex_query_selecting______1_row, bench_medium_complex_query_selecting______1_row_boxed);
bench_medium_complex_query!(10,
    bench_medium_complex_query_selecting_____10_rows, bench_medium_complex_query_selecting_____10_rows_boxed);
bench_medium_complex_query!(100,
    bench_medium_complex_query_selecting____100_rows, bench_medium_complex_query_selecting____100_rows_boxed);
bench_medium_complex_query!(1_000,
    bench_medium_complex_query_selecting__1_000_rows, bench_medium_complex_query_selecting__1_000_rows_boxed);
bench_medium_complex_query!(10_000,
    bench_medium_complex_query_selecting_10_000_rows, bench_medium_complex_query_selecting_10_000_rows_boxed);

#[bench]
fn loading_associations_sequentially(b: &mut Bencher) {
    // SETUP A FUCK TON OF DATA
    let conn = connection();
    let data: Vec<_> = (0..100).map(|i| {
        let hair_color = if i % 2 == 0 { "black" } else { "brown" };
        NewUser::new(&format!("User {}", i), Some(hair_color))
    }).collect();
    batch_insert(&data, users::table, &conn);
    let all_users = users::table.load::<User>(&conn).unwrap();
    let data: Vec<_> = all_users.iter().flat_map(|user| {
        let user_id = user.id;
        (0..10).map(move |i| {
            let title = format!("Post {} by user {}", i, user_id);
            NewPost::new(user_id, &title, None)
        })
    }).collect();
    batch_insert(&data, posts::table, &conn);
    let all_posts = posts::table.load::<Post>(&conn).unwrap();
    let data: Vec<_> = all_posts.iter().flat_map(|post| {
        let post_id = post.id;
        (0..10).map(move |i| {
            let title = format!("Comment {} on post {}", i, post_id);
            (title, post_id)
        })
    }).collect();
    let comment_data: Vec<_> = data.iter().map(|&(ref title, post_id)| {
        NewComment(post_id, &title)
    }).collect();
    batch_insert(&comment_data, comments::table, &conn);

    // ACTUAL BENCHMARK
    b.iter(|| {
        let users = users::table.load::<User>(&conn).unwrap();
        let posts = Post::belonging_to(&users).load::<Post>(&conn).unwrap();
        let comments = Comment::belonging_to(&posts).load::<Comment>(&conn).unwrap()
            .grouped_by(&posts);
        let posts_and_comments = posts.into_iter().zip(comments).grouped_by(&users);
        let result: Vec<(User, Vec<(Post, Vec<Comment>)>)> = users.into_iter().zip(posts_and_comments).collect();
    })
}
