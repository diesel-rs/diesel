use crate::dsl;
use crate::query_builder::combination_clause::{
    All, CombinationClause, Distinct, Except, Intersect, Union,
};
use crate::query_builder::{AsQuery, Query};
use crate::Table;

/// Extension trait to combine queries using a combinator like `UNION`, `INTERSECT` or `EXCEPT`
/// with or without `ALL` rule for duplicates
pub trait CombineDsl {
    /// What kind of query does this type represent?
    type Query: Query;

    /// Combine two queries using a SQL `UNION`
    ///
    /// # Examples
    /// ```rust
    /// # extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// # use schema::{users, animals};
    /// # use crate::diesel::query_dsl::positional_order_dsl::PositionalOrderDsl;
    /// #
    /// # fn main() {
    /// #     use self::users::dsl::{users, name as user_name};
    /// #     use self::animals::dsl::{animals, name as animal_name};
    /// #     let connection = &mut establish_connection();
    /// let data = users.select(user_name.nullable())
    ///     .union(animals.select(animal_name).filter(animal_name.is_not_null()))
    /// #   .positional_order_by(1)
    ///     .load(connection);
    ///
    /// let expected_data = vec![
    ///     Some(String::from("Jack")),
    ///     Some(String::from("Sean")),
    ///     Some(String::from("Tess")),
    /// ];
    /// assert_eq!(Ok(expected_data), data);
    /// # }
    /// ```
    fn union<Rhs>(self, rhs: Rhs) -> dsl::Union<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>;

    /// Combine two queries using a SQL `UNION ALL`
    fn union_all<Rhs>(self, rhs: Rhs) -> dsl::UnionAll<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>;

    /// Combine two queries using a SQL `INTERSECT`
    fn intersect<Rhs>(self, rhs: Rhs) -> dsl::Intersect<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>;

    /// Combine two queries using a SQL `INTERSECT ALL`
    fn intersect_all<Rhs>(self, rhs: Rhs) -> dsl::IntersectAll<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>;

    /// Combine two queries using a SQL `EXCEPT`
    fn except<Rhs>(self, rhs: Rhs) -> dsl::Except<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>;

    /// Combine two queries using a SQL `EXCEPT ALL`
    fn except_all<Rhs>(self, rhs: Rhs) -> dsl::ExceptAll<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>;
}

impl<T: Table> CombineDsl for T {
    type Query = T::Query;

    fn union<Rhs>(self, rhs: Rhs) -> dsl::Union<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        CombinationClause::new(Union, Distinct, self.as_query(), rhs.as_query())
    }

    fn union_all<Rhs>(self, rhs: Rhs) -> dsl::UnionAll<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        CombinationClause::new(Union, All, self.as_query(), rhs.as_query())
    }

    fn intersect<Rhs>(self, rhs: Rhs) -> dsl::Intersect<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        CombinationClause::new(Intersect, Distinct, self.as_query(), rhs.as_query())
    }

    fn intersect_all<Rhs>(self, rhs: Rhs) -> dsl::IntersectAll<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        CombinationClause::new(Intersect, All, self.as_query(), rhs.as_query())
    }

    fn except<Rhs>(self, rhs: Rhs) -> dsl::Except<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        CombinationClause::new(Except, Distinct, self.as_query(), rhs.as_query())
    }

    fn except_all<Rhs>(self, rhs: Rhs) -> dsl::ExceptAll<Self, Rhs>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        CombinationClause::new(Except, All, self.as_query(), rhs.as_query())
    }
}
