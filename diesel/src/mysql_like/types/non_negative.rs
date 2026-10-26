use core::error::Error;
use core::fmt::Display;
use core::ops::Deref;

use diesel_derives::FromSqlRow;
use num_traits::Num;

use crate::backend::Backend;
use crate::deserialize::FromSql;
use crate::expression::AsExpression;
use crate::expression::bound::Bound;
use crate::serialize::ToSql;
use crate::sql_types::{Nullable, SingleValue, Unsigned};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, FromSqlRow)]
/// Wrapper type, for Unsigned<ST> in Mariadb and Mysql
pub struct NonNegative<T>(T);

impl<T> NonNegative<T>
where
    T: Num + PartialOrd,
{
    /// Creates a `NonNegative<T>`
    ///
    /// # Arguments
    /// * `value` - The value to be wrapped.
    ///
    /// # Returns
    /// The wrapped value if it is greater or equal to zero.
    ///
    /// # Errors
    /// If the provided value is less than zero.
    pub fn new(value: T) -> Option<Self> {
        (value >= T::zero()).then_some(Self(value))
    }

    /// Extracts the raw inner type
    pub fn into_inner(self) -> T {
        self.0
    }
}

macro_rules! impl_tryfrom {
    ($type:ty) => {
        impl TryFrom<$type> for NonNegative<$type> {
            type Error = NumericIsNegative;

            fn try_from(value: $type) -> Result<Self, Self::Error> {
                Self::new(value).ok_or(NumericIsNegative)
            }
        }
    };
}

impl_tryfrom!(f32);
impl_tryfrom!(f64);

#[cfg(feature = "numeric")]
impl_tryfrom!(bigdecimal::BigDecimal);

#[derive(Debug, Clone, Copy)]
pub struct NumericIsNegative;

impl Display for NumericIsNegative {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Cannot create NonNegative from negative numeric")
    }
}

impl Error for NumericIsNegative {}

impl<T> Deref for NonNegative<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, ST> AsExpression<Unsigned<ST>> for NonNegative<T>
where
    ST: SingleValue,
    T: AsExpression<ST>,
{
    type Expression = Bound<Unsigned<ST>, Self>;

    fn as_expression(self) -> Self::Expression {
        Bound::new(self)
    }
}

impl<T, ST> AsExpression<Nullable<Unsigned<ST>>> for NonNegative<T>
where
    ST: SingleValue,
    T: AsExpression<Nullable<ST>>,
{
    type Expression = Bound<Nullable<Unsigned<ST>>, Self>;

    fn as_expression(self) -> Self::Expression {
        Bound::new(self)
    }
}

impl<T, ST> AsExpression<Unsigned<ST>> for &NonNegative<T>
where
    ST: SingleValue,
    T: AsExpression<ST>,
{
    type Expression = Bound<Unsigned<ST>, Self>;

    fn as_expression(self) -> Self::Expression {
        Bound::new(self)
    }
}

impl<T, ST> AsExpression<Nullable<Unsigned<ST>>> for &NonNegative<T>
where
    ST: SingleValue,
    T: AsExpression<Nullable<ST>>,
{
    type Expression = Bound<Nullable<Unsigned<ST>>, Self>;

    fn as_expression(self) -> Self::Expression {
        Bound::new(self)
    }
}

impl<T, ST> AsExpression<Unsigned<ST>> for &&NonNegative<T>
where
    ST: SingleValue,
    T: AsExpression<ST>,
{
    type Expression = Bound<Unsigned<ST>, Self>;

    fn as_expression(self) -> Self::Expression {
        Bound::new(self)
    }
}

impl<T, ST> AsExpression<Nullable<Unsigned<ST>>> for &&NonNegative<T>
where
    ST: SingleValue,
    T: AsExpression<Nullable<ST>>,
{
    type Expression = Bound<Nullable<Unsigned<ST>>, Self>;

    fn as_expression(self) -> Self::Expression {
        Bound::new(self)
    }
}

impl<DB, T, ST> ToSql<Nullable<ST>, DB> for NonNegative<T>
where
    DB: Backend,
    NonNegative<T>: ToSql<ST, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut crate::serialize::Output<'b, '_, DB>,
    ) -> crate::serialize::Result {
        ToSql::<ST, DB>::to_sql(self, out)
    }
}

impl<DB, T, ST> ToSql<Unsigned<ST>, DB> for NonNegative<T>
where
    T: ToSql<ST, DB>,
    DB: Backend,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut crate::serialize::Output<'b, '_, DB>,
    ) -> crate::serialize::Result {
        self.0.to_sql(out)
    }
}

impl<DB, T, ST> FromSql<Unsigned<ST>, DB> for NonNegative<T>
where
    T: FromSql<ST, DB>,
    DB: Backend,
{
    fn from_sql(bytes: <DB as Backend>::RawValue<'_>) -> crate::deserialize::Result<Self> {
        T::from_sql(bytes).map(NonNegative)
    }
}

#[cfg(test)]
mod tests {
    use super::NonNegative;
    use super::NumericIsNegative;
    use crate::query_dsl::QueryDsl;
    use crate::query_dsl::RunQueryDsl;
    use crate::*;
    use core::assert_matches;
    use core::ops::Deref;

    #[test]
    fn invalid_unsigned() {
        let res = NonNegative::<f64>::try_from(-0.0001);
        assert_matches!(res, Err(NumericIsNegative));
        assert_eq!(
            "Cannot create NonNegative from negative numeric",
            format!("{}", NumericIsNegative)
        )
    }

    #[test]
    fn check_into_inner() {
        let non_neg = NonNegative::new(1.0f32).unwrap();
        assert_eq!(&1.0f32, non_neg.deref());
        assert_eq!(1.0f32, non_neg.into_inner())
    }

    #[diesel_test_helper::test]
    fn fun_with_unsigned() {
        diesel::table! {
            test_table(a){
                a -> Unsigned<Float>,
                b -> Unsigned<Float>,
                c -> Unsigned<Float>,
            }
        }

        use test_table::dsl::{a, b, c};

        let conn = &mut crate::test_helpers::connection();

        crate::sql_query(
            "CREATE TEMPORARY TABLE test_table (
                    a FLOAT UNSIGNED NOT NULL,
                    b FLOAT UNSIGNED NOT NULL,
                    c FLOAT UNSIGNED NOT NULL
            )",
        )
        .execute(conn)
        .unwrap();

        insert_into(test_table::table)
            .values((
                a.eq(NonNegative(3.0f32)),
                b.eq(&NonNegative(5.0f32)),
                c.eq(&&NonNegative(5.0f32)),
            ))
            .execute(conn)
            .unwrap();

        let res = test_table::table
            .select(a - b)
            .get_result::<NonNegative<f32>>(conn)
            .unwrap();

        assert_eq!(res, NonNegative(-2.0))
    }

    #[diesel_test_helper::test]
    fn insert_and_select_unsigned_double() {
        diesel::table! {
            test_table(a){
                a -> Nullable<Unsigned<Double>>,
                b -> Nullable<Unsigned<Double>>,
                c -> Nullable<Unsigned<Double>>,
            }
        }

        use test_table::dsl::{a, b, c};

        let conn = &mut crate::test_helpers::connection();

        crate::sql_query(
            "CREATE TEMPORARY TABLE test_table (
                    a DOUBLE UNSIGNED NULL,
                    b DOUBLE UNSIGNED NULL,
                    c DOUBLE UNSIGNED NULL
            )",
        )
        .execute(conn)
        .unwrap();

        insert_into(test_table::table)
            .values((
                a.eq(NonNegative(3.0f64)),
                b.eq(&NonNegative(5.0f64)),
                c.eq(&&NonNegative(5.0f64)),
            ))
            .execute(conn)
            .unwrap();

        let res = test_table::table
            .select(a)
            .get_result::<Option<NonNegative<f64>>>(conn)
            .unwrap();

        assert_eq!(res, Some(NonNegative(3.0)));

        insert_into(test_table::table)
            .values((
                a.eq(NonNegative(-3.0f64)),
                b.eq(&NonNegative(-5.0f64)),
                c.eq(&&NonNegative(-5.0f64)),
            ))
            .execute(conn)
            .unwrap_err();
    }
}
