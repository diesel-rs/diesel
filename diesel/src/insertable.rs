use crate::backend::{Backend, SupportsDefaultKeyword};
use crate::expression::grouped::Grouped;
use crate::expression::{AppearsOnTable, Expression};
use crate::query_builder::{
    AstPass, BatchInsert, InsertStatement, QueryFragment, QueryId, UndecoratedInsertRecord,
    ValuesClause,
};
use crate::query_source::{Column, Table};
use crate::result::QueryResult;

/// Represents that a structure can be used to insert a new row into the
/// database. This is automatically implemented for `&[T]` and `&Vec<T>` for
/// inserting more than one record.
///
/// This trait can be [derived](derive@Insertable)
pub trait Insertable<T> {
    /// The `VALUES` clause to insert these records
    ///
    /// The types used here are generally internal to Diesel.
    /// Implementations of this trait should use the `Values`
    /// type of other `Insertable` types.
    /// For example `<diesel::dsl::Eq<column, &str> as Insertable<table>>::Values`.
    type Values;

    /// Construct `Self::Values`
    ///
    /// Implementations of this trait typically call `.values`
    /// on other `Insertable` types.
    fn values(self) -> Self::Values;

    /// Insert `self` into a given table.
    ///
    /// `foo.insert_into(table)` is identical to `insert_into(table).values(foo)`.
    /// However, when inserting from a select statement,
    /// this form is generally preferred.
    ///
    /// # Example
    ///
    /// ```rust
    /// # include!("doctest_setup.rs");
    /// #
    /// # fn main() {
    /// #     run_test().unwrap();
    /// # }
    /// #
    /// # fn run_test() -> QueryResult<()> {
    /// #     use schema::{posts, users};
    /// #     let conn = &mut establish_connection();
    /// #     diesel::delete(posts::table).execute(conn)?;
    /// users::table
    ///     .select((
    ///         users::name.concat("'s First Post"),
    ///         users::id,
    ///     ))
    ///     .insert_into(posts::table)
    ///     .into_columns((posts::title, posts::user_id))
    ///     .execute(conn)?;
    ///
    /// let inserted_posts = posts::table
    ///     .select(posts::title)
    ///     .load::<String>(conn)?;
    /// let expected = vec!["Sean's First Post", "Tess's First Post"];
    /// assert_eq!(expected, inserted_posts);
    /// #     Ok(())
    /// # }
    /// ```
    fn insert_into(self, table: T) -> InsertStatement<T, Self::Values>
    where
        Self: Sized,
    {
        crate::insert_into(table).values(self)
    }
}

#[doc(inline)]
pub use diesel_derives::Insertable;

pub trait CanInsertInSingleQuery<DB: Backend> {
    /// How many rows will this query insert?
    ///
    /// This function should only return `None` when the query is valid on all
    /// backends, regardless of how many rows get inserted.
    fn rows_to_insert(&self) -> Option<usize>;
}

impl<'a, T, DB> CanInsertInSingleQuery<DB> for &'a T
where
    T: ?Sized + CanInsertInSingleQuery<DB>,
    DB: Backend,
{
    fn rows_to_insert(&self) -> Option<usize> {
        (*self).rows_to_insert()
    }
}

impl<T, U, DB> CanInsertInSingleQuery<DB> for ColumnInsertValue<T, U>
where
    DB: Backend,
{
    fn rows_to_insert(&self) -> Option<usize> {
        Some(1)
    }
}

impl<V, DB> CanInsertInSingleQuery<DB> for DefaultableColumnInsertValue<V>
where
    DB: Backend,
    V: CanInsertInSingleQuery<DB>,
{
    fn rows_to_insert(&self) -> Option<usize> {
        Some(1)
    }
}

pub trait InsertValues<T: Table, DB: Backend>: QueryFragment<DB> {
    ///判断是否为自增ID
    fn is_self_increase_id(&self)->bool{
        false       
    }

    fn column_names(&self, out: AstPass<DB>) -> QueryResult<()>;
    fn col_value(&self, col_name:String, mut out: AstPass<DB>)->QueryResult<()>{
        let _ = col_name;
        out.push_sql("");
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, QueryId)]
#[doc(hidden)]
pub struct ColumnInsertValue<Col, Expr> {
    col: Col,
    expr: Expr,
}

impl<Col, Expr> ColumnInsertValue<Col, Expr> {
    pub(crate) fn new(col: Col, expr: Expr) -> Self {
        Self { col, expr }
    }
}

#[derive(Debug, Copy, Clone)]
#[doc(hidden)]
pub enum DefaultableColumnInsertValue<T> {
    Expression(T),
    Default,
}

impl<T> QueryId for DefaultableColumnInsertValue<T> {
    type QueryId = ();
    const HAS_STATIC_QUERY_ID: bool = false;
}

impl<T> Default for DefaultableColumnInsertValue<T> {
    fn default() -> Self {
        DefaultableColumnInsertValue::Default
    }
}

impl<Col, Expr:crate::query_builder::QueryFragment<DB>, DB> InsertValues<Col::Table, DB> for ColumnInsertValue<Col, Expr>
// impl<Col, Expr, DB> InsertValues<Col::Table, DB>
//     for DefaultableColumnInsertValue<ColumnInsertValue<Col, Expr>>
where
    DB: Backend + SupportsDefaultKeyword,
    Col: Column,
    Expr: Expression<SqlType = Col::SqlType> + AppearsOnTable<()>,
    Self: QueryFragment<DB>,
{    

    ///判断是否为自增ID
    fn is_self_increase_id(&self)->bool{
        self.col.is_self_increase_id()
    }

    fn column_names(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_identifier(Col::NAME)?;
        Ok(())
    }

    fn col_value(&self, col_name:String, mut out: AstPass<DB>)->QueryResult<()>{

        self.expr.walk_ast_primary_key(col_name, out.reborrow())?;
        Ok(())
    }

}


impl<Col, Expr, DB> QueryFragment<DB> for ColumnInsertValue<Col, Expr>
where
    Col: Column,
    DB: Backend + SupportsDefaultKeyword,
    Expr: QueryFragment<DB>,
{
    ///判断是否为自增ID
    fn is_self_increase_id1(&self)->bool{
        self.col.is_self_increase_id()
    }

    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        self.expr.walk_ast(out.reborrow())?;
        Ok(())
    }

    ///walk_ast_primary_key
    fn walk_ast_primary_key(&self, primary_key:String, mut pass: AstPass<DB>) -> QueryResult<()>{

        if primary_key == Col::NAME{
            self.expr.walk_ast_primary_key(primary_key, pass.reborrow())?;
        }
        Ok(())
    }
}

#[cfg(not(feature = "sqlite"))]
impl<Col, Expr, DB> InsertValues<Col::Table, DB>
    for DefaultableColumnInsertValue<ColumnInsertValue<Col, Expr>>
where
    DB: Backend + SupportsDefaultKeyword,
    Col: Column,
    Expr: Expression<SqlType = Col::SqlType> + AppearsOnTable<()>,
    Expr: QueryFragment<DB>,    
    Self: QueryFragment<DB>,
{
    ///判断是否为自增ID
    fn is_self_increase_id(&self)->bool{
        if let Self::Expression(ref inner) = *self{            
            return inner.col.is_self_increase_id();
        }
        else{
            return false;
        }
    }

    fn column_names(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.push_identifier(Col::NAME)?;
        Ok(())
    }

    fn col_value(&self, col_name:String, mut out: AstPass<DB>)->QueryResult<()>{
        if let Self::Expression(ref inner) = *self{            
            inner.expr.walk_ast_primary_key(col_name, out.reborrow())?;
        }
        
        Ok(())
    }

}

#[cfg(not(feature = "sqlite"))]
impl<Col, Expr, DB> QueryFragment<DB> for DefaultableColumnInsertValue<ColumnInsertValue<Col, Expr>>
where
    DB: Backend + SupportsDefaultKeyword,
    Col: Column,
    Expr: Expression<SqlType = Col::SqlType> + AppearsOnTable<()>,
    ColumnInsertValue<Col, Expr>: QueryFragment<DB>,
    Expr: QueryFragment<DB>,
{
    ///判断是否为自增ID
    fn is_self_increase_id1(&self)->bool{
        if let Self::Expression(ref inner) = *self{            
            return inner.col.is_self_increase_id();
        }
        else{
            return false;
        }
    }

    fn walk_ast(&self, mut out: AstPass<DB>) -> QueryResult<()> {
        out.unsafe_to_cache_prepared();
        if let Self::Expression(ref inner) = *self {
            inner.walk_ast(out.reborrow())?;
        } else {
            out.push_sql("DEFAULT");
        }
        Ok(())
    }

    ///walk_ast_primary_key
    fn walk_ast_primary_key(&self, primary_key:String, mut pass: AstPass<DB>) -> QueryResult<()>{

        if primary_key == Col::NAME{
            if let Self::Expression(ref inner) = *self{            
                inner.expr.walk_ast_primary_key(primary_key, pass.reborrow())?; 
            }
        }
        
        Ok(())
    }
}


#[cfg(feature = "sqlite")]
impl<Col, Expr> InsertValues<Col::Table, crate::sqlite::Sqlite>
    for DefaultableColumnInsertValue<ColumnInsertValue<Col, Expr>>
where
    Col: Column,
    Expr: Expression<SqlType = Col::SqlType> + AppearsOnTable<()>,
    Self: QueryFragment<crate::sqlite::Sqlite>,
{
    fn column_names(&self, mut out: AstPass<crate::sqlite::Sqlite>) -> QueryResult<()> {
        if let Self::Expression(..) = *self {
            out.push_identifier(Col::NAME)?;
        }
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
impl<Col, Expr> QueryFragment<crate::sqlite::Sqlite>
    for DefaultableColumnInsertValue<ColumnInsertValue<Col, Expr>>
where
    Expr: QueryFragment<crate::sqlite::Sqlite>,
{
    fn walk_ast(&self, mut out: AstPass<crate::sqlite::Sqlite>) -> QueryResult<()> {
        if let Self::Expression(ref inner) = *self {
            inner.expr.walk_ast(out.reborrow())?;
        }
        Ok(())
    }
}

impl<'a, T, Tab> Insertable<Tab> for &'a [T]
where
    &'a T: UndecoratedInsertRecord<Tab>,
{
    type Values = BatchInsert<&'a [T], Tab, (), false>;

    fn values(self) -> Self::Values {
        BatchInsert::new(self)
    }
}

impl<'a, T, Tab> Insertable<Tab> for &'a Vec<T>
where
    &'a [T]: Insertable<Tab>,
{
    type Values = <&'a [T] as Insertable<Tab>>::Values;

    fn values(self) -> Self::Values {
        (&**self).values()
    }
}

impl<T, Tab> Insertable<Tab> for Vec<T>
where
    T: Insertable<Tab> + UndecoratedInsertRecord<Tab>,
{
    type Values = BatchInsert<Vec<T>, Tab, (), false>;

    fn values(self) -> Self::Values {
        BatchInsert::new(self)
    }
}

impl<T, Tab, const N: usize> Insertable<Tab> for [T; N]
where
    T: Insertable<Tab>,
{
    type Values = BatchInsert<[T; N], Tab, [T::Values; N], true>;

    fn values(self) -> Self::Values {
        BatchInsert::new(self)
    }
}

impl<'a, T, Tab, const N: usize> Insertable<Tab> for &'a [T; N]
where
    T: Insertable<Tab>,
{
    // We can reuse the query id for [T; N] here as this
    // compiles down to the same query
    type Values = BatchInsert<&'a [T; N], Tab, [T::Values; N], true>;

    fn values(self) -> Self::Values {
        BatchInsert::new(self)
    }
}

impl<T, Tab, const N: usize> Insertable<Tab> for Box<[T; N]>
where
    T: Insertable<Tab>,
{
    // We can reuse the query id for [T; N] here as this
    // compiles down to the same query
    type Values = BatchInsert<Box<[T; N]>, Tab, [T::Values; N], true>;

    fn values(self) -> Self::Values {
        // let val = self.0.values();
        // val
        BatchInsert::new(self)
    }
}

impl<T, V, Tab> Insertable<Tab> for Option<T>
where
    T: Insertable<Tab, Values = ValuesClause<V, Tab>>,
{
    type Values = ValuesClause<DefaultableColumnInsertValue<V>, Tab>;

    fn values(self) -> Self::Values {
        ValuesClause::new(
            self.map(|v| DefaultableColumnInsertValue::Expression(Insertable::values(v).values))
                .unwrap_or_default(),
        )
    }
}

impl<'a, T, Tab> Insertable<Tab> for &'a Option<T>
where
    Option<&'a T>: Insertable<Tab>,
{
    type Values = <Option<&'a T> as Insertable<Tab>>::Values;

    fn values(self) -> Self::Values {
        self.as_ref().values()
    }
}

impl<L, R, Tab> Insertable<Tab> for Grouped<crate::expression::operators::Eq<L, R>>
where
    crate::expression::operators::Eq<L, R>: Insertable<Tab>,
{
    type Values = <crate::expression::operators::Eq<L, R> as Insertable<Tab>>::Values;

    fn values(self) -> Self::Values {
        self.0.values()
    }
}

impl<'a, L, R, Tab> Insertable<Tab> for &'a Grouped<crate::expression::operators::Eq<L, R>>
where
    &'a crate::expression::operators::Eq<L, R>: Insertable<Tab>,
{
    type Values = <&'a crate::expression::operators::Eq<L, R> as Insertable<Tab>>::Values;

    fn values(self) -> Self::Values {
        self.0.values()
    }
}
