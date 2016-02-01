#[cfg(feature = "postgres")] // FIXME: We need to test this on SQLite when we support these types
mod date_and_time;
mod ops;

use schema::{connection, NewUser, batch_insert};
use schema::users::dsl::*;
use diesel::*;
use diesel::backend::Backend;
use diesel::query_builder::*;
use diesel::expression::dsl::*;

#[test]
fn test_count_counts_the_rows() {
    let connection = connection();
    let source = users.select(count(id));

    assert_eq!(Ok(0), source.first(&connection));
    insert(&NewUser::new("Sean", None)).into(users).execute(&connection).unwrap();
    assert_eq!(Ok(1), source.first(&connection));
}

#[test]
fn test_count_star() {
    let connection = connection();
    let source = users.count();

    assert_eq!(Ok(0), source.first(&connection));
    insert(&NewUser::new("Sean", None)).into(users).execute(&connection).unwrap();
    assert_eq!(Ok(1), source.first(&connection));

    // Ensure we're doing COUNT(*) instead of COUNT(table.*) which is going to be more efficient
    assert!(debug_sql!(source).starts_with("SELECT COUNT(*) FROM"));
}

use diesel::types::VarChar;
sql_function!(lower, lower_t, (x: VarChar) -> VarChar);

#[test]
fn test_with_expression_aliased() {
    let n = lower("sean").aliased("n");
    assert_eq!(
        "SELECT `users`.`id` FROM `users`, lower(?) `n` WHERE `n` = ?",
        debug_sql!(users.with(n).filter(n.eq("Jim")).select(id))
    );
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

    assert_eq!(Ok(5), source.first(&connection));
    connection.execute("DELETE FROM numbers WHERE n = 5").unwrap();
    assert_eq!(Ok(2), source.first(&connection));
}

#[test]
fn max_returns_same_type_as_expression_being_maximized() {
    let connection = connection();
    let source = users.select(max(name));

    let data: &[_] = &[
        NewUser::new("B", None),
        NewUser::new("C", None),
        NewUser::new("A", None),
    ];
    batch_insert(data, users, &connection);
    assert_eq!(Ok("C".to_string()), source.first(&connection));
    connection.execute("DELETE FROM users WHERE name = 'C'").unwrap();
    assert_eq!(Ok("B".to_string()), source.first(&connection));
}

use std::marker::PhantomData;

struct Arbitrary<T> {
    _marker: PhantomData<T>,
}

impl<T> Expression for Arbitrary<T> {
    type SqlType = T;
}

impl<T, DB> QueryFragment<DB> for Arbitrary<T> where
    DB: Backend,
{
    fn to_sql(&self, _out: &mut DB::QueryBuilder) -> BuildQueryResult {
        Ok(())
    }
}

impl<T, QS> SelectableExpression<QS> for Arbitrary<T> {}

fn arbitrary<T>() -> Arbitrary<T> {
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

#[test]
fn test_min() {
    use self::numbers::columns::*;
    use self::numbers::table as numbers;

    let connection = connection();
    connection.execute("CREATE TABLE numbers (n integer)").unwrap();
    connection.execute("INSERT INTO numbers (n) VALUES (2), (1), (5)").unwrap();
    let source = numbers.select(min(n));

    assert_eq!(Ok(1), source.first(&connection));
    connection.execute("DELETE FROM numbers WHERE n = 1").unwrap();
    assert_eq!(Ok(2), source.first(&connection));
}

sql_function!(coalesce, coalesce_t, (x: types::Nullable<types::VarChar>, y: types::VarChar) -> types::VarChar);

#[test]
fn function_with_multiple_arguments() {
    use schema::users::dsl::*;

    let connection = connection();
    let new_users = vec![NewUser::new("Sean", Some("black")), NewUser::new("Tess", None)];
    batch_insert(&new_users, users, &connection);

    let expected_data = vec!["black".to_string(), "Tess".to_string()];
    let data: QueryResult<Vec<String>> = users.select(coalesce(hair_color, name))
        .load(&connection).map(Iterator::collect);

    assert_eq!(Ok(expected_data), data);
}

#[test]
fn test_sum() {
    use self::numbers::columns::*;
    use self::numbers::table as numbers;

    let connection = connection();
    connection.execute("CREATE TABLE numbers (n INTEGER)").unwrap();
    connection.execute("INSERT INTO numbers (n) VALUES (2), (1), (5)").unwrap();
    let source = numbers.select(sum(n));

    assert_eq!(Ok(8), source.first(&connection));
    connection.execute("DELETE FROM numbers WHERE n = 2").unwrap();
    assert_eq!(Ok(6), source.first(&connection));
}

table! {
    precision_numbers (n) {
        n -> Double,
    }
}

#[test]
fn test_sum_for_double() {
    use self::precision_numbers::columns::*;
    use self::precision_numbers::table as numbers;

    let connection = connection();
    connection.execute("CREATE TABLE precision_numbers (n DOUBLE PRECISION)").unwrap();
    connection.execute("INSERT INTO precision_numbers (n) VALUES (2), (1), (5.5)").unwrap();
    let source = numbers.select(sum(n));

    assert_eq!(Ok(8.5f64), source.first(&connection));
    connection.execute("DELETE FROM precision_numbers WHERE n = 2").unwrap();
    assert_eq!(Ok(6.5f64), source.first(&connection));
}

table! {
    nullable_doubles {
        id -> Serial,
        n -> Nullable<Double>,
    }
}

#[test]
fn test_sum_for_nullable() {
    use self::nullable_doubles::columns::*;
    use self::nullable_doubles::table as numbers;

    let connection = connection();
    connection.execute("CREATE TABLE nullable_doubles (n DOUBLE PRECISION)").unwrap();
    connection.execute("INSERT INTO nullable_doubles (n) VALUES (null), (null), (5.5)").unwrap();
    let source = numbers.select(sum(n));

    assert_eq!(Ok(Some(5.5f64)), source.first(&connection));
    connection.execute("DELETE FROM nullable_doubles WHERE n = 5.5").unwrap();
    assert_eq!(Ok(None), source.first::<Option<f64>>(&connection));
}

#[test]
fn test_avg() {
    use self::precision_numbers::columns::*;
    use self::precision_numbers::table as numbers;

    let connection = connection();
    connection.execute("CREATE TABLE precision_numbers (n DOUBLE PRECISION)").unwrap();
    connection.execute("INSERT INTO precision_numbers (n) VALUES (2), (1), (6)").unwrap();
    let source = numbers.select(avg(n));

    assert_eq!(Ok(3f64), source.first(&connection));
    connection.execute("DELETE FROM precision_numbers WHERE n = 2").unwrap();
    assert_eq!(Ok(3.5f64), source.first(&connection));
}


#[test]
fn test_avg_for_nullable() {
    use self::nullable_doubles::columns::*;
    use self::nullable_doubles::table as numbers;

    let connection = connection();
    connection.execute("CREATE TABLE nullable_doubles (n DOUBLE PRECISION)").unwrap();
    connection.execute("INSERT INTO nullable_doubles (n) VALUES (null), (null), (6)").unwrap();
    let source = numbers.select(avg(n));

    assert_eq!(Ok(Some(6f64)), source.first(&connection));
    connection.execute("DELETE FROM nullable_doubles WHERE n = 6").unwrap();
    assert_eq!(Ok(None), source.first::<Option<f64>>(&connection));
}

#[test]
#[cfg(feature = "postgres")] // FIXME: We need to test this on SQLite when we support these types
fn test_avg_for_integer() {
    use self::numbers::columns::*;
    use self::numbers::table as numbers;

    let connection = connection();
    connection.execute("CREATE TABLE numbers (n INTEGER)").unwrap();
    connection.execute("INSERT INTO numbers (n) VALUES (2), (1), (6)").unwrap();
    let source = numbers.select(avg(n));

    let result = source.first::<data_types::PgNumeric>(&connection);
    let expected_result = data_types::PgNumeric::Positive {
        digits: vec![3],
        weight: 0,
        scale: 16,
    };
    assert_eq!(Ok(expected_result), result);

    connection.execute("DELETE FROM numbers WHERE n = 2").unwrap();
    let result = source.first::<data_types::PgNumeric>(&connection);
    let expected_result = data_types::PgNumeric::Positive {
        digits: vec![3, 5000],
        weight: 0,
        scale: 16,
    };
    assert_eq!(Ok(expected_result), result);
}

table! {
    numeric (n) {
        n -> Numeric,
    }
}

#[test]
#[cfg(feature = "postgres")] // FIXME: We need to test this on SQLite
fn test_avg_for_numeric() {
    use self::numeric::columns::*;
    use self::numeric::table as numeric;

    let connection = connection();
    connection.execute("CREATE TABLE numeric (n NUMERIC(8,2))").unwrap();
    connection.execute("INSERT INTO numeric (n) VALUES (2), (1), (6)").unwrap();
    let source = numeric.select(avg(n));

    let result = source.first::<data_types::PgNumeric>(&connection);
    let expected_result = data_types::PgNumeric::Positive {
        digits: vec![3],
        weight: 0,
        scale: 16,
    };
    assert_eq!(Ok(expected_result), result);
}
