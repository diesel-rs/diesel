mod date_and_time;
mod ops;

use schema::{connection, NewUser, setup_users_table};
use schema::users::dsl::*;
use yaqb::*;
use yaqb::query_builder::*;
use yaqb::expression::dsl::*;

#[test]
fn test_count_counts_the_rows() {
    let connection = connection();
    setup_users_table(&connection);
    let source = users.select(count(users.star()));

    assert_eq!(Some(0), source.first(&connection).unwrap());
    connection.insert_returning_count(&users, &NewUser::new("Sean", None)).unwrap();
    assert_eq!(Some(1), source.first(&connection).unwrap());
}

#[test]
fn test_count_star() {
    let connection = connection();
    setup_users_table(&connection);
    let source = users.count();

    assert_eq!(Some(0), source.first(&connection).unwrap());
    connection.insert_returning_count(&users, &NewUser::new("Sean", None)).unwrap();
    assert_eq!(Some(1), source.first(&connection).unwrap());

    // Ensure we're doing COUNT(*) instead of COUNT(table.*) which is going to be more efficient
    let mut query_builder = ::yaqb::query_builder::pg::PgQueryBuilder::new(&connection);
    Expression::to_sql(&source.as_query(), &mut query_builder).unwrap();
    assert!(query_builder.sql.starts_with("SELECT COUNT(*) FROM"));
}

table! {
    numbers (n) {
        n -> Integer,
    }
}

#[test]
fn test_count_max() {
    use self::numbers::columns::*;
    use self::numbers::table as numbers;

    let connection = connection();
    connection.execute("CREATE TABLE numbers (n integer)").unwrap();
    connection.execute("INSERT INTO numbers (n) VALUES (2), (1), (5)").unwrap();
    let source = numbers.select(max(n));

    assert_eq!(Some(5), source.first(&connection).unwrap());
    connection.execute("DELETE FROM numbers WHERE n = 5").unwrap();
    assert_eq!(Some(2), source.first(&connection).unwrap());
}

#[test]
fn max_returns_same_type_as_expression_being_maxed() {
    let connection = connection();
    setup_users_table(&connection);
    let source = users.select(max(name));

    let data: &[_] = &[
        NewUser::new("B", None),
        NewUser::new("C", None),
        NewUser::new("A", None),
    ];
    connection.insert_returning_count(&users, data).unwrap();
    assert_eq!(Some("C".to_string()), source.first(&connection).unwrap());
    connection.execute("DELETE FROM users WHERE name = 'C'").unwrap();
    assert_eq!(Some("B".to_string()), source.first(&connection).unwrap());
}

use std::marker::PhantomData;

struct Arbitrary<T: types::NativeSqlType> {
    _marker: PhantomData<T>,
}

impl<T: types::NativeSqlType> Expression for Arbitrary<T> {
    type SqlType = T;

    fn to_sql<B: QueryBuilder>(&self, _out: &mut B) -> BuildQueryResult {
        Ok(())
    }
}

impl<T: types::NativeSqlType, QS> SelectableExpression<QS> for Arbitrary<T> {}

fn arbitrary<T: types::NativeSqlType>() -> Arbitrary<T> {
    Arbitrary { _marker: PhantomData }
}

#[test]
fn max_accepts_all_numeric_string_and_date_types() {
    let _ = users.select(max(arbitrary::<types::SmallSerial>()));
    let _ = users.select(max(arbitrary::<types::Serial>()));
    let _ = users.select(max(arbitrary::<types::BigSerial>()));
    let _ = users.select(max(arbitrary::<types::SmallInt>()));
    let _ = users.select(max(arbitrary::<types::Integer>()));
    let _ = users.select(max(arbitrary::<types::BigInt>()));
    let _ = users.select(max(arbitrary::<types::Float>()));
    let _ = users.select(max(arbitrary::<types::Double>()));

    let _ = users.select(max(arbitrary::<types::VarChar>()));
    let _ = users.select(max(arbitrary::<types::Text>()));

    let _ = users.select(max(arbitrary::<types::Nullable<types::SmallSerial>>()));
    let _ = users.select(max(arbitrary::<types::Nullable<types::Serial>>()));
    let _ = users.select(max(arbitrary::<types::Nullable<types::BigSerial>>()));
    let _ = users.select(max(arbitrary::<types::Nullable<types::SmallInt>>()));
    let _ = users.select(max(arbitrary::<types::Nullable<types::Integer>>()));
    let _ = users.select(max(arbitrary::<types::Nullable<types::BigInt>>()));
    let _ = users.select(max(arbitrary::<types::Nullable<types::Float>>()));
    let _ = users.select(max(arbitrary::<types::Nullable<types::Double>>()));

    let _ = users.select(max(arbitrary::<types::Nullable<types::VarChar>>()));
    let _ = users.select(max(arbitrary::<types::Nullable<types::Text>>()));
}
