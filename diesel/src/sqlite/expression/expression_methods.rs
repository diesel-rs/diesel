//! Sqlite specific expression methods.

pub(in crate::sqlite) use self::private::{
    BinaryOrNullableBinary, JsonOrNullableJson, JsonOrNullableJsonOrJsonbOrNullableJsonb,
    MaybeNullableValue, NotBlob, TextOrNullableText, TextOrNullableTextOrBinaryOrNullableBinary,
};
use super::operators::*;
use crate::dsl;
use crate::expression::grouped::Grouped;
use crate::expression::{AsExpression, Expression};
use crate::sql_types::SqlType;

/// Sqlite specific methods which are present on all expressions.
#[cfg(feature = "sqlite")]
pub trait SqliteExpressionMethods: Expression + Sized {
    /// Creates a Sqlite `IS` expression.
    ///
    /// The `IS` operator work like = except when one or both of the operands are NULL.
    /// In this case, if both operands are NULL, then the `IS` operator evaluates to true.
    /// If one operand is NULL and the other is not, then the `IS` operator evaluates to false.
    /// It is not possible for an `IS` expression to evaluate to NULL.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let jack_is_a_dog = animals
    ///     .select(name)
    ///     .filter(species.is("dog"))
    ///     .get_results::<Option<String>>(connection)?;
    /// assert_eq!(vec![Some("Jack".to_string())], jack_is_a_dog);
    /// #     Ok(())
    /// # }
    /// ```
    fn is<T>(self, other: T) -> dsl::Is<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(Is::new(self, other.as_expression()))
    }

    /// Creates a Sqlite `IS NOT` expression.
    ///
    /// The `IS NOT` operator work like != except when one or both of the operands are NULL.
    /// In this case, if both operands are NULL, then the `IS NOT` operator evaluates to false.
    /// If one operand is NULL and the other is not, then the `IS NOT` operator is true.
    /// It is not possible for an `IS NOT` expression to evaluate to NULL.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = &mut establish_connection();
    /// let jack_is_not_a_spider = animals
    ///     .select(name)
    ///     .filter(species.is_not("spider"))
    ///     .get_results::<Option<String>>(connection)?;
    /// assert_eq!(vec![Some("Jack".to_string())], jack_is_not_a_spider);
    /// #     Ok(())
    /// # }
    /// ```
    #[allow(clippy::wrong_self_convention)] // This is named after the sql operator
    fn is_not<T>(self, other: T) -> dsl::IsNot<Self, T>
    where
        Self::SqlType: SqlType,
        T: AsExpression<Self::SqlType>,
    {
        Grouped(IsNot::new(self, other.as_expression()))
    }
}

impl<T: Expression> SqliteExpressionMethods for T {}

pub(in crate::sqlite) mod private {
    use crate::sql_types::{
        BigInt, Binary, Bool, Date, Double, Float, Integer, Json, Jsonb, MaybeNullableType,
        Nullable, Numeric, SingleValue, SmallInt, SqlType, Text, Time, Timestamp,
        TimestamptzSqlite,
    };

    #[diagnostic::on_unimplemented(
        message = "`{Self}` is neither `diesel::sql_types::Text` nor `diesel::sql_types::Nullable<Text>`",
        note = "try to provide an expression that produces one of the expected sql types"
    )]
    pub trait TextOrNullableText {}

    impl TextOrNullableText for Text {}
    impl TextOrNullableText for Nullable<Text> {}

    #[diagnostic::on_unimplemented(
        message = "`{Self}` is neither `diesel::sql_types::Binary` nor `diesel::sql_types::Nullable<Binary>`",
        note = "try to provide an expression that produces one of the expected sql types"
    )]
    pub trait BinaryOrNullableBinary {}

    impl BinaryOrNullableBinary for Binary {}
    impl BinaryOrNullableBinary for Nullable<Binary> {}

    #[diagnostic::on_unimplemented(
        message = "`{Self}` is neither `diesel::sql_types::Text`, `diesel::sql_types::Nullable<Text>`, `diesel::sql_types::Binary` nor `diesel::sql_types::Nullable<Binary>`",
        note = "try to provide an expression that produces one of the expected sql types"
    )]
    pub trait TextOrNullableTextOrBinaryOrNullableBinary {}

    impl TextOrNullableTextOrBinaryOrNullableBinary for Text {}
    impl TextOrNullableTextOrBinaryOrNullableBinary for Nullable<Text> {}
    impl TextOrNullableTextOrBinaryOrNullableBinary for Binary {}
    impl TextOrNullableTextOrBinaryOrNullableBinary for Nullable<Binary> {}

    #[diagnostic::on_unimplemented(
        message = "`{Self}` is neither `diesel::sql_types::Json`, `diesel::sql_types::Jsonb`, `diesel::sql_types::Nullable<Json>` nor `diesel::sql_types::Nullable<Jsonb>`",
        note = "try to provide an expression that produces one of the expected sql types"
    )]
    pub trait JsonOrNullableJsonOrJsonbOrNullableJsonb {}
    impl JsonOrNullableJsonOrJsonbOrNullableJsonb for Json {}
    impl JsonOrNullableJsonOrJsonbOrNullableJsonb for Nullable<Json> {}
    impl JsonOrNullableJsonOrJsonbOrNullableJsonb for Jsonb {}
    impl JsonOrNullableJsonOrJsonbOrNullableJsonb for Nullable<Jsonb> {}

    #[diagnostic::on_unimplemented(
        message = "`{Self}` is neither `diesel::sql_types::Json` nor `diesel::sql_types::Nullable<Json>`",
        note = "try to provide an expression that produces one of the expected sql types"
    )]
    pub trait JsonOrNullableJson {}
    impl JsonOrNullableJson for Json {}
    impl JsonOrNullableJson for Nullable<Json> {}

    pub trait MaybeNullableValue<T>: SingleValue {
        type Out: SingleValue;
    }

    impl<T, O> MaybeNullableValue<O> for T
    where
        T: SingleValue,
        T::IsNull: MaybeNullableType<O>,
        <T::IsNull as MaybeNullableType<O>>::Out: SingleValue,
    {
        type Out = <T::IsNull as MaybeNullableType<O>>::Out;
    }

    #[diagnostic::on_unimplemented(
        message = "`{Self}` is neither any of `diesel::sql_types::{{
            Text, Float, Double, Numeric,  Bool, Integer, SmallInt, BigInt, 
            Date, Time, Timestamp, TimestamptzSqlite, Json
         }}`  nor `diesel::sql_types::Nullable<Any of the above>`",
        note = "try to provide an expression that produces one of the expected sql types"
    )]
    pub trait NotBlob: SqlType + SingleValue {}

    impl<T> NotBlob for Nullable<T> where T: NotBlob {}
    impl NotBlob for Text {}
    impl NotBlob for Float {}
    impl NotBlob for Double {}
    impl NotBlob for Numeric {}
    impl NotBlob for Bool {}
    impl NotBlob for Integer {}
    impl NotBlob for SmallInt {}
    impl NotBlob for BigInt {}
    impl NotBlob for Date {}
    impl NotBlob for Time {}
    impl NotBlob for Timestamp {}
    impl NotBlob for TimestamptzSqlite {}
    impl NotBlob for Json {}
}
