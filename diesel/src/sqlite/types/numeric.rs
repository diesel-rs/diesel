#[cfg(feature = "numeric")]
mod bigdecimal {
    use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};

    use crate::deserialize::{self, FromSql};
    use crate::serialize::{self, IsNull, Output, ToSql};
    use crate::sql_types::{Double, Numeric};
    use crate::sqlite::connection::SqliteValue;
    use crate::sqlite::Sqlite;

    #[cfg(all(feature = "sqlite", feature = "numeric"))]
    impl ToSql<Numeric, Sqlite> for BigDecimal {
        fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
            let x = self
                .to_f64()
                .ok_or_else(|| format!("{self} is not representable as an f64"))?;
            out.set_value(x);
            Ok(IsNull::No)
        }
    }

    #[cfg(all(feature = "sqlite", feature = "numeric"))]
    impl FromSql<Numeric, Sqlite> for BigDecimal {
        fn from_sql(bytes: SqliteValue<'_, '_, '_>) -> deserialize::Result<Self> {
            let x = <f64 as FromSql<Double, Sqlite>>::from_sql(bytes)?;
            BigDecimal::from_f64(x)
                .ok_or_else(|| format!("{x} is not valid decimal number ").into())
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::prelude::*;
        use bigdecimal::{BigDecimal, ToPrimitive};

        table! {
            bigdecimal_test {
                id -> Integer,
                value -> BigInt,
            }
        }

        #[test]
        fn sum_bigdecimal_to_i64() {
            use self::bigdecimal_test::dsl::*;

            let connection = &mut SqliteConnection::establish(":memory:").unwrap();
            crate::sql_query(
                "CREATE TABLE bigdecimal_test (id integer primary key autoincrement, value integer)",
            )
            .execute(connection)
            .unwrap();
            crate::sql_query("INSERT INTO bigdecimal_test (value) VALUES (14), (14), (14)")
                .execute(connection)
                .unwrap();

            let result: Option<BigDecimal> = bigdecimal_test
                .select(crate::dsl::sum(value))
                .first(connection)
                .expect("Summed result");

            let result = match result.map(|r| r.to_i64()) {
                Some(Some(r)) => r,
                Some(None) => i64::MAX,
                None => 0,
            };

            assert_eq!(42i64, result);
        }

        #[test]
        fn sum_bigdecimal_to_f64() {
            use self::bigdecimal_test::dsl::*;

            let connection = &mut SqliteConnection::establish(":memory:").unwrap();
            crate::sql_query(
                "CREATE TABLE bigdecimal_test (id integer primary key autoincrement, value numeric)",
            )
            .execute(connection)
            .unwrap();
            crate::sql_query(
                "INSERT INTO bigdecimal_test (value) VALUES (14.14), (14.14), (14.14)",
            )
            .execute(connection)
            .unwrap();

            let result: Option<BigDecimal> = bigdecimal_test
                .select(crate::dsl::sum(value))
                .first(connection)
                .expect("Summed result");

            let result = match result.map(|r| r.to_f64()) {
                Some(Some(r)) => r,
                Some(None) => f64::MAX,
                None => 0.0,
            };

            assert_eq!(42.42f64, result);
        }
    }
}
