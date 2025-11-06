//! Sqlite specific expression methods.

pub(in crate::sqlite) use self::private::{
    BinaryOrNullableBinary, JsonIndex, JsonOrNullableJson,
    JsonOrNullableJsonOrJsonbOrNullableJsonb, MaybeNullableValue, NotBlob, TextOrNullableText,
    TextOrNullableTextOrBinaryOrNullableBinary,
};
use super::operators::*;
use crate::dsl;
use crate::expression::grouped::Grouped;
use crate::expression::operators::{RetrieveAsObjectJson, RetrieveAsTextJson};
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

/// SQLite specific methods present on JSON and JSONB expressions.
#[cfg(feature = "sqlite")]
pub trait SqliteAnyJsonExpressionMethods: Expression + Sized {
    /// Creates a SQLite `->` expression.
    ///
    /// This operator extracts the value associated with the given path or key from a JSON value.
    /// The right-hand side can be:
    /// - A string path expression (e.g., `"$.key"`, `"$.c"`, or `"c"` which is interpreted as `"$.c"`)
    /// - An integer for array indexing (e.g., `0` for the first element, or `-1` for the last element on SQLite 3.47+)
    ///
    /// **Always returns a TEXT JSON representation** (SQL type `Json`), even when the input is JSONB.
    /// To get JSONB output, use `jsonb_extract()` function instead.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #    contacts {
    /// #        id -> Integer,
    /// #        name -> Text,
    /// #        address -> Json,
    /// #    }
    /// # }
    /// #
    /// # fn main() {
    /// #     #[cfg(feature = "serde_json")]
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # #[cfg(feature = "serde_json")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::contacts::dsl::*;
    /// #     use diesel::dsl::sql;
    /// #     use diesel::sql_types::Json;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS contacts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE contacts (
    /// #         id INTEGER PRIMARY KEY,
    /// #         name TEXT NOT NULL,
    /// #         address TEXT NOT NULL
    /// #     )").execute(conn).unwrap();
    /// #
    /// let json_value = serde_json::json!({
    ///     "street": "Article Circle Expressway 1",
    ///     "city": "North Pole",
    ///     "postcode": "99705",
    ///     "state": "Alaska"
    /// });
    ///
    /// let result = diesel::select(sql::<Json>(r#"json('{"a": {"b": [1, 2, 3]}}')"#)
    ///     .retrieve_as_object("$.a.b[0]"))
    ///     .get_result::<serde_json::Value>(conn)?;
    /// assert_eq!(serde_json::json!(1), result);
    ///
    /// let result = diesel::select(sql::<Json>(r#"json('{"a": [1, 2, 3]}')"#)
    ///     .retrieve_as_object("$.a[1]"))
    ///     .get_result::<serde_json::Value>(conn)?;
    /// assert_eq!(serde_json::json!(2), result);
    ///
    /// #     Ok(())
    /// # }
    /// ```
    fn retrieve_as_object<T>(
        self,
        other: T,
    ) -> crate::sqlite::expression::helper_types::RetrieveAsObjectJson<
        Self,
        T::Expression,
        <T::Expression as Expression>::SqlType,
    >
    where
        T: JsonIndex,
        <T::Expression as Expression>::SqlType: SqlType,
    {
        Grouped(RetrieveAsObjectJson::new(
            self,
            other.into_json_index_expression(),
        ))
    }

    /// Creates a SQLite `->>` expression.
    ///
    /// This operator extracts the value associated with the given path or key from a JSON value
    /// and **returns an SQL value as TEXT** (or INTEGER/REAL/NULL in some cases, but typed as TEXT).
    ///
    /// The right-hand side can be:
    /// - A string path expression (e.g., `"$.key"`, `"$.c"`, or `"c"` which is interpreted as `"$.c"`)
    /// - An integer for array indexing (e.g., `0` for the first element, or `-1` for the last element on SQLite 3.47+)
    ///
    /// Unlike `->`, this operator returns an SQL representation:
    /// - JSON strings are returned without quotes (e.g., `'{"a":"xyz"}' ->> '$.a'` returns `'xyz'`, not `'"xyz"'`)
    /// - JSON null becomes SQL NULL
    /// - Numbers and booleans are returned as text representations
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// #
    /// # table! {
    /// #    contacts {
    /// #        id -> Integer,
    /// #        name -> Text,
    /// #        address -> Json,
    /// #    }
    /// # }
    /// #
    /// # fn main() {
    /// #     #[cfg(feature = "serde_json")]
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # #[cfg(feature = "serde_json")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::contacts::dsl::*;
    /// #     use diesel::dsl::sql;
    /// #     use diesel::sql_types::Json;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS contacts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE contacts (
    /// #         id INTEGER PRIMARY KEY,
    /// #         name TEXT NOT NULL,
    /// #         address TEXT NOT NULL
    /// #     )").execute(conn).unwrap();
    /// #
    /// let result = diesel::select(sql::<Json>(r#"json('{"a": {"b": "test"}}')"#)
    ///     .retrieve_as_text("$.a.b"))
    ///     .get_result::<String>(conn)?;
    /// assert_eq!("test", result);
    ///
    /// let result = diesel::select(sql::<Json>(r#"json('{"a": [1, 2, 3]}')"#)
    ///     .retrieve_as_text("$.a[0]"))
    ///     .get_result::<String>(conn)?;
    /// assert_eq!("1", result);
    ///
    /// #     Ok(())
    /// # }
    /// ```
    fn retrieve_as_text<T>(
        self,
        other: T,
    ) -> crate::sqlite::expression::helper_types::RetrieveAsTextJson<
        Self,
        T::Expression,
        <T::Expression as Expression>::SqlType,
    >
    where
        T: JsonIndex,
        <T::Expression as Expression>::SqlType: SqlType,
    {
        Grouped(RetrieveAsTextJson::new(
            self,
            other.into_json_index_expression(),
        ))
    }
}

#[doc(hidden)]
impl<T> SqliteAnyJsonExpressionMethods for T
where
    T: Expression,
    T::SqlType: JsonOrNullableJsonOrJsonbOrNullableJsonb,
{
}

pub(in crate::sqlite) mod private {
    use crate::expression::{Expression, IntoSql};
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

    #[diagnostic::on_unimplemented(
        message = "`{Self}` is neither `diesel::sql_types::Text` nor `diesel::sql_types::Integer`",
        note = "try to provide an expression that produces one of the expected sql types"
    )]
    pub trait TextOrInteger {}
    impl TextOrInteger for Text {}
    impl TextOrInteger for Integer {}

    /// A trait that describes valid json indices used by SQLite
    pub trait JsonIndex {
        /// The Expression node created by this index type
        type Expression: Expression;

        /// Convert a index value into the corresponding index expression
        fn into_json_index_expression(self) -> Self::Expression;
    }

    impl<'a> JsonIndex for &'a str {
        type Expression = crate::dsl::AsExprOf<&'a str, Text>;

        fn into_json_index_expression(self) -> Self::Expression {
            self.into_sql::<Text>()
        }
    }

    impl JsonIndex for String {
        type Expression = crate::dsl::AsExprOf<String, Text>;

        fn into_json_index_expression(self) -> Self::Expression {
            self.into_sql::<Text>()
        }
    }

    impl JsonIndex for i32 {
        type Expression = crate::dsl::AsExprOf<i32, Integer>;

        fn into_json_index_expression(self) -> Self::Expression {
            self.into_sql::<Integer>()
        }
    }

    impl<T> JsonIndex for T
    where
        T: Expression,
        T::SqlType: TextOrInteger,
    {
        type Expression = Self;

        fn into_json_index_expression(self) -> Self::Expression {
            self
        }
    }
}
