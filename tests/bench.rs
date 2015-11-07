#![feature(test)]
#[macro_use]
extern crate yaqb;
extern crate test;

mod schema;

use self::test::Bencher;
use self::schema::*;

#[bench]
fn bench_selecting_10k_rows(b: &mut Bencher) {
    let conn = connection();
    setup_users_table(&conn);
    let data: Vec<_> = (0..10_000).map(|i| {
        NewUser::new(&format!("User {}", i), None)
    }).collect();
    conn.insert_without_return(&users::table, &data).unwrap();

    b.iter(|| {
        conn.query_all(users::table).unwrap().collect::<Vec<User>>()
    })
}
