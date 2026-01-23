use crate::expression::Expression;
use crate::expression::grouped::Grouped;
use crate::expression::operators::RetrieveAsTextJson;
use crate::sql_types::SqlType;

/// PostgreSQL specific methods present on JSON and JSONB expressions.
#[cfg(any(feature = "postgres_backend", feature = "sqlite"))]
pub trait AnyJsonExpressionMethods: Expression + Sized {
    /// Creates a `->>` expression JSON.
    ///
    /// This operator extracts the value associated with the given key, that is provided on the
    /// Right Hand Side of the operator.
    ///
    /// Extracts n'th element of JSON array (array elements are indexed from zero, but negative integers count from the end).
    /// Extracts JSON object field as Text with the given key.
    /// # Example
    ///
    /// ```rust
    /// # include!("../doctest_setup.rs");
    /// #
    /// # table! {
    /// #    contacts {
    /// #        id -> Integer,
    /// #        name -> VarChar,
    /// #        address -> Jsonb,
    /// #    }
    /// # }
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    ///
    /// # #[cfg(feature = "serde_json")]
    /// # fn run_test() -> QueryResult<()> {
    /// #     use self::contacts::dsl::*;
    /// #     let conn = &mut establish_connection();
    /// #     diesel::sql_query("DROP TABLE IF EXISTS contacts").execute(conn).unwrap();
    /// #     diesel::sql_query("CREATE TABLE contacts (
    /// #         id SERIAL PRIMARY KEY,
    /// #         name VARCHAR NOT NULL,
    /// #         address JSONB NOT NULL
    /// #     )").execute(conn)
    /// #        .unwrap();
    /// #
    /// let santas_address: serde_json::Value = serde_json::json!({
    ///     "street": "Article Circle Expressway 1",
    ///     "city": "North Pole",
    ///     "postcode": "99705",
    ///     "state": "Alaska"
    /// });
    /// diesel::insert_into(contacts)
    ///     .values((name.eq("Claus"), address.eq(&santas_address)))
    ///     .execute(conn)?;
    ///
    /// let santas_postcode = contacts.select(address.retrieve_as_text("postcode")).get_result::<String>(conn)?;
    /// assert_eq!(santas_postcode, "99705");
    ///
    ///
    /// let robert_downey_jr_addresses: serde_json::Value = serde_json::json!([
    ///     {
    ///         "street": "Somewhere In La 251",
    ///         "city": "Los Angeles",
    ///         "postcode": "12231223",
    ///         "state": "California"
    ///     },
    ///     {
    ///         "street": "Somewhere In Ny 251",
    ///         "city": "New York",
    ///         "postcode": "3213212",
    ///         "state": "New York"
    ///     }
    /// ]);
    ///
    /// diesel::insert_into(contacts)
    ///     .values((name.eq("Robert Downey Jr."), address.eq(&robert_downey_jr_addresses)))
    ///     .execute(conn)?;
    ///
    /// let roberts_second_address_in_db = contacts
    ///                             .filter(name.eq("Robert Downey Jr."))
    ///                             .select(address.retrieve_as_text(1))
    ///                             .get_result::<String>(conn)?;
    ///
    /// let roberts_second_address = serde_json::json!{{
    ///     "city": "New York",
    ///     "state": "New York",
    ///     "street": "Somewhere In Ny 251",
    ///     "postcode": "3213212"
    ///     }};
    /// assert_eq!(roberts_second_address, serde_json::from_str::<serde_json::Value>(&roberts_second_address_in_db).unwrap());
    /// #     Ok(())
    /// # }
    /// # #[cfg(not(feature = "serde_json"))]
    /// # fn run_test() -> QueryResult<()> {
    /// #     Ok(())
    /// # }
    /// ```
    fn retrieve_as_text<T>(
        self,
        other: T,
    ) -> crate::expression::helper_types::RetrieveAsText<Self, T>
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

/// A marker trait indicating which types can be used as index into a json field
pub trait JsonIndex: self::private::Sealed {
    #[doc(hidden)]
    type Expression: Expression;

    #[doc(hidden)]
    fn into_json_index_expression(self) -> Self::Expression;
}

impl<T> AnyJsonExpressionMethods for T
where
    T: Expression,
    T::SqlType: private::JsonOrNullableJsonOrJsonbOrNullableJsonb,
{
}

pub(crate) mod private {
    use super::JsonIndex;
    use crate::Expression;
    use crate::expression::IntoSql;
    use crate::sql_types::{Integer, Json, Jsonb, Nullable, Text};

    pub trait Sealed {}

    #[diagnostic::on_unimplemented(
        message = "`{Self}` is neither `diesel::sql_types::Text` nor `diesel::sql_types::Integer`",
        note = "try to provide an expression that produces one of the expected sql types"
    )]
    pub trait TextOrInteger {}
    impl TextOrInteger for Text {}
    impl TextOrInteger for Integer {}

    /// Marker trait used to implement `PgAnyJsonExpressionMethods` on the appropriate types.
    #[diagnostic::on_unimplemented(
        message = "`{Self}` is neither `diesel::sql_types::Json`, `diesel::sql_types::Jsonb`, `diesel::sql_types::Nullable<Json>` nor `diesel::sql_types::Nullable<Jsonb>`",
        note = "try to provide an expression that produces one of the expected sql types"
    )]
    pub trait JsonOrNullableJsonOrJsonbOrNullableJsonb {}
    impl JsonOrNullableJsonOrJsonbOrNullableJsonb for Json {}
    impl JsonOrNullableJsonOrJsonbOrNullableJsonb for Nullable<Json> {}
    impl JsonOrNullableJsonOrJsonbOrNullableJsonb for Jsonb {}
    impl JsonOrNullableJsonOrJsonbOrNullableJsonb for Nullable<Jsonb> {}

    impl Sealed for &'_ str {}
    impl Sealed for String {}
    impl Sealed for i32 {}
    impl<T> Sealed for T
    where
        T: Expression,
        T::SqlType: TextOrInteger,
    {
    }

    impl<'a> JsonIndex for &'a str {
        type Expression = crate::dsl::AsExprOf<&'a str, crate::sql_types::Text>;

        fn into_json_index_expression(self) -> Self::Expression {
            self.into_sql::<Text>()
        }
    }

    impl JsonIndex for String {
        type Expression = crate::dsl::AsExprOf<String, crate::sql_types::Text>;

        fn into_json_index_expression(self) -> Self::Expression {
            self.into_sql::<Text>()
        }
    }

    impl JsonIndex for i32 {
        type Expression = crate::dsl::AsExprOf<i32, crate::sql_types::Int4>;

        fn into_json_index_expression(self) -> Self::Expression {
            self.into_sql::<crate::sql_types::Int4>()
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
