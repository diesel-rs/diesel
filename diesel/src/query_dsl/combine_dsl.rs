use crate::query_builder::combination_clause::{
    All, Combination, Distinct, Except, Intersect, Union,
};
use crate::query_builder::{AsQuery, BoxedSelectStatement, Query, SelectStatement};
use crate::Table;

/// Extension trait to combine queries using a combinator like `UNION`, `INTERSECT` or `EXPECT`
/// with or without `ALL` rule for duplicates
pub trait CombineDsl {
    /// What kind of query does this type represent?
    type Query: Query;

    /// Combine two queries using a SQL `UNION`
    ///
    /// # Examples
    /// ```rust
    /// # #[macro_use] extern crate diesel;
    /// # include!("../doctest_setup.rs");
    /// # use schema::{users, animals};
    ///
    /// # fn main() {
    /// #     use self::users::dsl::{users, name as user_name};
    /// #     use self::animals::dsl::{animals, name as animal_name};
    /// #     let connection = establish_connection();
    /// let data = users.select(user_name.nullable())
    ///     .union(animals.select(animal_name))
    ///     .load(&connection);
    ///
    /// let expected_data = vec![
    ///     None,
    ///     Some(String::from("Jack")),
    ///     Some(String::from("Tess")),
    ///     Some(String::from("Sean")),
    /// ];
    /// assert_eq!(Ok(expected_data), data);
    /// # }
    /// ```
    fn union<Rhs>(self, rhs: Rhs) -> Combination<Union, Distinct, Self::Query, Rhs::Query>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>;

    /// Combine two queries using a SQL `UNION ALL`
    fn union_all<Rhs>(self, rhs: Rhs) -> Combination<Union, All, Self::Query, Rhs::Query>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>;

    /// Combine two queries using a SQL `INTERSECT`
    fn intersect<Rhs>(self, rhs: Rhs) -> Combination<Intersect, Distinct, Self::Query, Rhs::Query>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>;

    /// Combine two queries using a SQL `INTERSECT ALL`
    fn intersect_all<Rhs>(self, rhs: Rhs) -> Combination<Intersect, All, Self::Query, Rhs::Query>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>;

    /// Combine two queries using a SQL `EXCEPT`
    fn except<Rhs>(self, rhs: Rhs) -> Combination<Except, Distinct, Self::Query, Rhs::Query>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>;

    /// Combine two queries using a SQL `EXCEPT ALL`
    fn except_all<Rhs>(self, rhs: Rhs) -> Combination<Except, All, Self::Query, Rhs::Query>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>;
}

impl<T: Table> CombineDsl for T {
    type Query = T::Query;

    fn union<Rhs>(self, rhs: Rhs) -> Combination<Union, Distinct, Self::Query, Rhs::Query>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        Combination::new(Union, Distinct, self.as_query(), rhs.as_query())
    }

    fn union_all<Rhs>(self, rhs: Rhs) -> Combination<Union, All, Self::Query, Rhs::Query>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        Combination::new(Union, All, self.as_query(), rhs.as_query())
    }

    fn intersect<Rhs>(self, rhs: Rhs) -> Combination<Intersect, Distinct, Self::Query, Rhs::Query>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        Combination::new(Intersect, Distinct, self.as_query(), rhs.as_query())
    }

    fn intersect_all<Rhs>(self, rhs: Rhs) -> Combination<Intersect, All, Self::Query, Rhs::Query>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        Combination::new(Intersect, All, self.as_query(), rhs.as_query())
    }

    fn except<Rhs>(self, rhs: Rhs) -> Combination<Except, Distinct, Self::Query, Rhs::Query>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        Combination::new(Except, Distinct, self.as_query(), rhs.as_query())
    }

    fn except_all<Rhs>(self, rhs: Rhs) -> Combination<Except, All, Self::Query, Rhs::Query>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        Combination::new(Except, All, self.as_query(), rhs.as_query())
    }
}

impl<F, S, D, W, O, LOf, G, LC> CombineDsl for SelectStatement<F, S, D, W, O, LOf, G, LC>
where
    Self: Query,
{
    type Query = Self;

    fn union<Rhs>(
        self,
        rhs: Rhs,
    ) -> Combination<Union, Distinct, Self::Query, <Rhs as AsQuery>::Query>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        Combination::new(Union, Distinct, self, rhs.as_query())
    }

    fn union_all<Rhs>(
        self,
        rhs: Rhs,
    ) -> Combination<Union, All, Self::Query, <Rhs as AsQuery>::Query>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        Combination::new(Union, All, self, rhs.as_query())
    }

    fn intersect<Rhs>(
        self,
        rhs: Rhs,
    ) -> Combination<Intersect, Distinct, Self::Query, <Rhs as AsQuery>::Query>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        Combination::new(Intersect, Distinct, self, rhs.as_query())
    }

    fn intersect_all<Rhs>(
        self,
        rhs: Rhs,
    ) -> Combination<Intersect, All, Self::Query, <Rhs as AsQuery>::Query>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        Combination::new(Intersect, All, self, rhs.as_query())
    }

    fn except<Rhs>(
        self,
        rhs: Rhs,
    ) -> Combination<Except, Distinct, Self::Query, <Rhs as AsQuery>::Query>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        Combination::new(Except, Distinct, self, rhs.as_query())
    }

    fn except_all<Rhs>(
        self,
        rhs: Rhs,
    ) -> Combination<Except, All, Self::Query, <Rhs as AsQuery>::Query>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        Combination::new(Except, All, self, rhs.as_query())
    }
}

impl<'a, ST, QS, DB> CombineDsl for BoxedSelectStatement<'a, ST, QS, DB>
where
    Self: Query,
{
    type Query = Self;

    fn union<Rhs>(
        self,
        rhs: Rhs,
    ) -> Combination<Union, Distinct, Self::Query, <Rhs as AsQuery>::Query>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        Combination::new(Union, Distinct, self, rhs.as_query())
    }

    fn union_all<Rhs>(
        self,
        rhs: Rhs,
    ) -> Combination<Union, All, Self::Query, <Rhs as AsQuery>::Query>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        Combination::new(Union, All, self, rhs.as_query())
    }

    fn intersect<Rhs>(
        self,
        rhs: Rhs,
    ) -> Combination<Intersect, Distinct, Self::Query, <Rhs as AsQuery>::Query>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        Combination::new(Intersect, Distinct, self, rhs.as_query())
    }

    fn intersect_all<Rhs>(
        self,
        rhs: Rhs,
    ) -> Combination<Intersect, All, Self::Query, <Rhs as AsQuery>::Query>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        Combination::new(Intersect, All, self, rhs.as_query())
    }

    fn except<Rhs>(
        self,
        rhs: Rhs,
    ) -> Combination<Except, Distinct, Self::Query, <Rhs as AsQuery>::Query>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        Combination::new(Except, Distinct, self, rhs.as_query())
    }

    fn except_all<Rhs>(
        self,
        rhs: Rhs,
    ) -> Combination<Except, All, Self::Query, <Rhs as AsQuery>::Query>
    where
        Rhs: AsQuery<SqlType = <Self::Query as Query>::SqlType>,
    {
        Combination::new(Except, All, self, rhs.as_query())
    }
}
