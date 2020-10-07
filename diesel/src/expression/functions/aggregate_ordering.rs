use crate::expression::functions::sql_function;
use crate::sql_types::{IntoNullable, Nullable, SingleValue, SqlOrd, SqlType};

pub trait SqlOrdAggregate: SingleValue {
    type Ret: SqlType + SingleValue;
}

impl<ST> SqlOrdAggregate for ST
where
    ST: SqlOrd + SingleValue + IntoNullable,
    ST::Nullable: SingleValue,
{
    type Ret = <Self as IntoNullable>::Nullable;
}

sql_function! {
    /// Represents a SQL `MAX` function. This function can only take types which are
    /// ordered.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = establish_connection();
    /// assert_eq!(Ok(Some(8)), animals.select(max(legs)).first(&connection));
    /// # }
    #[aggregate]
    fn max<ST: SqlOrdAggregate>(expr: Nullable<ST>) -> ST::Ret;
}

sql_function! {
    /// Represents a SQL `MIN` function. This function can only take types which are
    /// ordered.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = establish_connection();
    /// assert_eq!(Ok(Some(4)), animals.select(min(legs)).first(&connection));
    /// # }
    #[aggregate]
    fn min<ST: SqlOrdAggregate>(expr: Nullable<ST>) -> ST::Ret;
}
