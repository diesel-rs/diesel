use bigdecimal;

mod date_and_time;
mod ops;

use self::bigdecimal::BigDecimal;
use crate::schema::users::dsl::*;
use crate::schema::{
    connection, connection_with_sean_and_tess_in_users_table, NewUser, TestBackend,
};
use diesel::backend::Backend;
use diesel::dsl::*;
use diesel::expression::TypedExpressionType;
use diesel::query_builder::*;
use diesel::sql_types::SqlType;
use diesel::*;

#[declare_sql_function]
extern "SQL" {
    fn coalesce(
        x: sql_types::Nullable<sql_types::VarChar>,
        y: sql_types::VarChar,
    ) -> sql_types::VarChar;
}

#[cfg(feature = "postgres")]
#[declare_sql_function]
extern "SQL" {
    fn unnest(a: sql_types::Array<sql_types::Int4>) -> sql_types::Int4;
}

#[test]
fn test_count_counts_the_rows() {
    let connection = &mut connection();
    let source = users.select(count(id));

    assert_eq!(Ok(0), source.first(connection));
    insert_into(users)
        .values(&NewUser::new("Sean", None))
        .execute(connection)
        .unwrap();
    assert_eq!(Ok(1), source.first(connection));
}

#[test]
fn test_count_star() {
    let connection = &mut connection();
    let source = users.count();

    assert_eq!(Ok(0), source.first(connection));
    insert_into(users)
        .values(&NewUser::new("Sean", None))
        .execute(connection)
        .unwrap();
    assert_eq!(Ok(1), source.first(connection));

    // Ensure we're doing COUNT(*) instead of COUNT(table.*) which is going to be more efficient
    assert!(debug_query::<TestBackend, _>(&source)
        .to_string()
        .starts_with("SELECT COUNT(*) FROM"));
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

    let connection = &mut connection();
    diesel::sql_query("INSERT INTO numbers (n) VALUES (2), (1), (5)")
        .execute(connection)
        .unwrap();
    let source = numbers.select(max(n));

    assert_eq!(Ok(Some(5)), source.first(connection));
    diesel::sql_query("DELETE FROM numbers WHERE n = 5")
        .execute(connection)
        .unwrap();
    assert_eq!(Ok(Some(2)), source.first(connection));
    diesel::sql_query("DELETE FROM numbers")
        .execute(connection)
        .unwrap();
    assert_eq!(Ok(None::<i32>), source.first(connection));
}

#[cfg(feature = "postgres")]
table! {
    number_arrays (na) {
        na -> Array<Integer>,
    }
}

#[test]
#[cfg(feature = "postgres")]
fn test_min_max_of_array() {
    use self::number_arrays::dsl::*;

    let connection = &mut connection();
    diesel::sql_query("CREATE TABLE number_arrays ( na INTEGER[] PRIMARY KEY )")
        .execute(connection)
        .unwrap();

    insert_into(number_arrays)
        .values(&vec![
            na.eq(vec![1, 1, 100]),
            na.eq(vec![1, 5, 5]),
            na.eq(vec![5, 0]),
        ])
        .execute(connection)
        .unwrap();

    let max_query = number_arrays.select(max(na));
    let min_query = number_arrays.select(min(na));
    assert_eq!(Ok(Some(vec![5, 0])), max_query.first(connection));
    assert_eq!(Ok(Some(vec![1, 1, 100])), min_query.first(connection));

    delete(number_arrays.filter(na.eq(vec![5, 0])))
        .execute(connection)
        .unwrap();
    assert_eq!(Ok(Some(vec![1, 5, 5])), max_query.first(connection));
    assert_eq!(Ok(Some(vec![1, 1, 100])), min_query.first(connection));

    delete(number_arrays.filter(na.eq(vec![1, 1, 100])))
        .execute(connection)
        .unwrap();
    assert_eq!(Ok(Some(vec![1, 5, 5])), max_query.first(connection));
    assert_eq!(Ok(Some(vec![1, 5, 5])), min_query.first(connection));

    delete(number_arrays).execute(connection).unwrap();
    assert_eq!(Ok(None::<Vec<i32>>), max_query.first(connection));
    assert_eq!(Ok(None::<Vec<i32>>), min_query.first(connection));
}

#[test]
fn max_returns_same_type_as_expression_being_maximized() {
    let connection = &mut connection();
    let source = users.select(max(name));

    let data: &[_] = &[
        NewUser::new("B", None),
        NewUser::new("C", None),
        NewUser::new("A", None),
    ];
    insert_into(users).values(data).execute(connection).unwrap();
    assert_eq!(Ok(Some("C".to_string())), source.first(connection));
    diesel::sql_query("DELETE FROM users WHERE name = 'C'")
        .execute(connection)
        .unwrap();
    assert_eq!(Ok(Some("B".to_string())), source.first(connection));
    diesel::sql_query("DELETE FROM users")
        .execute(connection)
        .unwrap();
    assert_eq!(Ok(None::<String>), source.first(connection));
}

use std::marker::PhantomData;

struct Arbitrary<T> {
    _marker: PhantomData<T>,
}

impl<T> Expression for Arbitrary<T>
where
    T: SqlType + TypedExpressionType,
{
    type SqlType = T;
}

impl<T, DB> QueryFragment<DB> for Arbitrary<T>
where
    DB: Backend,
{
    fn walk_ast<'b>(&'b self, _: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        Ok(())
    }
}

impl<T, QS> SelectableExpression<QS> for Arbitrary<T> where Self: Expression {}

impl<T, QS> AppearsOnTable<QS> for Arbitrary<T> where Self: Expression {}

fn arbitrary<T>() -> Arbitrary<T> {
    Arbitrary {
        _marker: PhantomData,
    }
}

#[test]
fn max_accepts_all_numeric_string_and_date_types() {
    let _ = users.select(max(arbitrary::<sql_types::SmallInt>()));
    let _ = users.select(max(arbitrary::<sql_types::Integer>()));
    let _ = users.select(max(arbitrary::<sql_types::BigInt>()));
    let _ = users.select(max(arbitrary::<sql_types::Float>()));
    let _ = users.select(max(arbitrary::<sql_types::Double>()));

    let _ = users.select(max(arbitrary::<sql_types::VarChar>()));
    let _ = users.select(max(arbitrary::<sql_types::Text>()));

    let _ = users.select(max(arbitrary::<sql_types::Nullable<sql_types::SmallInt>>()));
    let _ = users.select(max(arbitrary::<sql_types::Nullable<sql_types::Integer>>()));
    let _ = users.select(max(arbitrary::<sql_types::Nullable<sql_types::BigInt>>()));
    let _ = users.select(max(arbitrary::<sql_types::Nullable<sql_types::Float>>()));
    let _ = users.select(max(arbitrary::<sql_types::Nullable<sql_types::Double>>()));

    let _ = users.select(max(arbitrary::<sql_types::Nullable<sql_types::VarChar>>()));
    let _ = users.select(max(arbitrary::<sql_types::Nullable<sql_types::Text>>()));
}

#[test]
fn test_min() {
    use self::numbers::columns::*;
    use self::numbers::table as numbers;

    let connection = &mut connection();
    diesel::sql_query("INSERT INTO numbers (n) VALUES (2), (1), (5)")
        .execute(connection)
        .unwrap();
    let source = numbers.select(min(n));

    assert_eq!(Ok(Some(1)), source.first(connection));
    diesel::sql_query("DELETE FROM numbers WHERE n = 1")
        .execute(connection)
        .unwrap();
    assert_eq!(Ok(Some(2)), source.first(connection));
    diesel::sql_query("DELETE FROM numbers")
        .execute(connection)
        .unwrap();
    assert_eq!(Ok(None::<i32>), source.first(connection));
}

#[test]
fn function_with_multiple_arguments() {
    use crate::schema::users::dsl::*;

    let connection = &mut connection();
    let new_users = vec![
        NewUser::new("Sean", Some("black")),
        NewUser::new("Tess", None),
    ];
    insert_into(users)
        .values(&new_users)
        .execute(connection)
        .unwrap();

    let expected_data = vec!["black".to_string(), "Tess".to_string()];
    let data = users
        .select(coalesce(hair_color, name))
        .order(id)
        .load::<String>(connection);

    assert_eq!(Ok(expected_data), data);
}

#[test]
fn test_sum() {
    use self::numbers::columns::*;
    use self::numbers::table as numbers;

    let connection = &mut connection();
    diesel::sql_query("INSERT INTO numbers (n) VALUES (2), (1), (5)")
        .execute(connection)
        .unwrap();
    let source = numbers.select(sum(n));

    assert_eq!(Ok(Some(8)), source.first(connection));
    diesel::sql_query("DELETE FROM numbers WHERE n = 2")
        .execute(connection)
        .unwrap();
    assert_eq!(Ok(Some(6)), source.first(connection));
    diesel::sql_query("DELETE FROM numbers")
        .execute(connection)
        .unwrap();
    assert_eq!(Ok(None::<i64>), source.first(connection));
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

    let connection = &mut connection();
    diesel::sql_query("INSERT INTO precision_numbers (n) VALUES (2), (1), (5.5)")
        .execute(connection)
        .unwrap();
    let source = numbers.select(sum(n));

    assert_eq!(Ok(Some(8.5f64)), source.first(connection));
    diesel::sql_query("DELETE FROM precision_numbers WHERE n = 2")
        .execute(connection)
        .unwrap();
    assert_eq!(Ok(Some(6.5f64)), source.first(connection));
    diesel::sql_query("DELETE FROM precision_numbers")
        .execute(connection)
        .unwrap();
    assert_eq!(Ok(None::<f64>), source.first(connection));
}

table! {
    nullable_doubles {
        id -> Integer,
        n -> Nullable<Double>,
    }
}

#[test]
fn test_sum_for_nullable() {
    use self::nullable_doubles::columns::*;
    use self::nullable_doubles::table as numbers;

    let connection = &mut connection();
    diesel::sql_query("INSERT INTO nullable_doubles (n) VALUES (null), (null), (5.5)")
        .execute(connection)
        .unwrap();
    let source = numbers.select(sum(n));

    assert_eq!(Ok(Some(5.5f64)), source.first(connection));
    diesel::sql_query("DELETE FROM nullable_doubles WHERE n = 5.5")
        .execute(connection)
        .unwrap();
    assert_eq!(Ok(None), source.first::<Option<f64>>(connection));
}

#[test]
fn test_avg() {
    use self::precision_numbers::columns::*;
    use self::precision_numbers::table as numbers;

    let connection = &mut connection();
    diesel::sql_query("INSERT INTO precision_numbers (n) VALUES (2), (1), (6)")
        .execute(connection)
        .unwrap();
    let source = numbers.select(avg(n));

    assert_eq!(Ok(Some(3f64)), source.first(connection));
    diesel::sql_query("DELETE FROM precision_numbers WHERE n = 2")
        .execute(connection)
        .unwrap();
    assert_eq!(Ok(Some(3.5f64)), source.first(connection));
    diesel::sql_query("DELETE FROM precision_numbers")
        .execute(connection)
        .unwrap();
    assert_eq!(Ok(None::<f64>), source.first(connection));
}

#[test]
fn test_avg_integer() {
    let conn = &mut connection_with_sean_and_tess_in_users_table();
    let avg_id = users.select(avg(id)).get_result(conn);
    let expected = "1.5".parse::<BigDecimal>().unwrap();
    assert_eq!(Ok(Some(expected)), avg_id);
}

#[test]
fn test_avg_for_nullable() {
    use self::nullable_doubles::columns::*;
    use self::nullable_doubles::table as numbers;

    let connection = &mut connection();
    diesel::sql_query("INSERT INTO nullable_doubles (n) VALUES (null), (null), (6)")
        .execute(connection)
        .unwrap();
    let source = numbers.select(avg(n));

    assert_eq!(Ok(Some(6f64)), source.first(connection));
    diesel::sql_query("DELETE FROM nullable_doubles WHERE n = 6")
        .execute(connection)
        .unwrap();
    assert_eq!(Ok(None), source.first::<Option<f64>>(connection));
}

#[test]
#[cfg(feature = "postgres")] // FIXME: We need to test this on SQLite when we support these types
fn test_avg_for_integer() {
    use self::numbers::columns::*;
    use self::numbers::table as numbers;

    let connection = &mut connection();
    diesel::sql_query("INSERT INTO numbers (n) VALUES (2), (1), (6)")
        .execute(connection)
        .unwrap();
    let source = numbers.select(avg(n));

    let result = source.first(connection);
    let expected_result = data_types::PgNumeric::Positive {
        digits: vec![3],
        weight: 0,
        scale: 16,
    };
    assert_eq!(Ok(Some(expected_result)), result);

    diesel::sql_query("DELETE FROM numbers WHERE n = 2")
        .execute(connection)
        .unwrap();
    let result = source.first(connection);
    let expected_result = data_types::PgNumeric::Positive {
        digits: vec![3, 5000],
        weight: 0,
        scale: 16,
    };
    assert_eq!(Ok(Some(expected_result)), result);
}

table! {
    numeric (n) {
        n -> Numeric,
    }
}

#[test]
#[cfg(feature = "postgres")] // FIXME: We need to test this on MySQL
fn test_avg_for_numeric() {
    use self::numeric::columns::*;
    use self::numeric::table as numeric;

    let connection = &mut connection();
    diesel::sql_query("CREATE TABLE numeric (n NUMERIC(8,2))")
        .execute(connection)
        .unwrap();
    diesel::sql_query("INSERT INTO numeric (n) VALUES (2), (1), (6)")
        .execute(connection)
        .unwrap();
    let source = numeric.select(avg(n));

    let result = source.first(connection);
    let expected_result = data_types::PgNumeric::Positive {
        digits: vec![3],
        weight: 0,
        scale: 16,
    };
    assert_eq!(Ok(Some(expected_result)), result);
}

#[test]
#[cfg(feature = "postgres")]
fn test_arrays_a() {
    let connection = &mut connection();

    use diesel::sql_types::Int4;
    let value = select(array::<Int4, _>((1, 2)))
        .get_result::<Vec<i32>>(connection)
        .unwrap();

    assert_eq!(value, vec![1, 2]);
}

#[test]
#[cfg(feature = "postgres")]
fn test_arrays_b() {
    use self::numbers::columns::*;
    use self::numbers::table as numbers;

    let connection = &mut connection();
    diesel::sql_query("INSERT INTO numbers (n) VALUES (7)")
        .execute(connection)
        .unwrap();

    let value = numbers
        .select(unnest(array((n, n + n))))
        .load::<i32>(connection)
        .unwrap();

    assert_eq!(value, vec![7, 14]);
}

#[test]
#[cfg(feature = "postgres")]
fn test_arrays_c() {
    use self::numbers::columns::*;
    use self::numbers::table as numbers;

    let connection = &mut connection();
    diesel::sql_query("INSERT INTO numbers (n) VALUES (7), (8)")
        .execute(connection)
        .unwrap();

    let value = diesel::select(array(numbers.select(n).order_by(n)))
        .first::<Vec<i32>>(connection)
        .unwrap();

    assert_eq!(value, vec![7, 8]);
}

#[test]
fn test_operator_precedence() {
    use self::numbers;

    let connection = &mut connection();
    diesel::sql_query("INSERT INTO numbers (n) VALUES (2)")
        .execute(connection)
        .unwrap();
    let source = numbers::table.select(numbers::n.gt(0).eq(true));

    assert_eq!(Ok(true), source.first(connection));
}
