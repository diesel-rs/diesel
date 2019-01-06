use crate::sql_types::{IntoNullable, SqlOrd};

sql_function! {
    /// Represents a SQL `MAX` function. This function can only take types which are
    /// ordered.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = establish_connection();
    /// assert_eq!(Ok(Some(8)), animals.select(max(legs)).first(&connection));
    /// # }
    #[aggregate]
    fn max<ST: SqlOrd + IntoNullable>(expr: ST) -> ST::Nullable;
}

sql_function! {
    /// Represents a SQL `MIN` function. This function can only take types which are
    /// ordered.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../../doctest_setup.rs");
    /// # use diesel::dsl::*;
    /// #
    /// # fn main() {
    /// #     use schema::animals::dsl::*;
    /// #     let connection = establish_connection();
    /// assert_eq!(Ok(Some(4)), animals.select(min(legs)).first(&connection));
    /// # }
    #[aggregate]
    fn min<ST: SqlOrd + IntoNullable>(expr: ST) -> ST::Nullable;
}
