extern crate diesel;

use diesel::sql_types::*;
use diesel::*;

table! {
    users {
        id -> Integer,
        name -> VarChar,
    }
}

table! {
    posts {
        id -> Integer,
        title -> VarChar,
    }
}

allow_tables_to_appear_in_same_query!(users, posts);

#[derive(Queryable)]
struct User {
    id: i32,
    name: String,
}

#[declare_sql_function]
extern "SQL" {
    fn foo(x: Integer) -> Integer;
    fn bar(x: VarChar) -> VarChar;
}

fn main() {
    use self::posts::title;
    use self::users::name;

    let mut conn = PgConnection::establish("").unwrap();

    let _ = users::table.filter(name.eq(foo(1)));
    //~^ ERROR: the trait bound `foo<Bound<Integer, i32>>: AsExpression<Text>` is not satisfied

    let _ = users::table
        .filter(name.eq(bar(title)))
        .load::<User>(&mut conn);
    //~^ ERROR: type mismatch resolving `<table as AppearsInFromClause<table>>::Count == Once`
}
