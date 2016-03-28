#![feature(custom_derive, plugin, custom_attribute, test)]
#![plugin(diesel_codegen, dotenv_macros)]
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

    b.iter(|| {
        users::table.load::<User>(&conn).unwrap();
    })
}

#[bench]
fn bench_selecting_10k_rows_with_trivial_query(b: &mut Bencher) {
    let conn = connection();
    let data: Vec<_> = (0..10_000).map(|i| {
        NewUser::new(&format!("User {}", i), None)
    }).collect();
    batch_insert(&data, users::table, &conn);

    b.iter(|| {
        users::table.load::<User>(&conn).unwrap()
    })
}

#[bench]
fn bench_selecting_10k_rows_with_trivial_query_boxed(b: &mut Bencher) {
    let conn = connection();
    let data: Vec<_> = (0..10_000).map(|i| {
        NewUser::new(&format!("User {}", i), None)
    }).collect();
    batch_insert(&data, users::table, &conn);

    b.iter(|| {
        users::table.into_boxed().load::<User>(&conn).unwrap()
    })
}


#[bench]
fn bench_selecting_0_rows_with_medium_complex_query(b: &mut Bencher) {
    let conn = connection();

    b.iter(|| {
        use schema::users::dsl::*;
        let target = users.left_outer_join(posts::table)
            .filter(hair_color.eq("black"))
            .order(name.desc());
        target.load::<(User, Option<Post>)>(&conn).unwrap()
    })
}

#[bench]
fn bench_selecting_10k_rows_with_medium_complex_query(b: &mut Bencher) {
    let conn = connection();

    let data: Vec<_> = (0..10_000).map(|i| {
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
fn bench_selecting_10k_rows_with_medium_complex_query_boxed(b: &mut Bencher) {
    let conn = connection();

    let data: Vec<_> = (0..10_000).map(|i| {
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
