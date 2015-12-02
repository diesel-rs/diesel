#![feature(custom_derive, plugin, custom_attribute, test)]
#![plugin(diesel_codegen)]
#[macro_use]
extern crate diesel;
extern crate test;

mod schema;

use self::test::Bencher;
use self::schema::*;
use diesel::*;

#[bench]
fn bench_selecting_0_rows_with_trivial_query(b: &mut Bencher) {
    let conn = connection();
    setup_users_table(&conn);

    b.iter(|| {
        users::table.load(&conn).unwrap().collect::<Vec<User>>();
    })
}

#[bench]
fn bench_selecting_10k_rows_with_trivial_query(b: &mut Bencher) {
    let conn = connection();
    setup_users_table(&conn);
    let data: Vec<_> = (0..10_000).map(|i| {
        NewUser::new(&format!("User {}", i), None)
    }).collect();
    insert(&data).into(users::table).execute(&conn).unwrap();

    b.iter(|| {
        users::table.load(&conn).unwrap().collect::<Vec<User>>()
    })
}

#[bench]
fn bench_selecting_0_rows_with_medium_complex_query(b: &mut Bencher) {
    let conn = connection();
    setup_users_table(&conn);
    setup_posts_table(&conn);

    b.iter(|| {
        use schema::users::dsl::*;
        let target = users.left_outer_join(posts::table)
            .filter(hair_color.eq("black"))
            .order(name.desc());
        target.load(&conn).unwrap().collect::<Vec<(User, Option<Post>)>>()
    })
}

#[bench]
fn bench_selecting_10k_rows_with_medium_complex_query(b: &mut Bencher) {
    let conn = connection();
    setup_users_table(&conn);
    setup_posts_table(&conn);

    let data: Vec<_> = (0..10_000).map(|i| {
        let hair_color = if i % 2 == 0 { "black" } else { "brown" };
        NewUser::new(&format!("User {}", i), Some(hair_color))
    }).collect();
    insert(&data).into(users::table).execute(&conn).unwrap();

    b.iter(|| {
        use schema::users::dsl::*;
        let target = users.left_outer_join(posts::table)
            .filter(hair_color.eq("black"))
            .order(name.desc());
        target.load(&conn).unwrap().collect::<Vec<(User, Option<Post>)>>()
    })
}
